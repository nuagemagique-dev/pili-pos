package dev.nuagemagique.pilipos.feature.login

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadState
import dev.nuagemagique.pilipos.ui.feedback.DEFAULT_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole

/** Lifetime of the toast that warns about an operator without a PIN, in milliseconds (5000 in `LoginView.vue`). */
private const val NO_PIN_TOAST_MS: Long = 5000L

/** Lifetime of the toast that reports a failed clock-out, in milliseconds (6000 in `auth.ts`). */
private const val PRESENCE_TOAST_MS: Long = 6000L

/** The engine code of a manager who has no PIN yet. */
private const val PIN_SETUP_REQUIRED = "pin_setup_required"

/** The engine code of an INSZ typed by hand whose operator has a PIN. */
private const val PIN_REQUIRED = "pin_required"

/**
 * State holder of the login screen, a port of the script of `ui/src/views/LoginView.vue`.
 *
 * The engine owns every rule (clock-in, PIN checks, end of shift): this only calls it through the shared stores
 * and the gateway and keeps what the screen shows. Every engine failure is caught: with the engine not ready
 * the screen shows the INSZ entry and the checks as not configured.
 *
 * It never navigates by itself. The only effect, [LoginEffect.GoToTables], is sent after an action of the
 * operator succeeded (PD204, PD205, PD211).
 */
@Suppress("TooManyFunctions") // One entry per action of the screen, as the script of LoginView.vue.
internal class LoginViewModel(
    private val entry: LoginEntryPoint,
    fdm: FdmProbe = StoreFdmProbe(entry.fdm()),
) : ViewModel() {
    private val settings = entry.settings()
    private val checks = LoginStartupChecks(entry.gateway(), fdm)
    private val own = MutableStateFlow(LoginUiState())
    private val effectChannel = Channel<LoginEffect>(Channel.BUFFERED)

    /** What the screen draws. */
    val state: StateFlow<LoginUiState> =
        combine(own, settings.settings, settings.language, settings.trainingMode) { base, engine, language, training ->
            withSettings(base, engine, language, training)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            withSettings(own.value, settings.settings.value, settings.language.value, settings.trainingMode.value),
        )

    /** One-shot effects, each delivered once. */
    val effects: Flow<LoginEffect> = effectChannel.receiveAsFlow()

    init {
        viewModelScope.launch { runChecks() }
        viewModelScope.launch { loadShifts() }
        viewModelScope.launch { loadUsers() }
    }

    // ── Start-up loading ──────────────────────────────────────────

    private suspend fun runChecks() {
        // The HubRise switch loads when the engine turns ready: make sure it has before reading it.
        if (entry.integrations().state.value == null) entry.integrations().load()
        val list = checks.run(entry.integrations().hubriseEnabled.value)
        own.update { it.copy(checks = list) }
    }

    private suspend fun loadShifts() {
        val shifts = attempt { entry.gateway().call { engine -> engine.listOpenShifts() } }.getOrDefault(emptyList())
        own.update { it.copy(shifts = shifts) }
    }

    private suspend fun loadUsers() {
        attempt { entry.gateway().call { engine -> engine.listUsers() } }.fold(
            onSuccess = { list ->
                val active = list.filter { it.active }
                own.update { it.copy(users = active, numpadMode = active.isEmpty()) }
            },
            onFailure = { own.update { it.copy(numpadMode = true) } },
        )
    }

    // ── Tiles and the PIN panel ───────────────────────────────────

    /** A tap on the tile of [user] (`quickLogin`). */
    fun onTile(user: User) {
        own.update { it.copy(error = null) }
        when {
            user.hasPin -> openPin(user)
            user.role == UserRole.ADMIN -> askFirstPin(user.insz, user.name)
            own.value.shiftStart(user.insz) != null -> openPin(user)
            else -> loginWithoutPin(user)
        }
    }

    private fun openPin(user: User) {
        own.update { it.copy(pin = PinPanel(user)) }
    }

    /** A key of the PIN pad; the 4th digit logs in, or ends the shift in end-shift mode. */
    fun onPinKey(key: NumPadKey) {
        val panel = own.value.pin ?: return
        val pin = panel.pad.apply(key) ?: return
        viewModelScope.launch { submitPin(panel, pin) }
    }

    private suspend fun submitPin(
        panel: PinPanel,
        pin: String,
    ) {
        if (panel.endShift) {
            finishShift(panel, pin)
            return
        }
        attempt { entry.session().login(panel.user.insz, pin) }.fold(
            onSuccess = {
                panel.pad.succeeded()
                own.update { it.copy(pin = null) }
                effectChannel.send(LoginEffect.GoToTables)
            },
            onFailure = { showPanelFailure(panel, describeError(it)) },
        )
    }

    /** Clocks the operator out (S event) and stays on the login screen, then reloads the open shifts. */
    private suspend fun finishShift(
        panel: PinPanel,
        pin: String?,
    ) {
        val user = panel.user
        attempt { entry.session().endShift(user.insz, pin) }.fold(
            onSuccess = { result ->
                if (result.success) {
                    toast(ToastVariant.Success, text("shift_ended", "name" to user.name))
                } else {
                    val reason = entry.errors().textRaw(result.error ?: "FDM error", language(), text("presence_failed"))
                    entry.toasts().show(reason, ToastVariant.Error, PRESENCE_TOAST_MS)
                }
                own.update { it.copy(pin = null) }
                loadShifts()
            },
            onFailure = { showPanelFailure(panel, describeError(it)) },
        )
    }

    /** The pad of an operator with a PIN shows the refusal and clears itself; the panel of one without shows it alone. */
    private fun showPanelFailure(
        panel: PinPanel,
        message: String,
    ) {
        if (panel.user.hasPin) {
            panel.pad.failed(message)
        } else {
            own.update { current -> current.copy(pin = current.pin?.copy(error = message)) }
        }
    }

    /** The back button of the PIN panel (`cancelPin`). */
    fun cancelPin() {
        own.update { it.copy(pin = null) }
    }

    /** The end-shift button of the PIN panel: the PIN now ends the shift instead of logging in, and back. */
    fun toggleEndShift() {
        own.update { current ->
            current.copy(pin = current.pin?.copy(endShift = !current.pin.endShift, pad = PinPadState(PinMode.Verify), error = null))
        }
    }

    /** The login button of the panel of an operator on shift without a PIN. */
    fun loginFromPanel() {
        val panel = own.value.pin ?: return
        loginWithoutPin(panel.user)
    }

    /** The end-shift button of the panel of an operator on shift without a PIN: no PIN to ask. */
    fun endShiftFromPanel() {
        val panel = own.value.pin ?: return
        viewModelScope.launch { finishShift(panel, null) }
    }

    private fun loginWithoutPin(user: User) {
        viewModelScope.launch {
            attempt { entry.session().login(user.insz, null) }.fold(
                onSuccess = {
                    toast(ToastVariant.Warning, text("pin_not_set_warning"), NO_PIN_TOAST_MS)
                    effectChannel.send(LoginEffect.GoToTables)
                },
                onFailure = { failure ->
                    if (codeOf(failure) == PIN_SETUP_REQUIRED) {
                        askFirstPin(user.insz, user.name)
                    } else {
                        own.update { it.copy(error = describeError(failure)) }
                    }
                },
            )
        }
    }

    // ── INSZ entry ────────────────────────────────────────────────

    /** The "other person" / back button: swaps the tiles and the INSZ entry (`toggleInsz`). */
    fun toggleInsz() {
        own.update { it.copy(numpadMode = !it.numpadMode, insz = "", pin = null) }
    }

    /** A key of the INSZ numpad. */
    fun onInszKey(key: NumPadKey) {
        own.update { current ->
            val typed = current.insz
            val next =
                when (key) {
                    NumPadKey.Clear -> ""
                    NumPadKey.Backspace -> typed.dropLast(1)
                    is NumPadKey.Digit -> if (typed.length < LoginUiState.INSZ_LENGTH) typed + key.digit else typed
                }
            current.copy(insz = next)
        }
    }

    /** The login button of the INSZ entry; the operator who needs a PIN gets the PIN panel. */
    fun submitInsz() {
        val insz = own.value.insz
        if (insz.length != LoginUiState.INSZ_LENGTH) {
            own.update { it.copy(error = text("settings_err_11_digits_required")) }
            return
        }
        own.update { it.copy(error = null) }
        viewModelScope.launch {
            attempt { entry.session().login(insz, null) }.fold(
                onSuccess = { effectChannel.send(LoginEffect.GoToTables) },
                onFailure = { onInszRefused(insz, it) },
            )
        }
    }

    private fun onInszRefused(
        insz: String,
        failure: Throwable,
    ) {
        when (codeOf(failure)) {
            PIN_SETUP_REQUIRED -> askFirstPin(insz, own.value.nameOf(insz))
            PIN_REQUIRED -> {
                val known = own.value.users.firstOrNull { it.insz == insz }
                openPin(known ?: User(insz = insz, name = insz, role = UserRole.WAITER, active = true, hasPin = true))
            }
            else -> own.update { it.copy(error = describeError(failure)) }
        }
    }

    // ── First PIN ─────────────────────────────────────────────────

    private fun askFirstPin(
        insz: String,
        name: String,
    ) {
        own.update { it.copy(firstPin = FirstPinTarget(insz, name)) }
    }

    /** Stores the PIN just chosen and opens the session (`login_with_new_pin`); a refusal propagates to the dialog. */
    suspend fun chooseFirstPin(pin: String) {
        val target = own.value.firstPin ?: return
        entry.session().loginWithNewPin(target.insz, pin)
        toast(ToastVariant.Success, text("pin_chosen"))
    }

    /** The first-PIN dialog succeeded: the session is open. */
    fun onFirstPinDone() {
        own.update { it.copy(firstPin = null) }
        effectChannel.trySend(LoginEffect.GoToTables)
    }

    /** The first-PIN dialog was dismissed. */
    fun cancelFirstPin() {
        own.update { it.copy(firstPin = null) }
    }

    // ── Language and texts ────────────────────────────────────────

    /** Chooses the language of the texts. */
    fun setLanguage(language: AppLanguage) {
        settings.setLanguage(language)
    }

    /** The screen text of a refusal, in the language of the till. */
    fun describeError(error: Throwable): String = entry.errors().text(error, language())

    private fun language(): AppLanguage = settings.language.value

    private fun text(
        key: String,
        vararg vars: Pair<String, Any>,
    ): String = entry.catalog().text(key, language(), vars.toMap())

    private fun toast(
        variant: ToastVariant,
        message: String,
        ttlMs: Long = DEFAULT_TOAST_TTL_MS,
    ) {
        entry.toasts().show(message, variant, ttlMs)
    }

    private fun codeOf(error: Throwable): String? = (error as? PosException.Coded)?.code

    private companion object {
        fun withSettings(
            base: LoginUiState,
            engine: PosSettings?,
            language: AppLanguage,
            training: Boolean,
        ): LoginUiState =
            base.copy(
                footer = listOfNotNull(engine?.restaurantName, engine?.terminalId).filter { it.isNotEmpty() }.joinToString(" · "),
                version = engine?.posSwVersion.orEmpty(),
                language = language,
                training = training,
            )
    }
}

/** Runs [block]; the answer or the failure it threw (cancellation is rethrown). */
@Suppress("TooGenericExceptionCaught")
private suspend fun <T> attempt(block: suspend () -> T): Result<T> =
    try {
        Result.success(block())
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Result.failure(e)
    }
