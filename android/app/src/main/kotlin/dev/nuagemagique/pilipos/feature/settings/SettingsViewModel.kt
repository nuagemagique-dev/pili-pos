package dev.nuagemagique.pilipos.feature.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.nav.SettingsEntry
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.CreateUserInput
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.User

/** How many ticket deliveries the list asks for (`listTicketDeliveries(50)` of `SettingsView.vue`). */
private const val DELIVERIES_LIMIT: Long = 50L

/** Lifetime of the toast that reports a refused training switch, in milliseconds (6000 in `SettingsView.vue`). */
private const val TRAINING_TOAST_MS: Long = 6000L

/** An INSZ has 11 digits. */
internal const val INSZ_LENGTH: Int = 11

/** What the session, the settings store and the integrations store say, gathered before the screen state is built. */
private data class Shared(
    val user: User?,
    val manager: Boolean,
    val engine: PosSettings?,
    val training: Boolean,
    val language: AppLanguage,
)

/** The product label and the mail status, gathered before the screen state is built. */
private data class Pieces(
    val label: ProductLabel,
    val mail: MailStatus?,
)

/**
 * State holder of the settings screen, a port of the script of `ui/src/views/SettingsView.vue` and of
 * `IntegrationSwitch.vue` (the HubRise panel has its own [HubriseViewModel]).
 *
 * The engine owns every rule (who may create a user, the PIN checks, the refusal to switch HubRise off while
 * online orders are unsettled): this calls it through the shared stores and the gateway and keeps what the screen
 * shows. A refusal is turned into text with the [ErrorPresenter][dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter]
 * in the language of the till. The remote support section of the Vue view is not ported (no remote support in v1).
 *
 * PIN dialogs (`PinDialog`): in [SettingsUiState.pin] the screen shows, by [SettingsPinFlow.stage],
 * `PinMode.Choose` calling [submitChosenPin] or `PinMode.Verify` calling [confirmWithManagerPin]; both throw the
 * engine refusal for the dialog to display. [onPinDone] is the `onDone` of an own-PIN dialog and of the manager
 * confirmation; the choose dialog of another employee or of a new manager passes a no-op, because
 * [submitChosenPin] has already moved on to the confirmation.
 */
@Suppress("TooManyFunctions") // One entry per action of the screen, as the script of SettingsView.vue.
internal class SettingsViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val settings = entry.settings()
    private val session = entry.session()
    private val integrations = entry.integrations()
    private val delivery = entry.delivery()
    private val own = MutableStateFlow(SettingsUiState())

    /** What the screen draws. */
    val state: StateFlow<SettingsUiState> =
        combine(
            combine(
                session.currentUser,
                session.isManager,
                settings.settings,
                settings.trainingMode,
                settings.language,
                ::Shared,
            ),
            combine(settings.productLabel, delivery.mailStatus, ::Pieces),
            integrations.state,
            own,
        ) { shared, pieces, switch, base ->
            build(base, shared, pieces, switch)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            build(own.value, currentShared(), Pieces(settings.productLabel.value, delivery.mailStatus.value), integrations.state.value),
        )

    init {
        // onMounted of the Vue view: load everything once the engine answers.
        viewModelScope.launch {
            entry.gateway().ready.collect { ready -> if (ready) refresh() }
        }
    }

    // ── Loading ───────────────────────────────────────────────────

    /** Loads the employees, the mail status, the deliveries and the HubRise switch again. */
    suspend fun refresh() {
        settings.load()
        integrations.load()
        loadUsers()
        refreshMailStatus()
        loadDeliveries()
        loadTablePlan()
    }

    /** Reads the table plan; a failure keeps the panel as it was. */
    suspend fun loadTablePlan() {
        attempt { entry.gateway().call { engine -> engine.getTablePlan() } }
            .onSuccess { plan -> own.update { it.copy(tablePlan = TablePlanState(plan.plan, plan.tableCount, plan.plan)) } }
    }

    /** Lists the employees; a failure keeps the previous list (`loadWorkers`). */
    suspend fun loadUsers() {
        attempt { entry.gateway().call { engine -> engine.listUsers() } }
            .onSuccess { list -> own.update { it.copy(users = list) } }
    }

    /** Asks the engine whether the mail service is configured and reachable (through the delivery store). A failure gives no status. */
    suspend fun refreshMailStatus() {
        delivery.refreshMailStatus()
    }

    /** Lists the last ticket deliveries; a failure gives an empty list (`loadDeliveries`). */
    suspend fun loadDeliveries() {
        own.update { it.copy(deliveriesLoading = true) }
        val list =
            attempt { entry.gateway().call { engine -> engine.listTicketDeliveries(DELIVERIES_LIMIT) } }
                .getOrDefault(emptyList())
        own.update { it.copy(deliveries = list, deliveriesLoading = false) }
    }

    /** The refresh button of the deliveries list. */
    fun reloadDeliveries() {
        viewModelScope.launch { loadDeliveries() }
    }

    // ── Rows (UI-PD308) ───────────────────────────────────────────

    /** The settings rows [isManager] may see, in the order of `NavRegistry.settingsEntries`. */
    fun rows(isManager: Boolean): List<SettingsEntry> = visibleRows(entry.registry().settingsEntries, isManager)

    // ── Training, language, product label ────────────────────────

    /** The training switch; the engine refuses it while an order is open, and the refusal is shown as a toast. */
    fun toggleTraining() {
        viewModelScope.launch {
            attempt { settings.setTrainingMode(!settings.trainingMode.value) }
                .onFailure { entry.errors().show(it, language(), ttlMs = TRAINING_TOAST_MS) }
        }
    }

    /** Chooses the language of the texts (the order screen reloads its own menu, UI-PD303). */
    fun setLanguage(language: AppLanguage) {
        settings.setLanguage(language)
    }

    /** Chooses what the product tiles show. */
    fun setProductLabel(label: ProductLabel) {
        settings.setProductLabel(label)
    }

    // ── New employee ──────────────────────────────────────────────

    /** The name field. */
    fun setNewName(name: String) {
        own.update { it.copy(form = it.form.copy(name = name)) }
    }

    /** The INSZ field: digits only, at most [INSZ_LENGTH] (`inputmode="numeric" maxlength="11"`). */
    fun setNewInsz(insz: String) {
        own.update { it.copy(form = it.form.copy(insz = insz.filter { c -> c in '0'..'9' }.take(INSZ_LENGTH))) }
    }

    /** The role buttons. */
    fun setNewRole(role: NewRole) {
        own.update { it.copy(form = it.form.copy(role = role)) }
    }

    /**
     * The add button (`addWorker`). A missing name or INSZ, or an INSZ that is not 11 digits, is refused here. A
     * manager needs a first PIN confirmed by the creating manager: the PIN flow opens; any other role is created
     * at once.
     */
    fun addEmployee() {
        val form = own.value.form
        val name = form.name.trim()
        val problem =
            when {
                name.isEmpty() || form.insz.isEmpty() -> text("settings_err_name_insz_required")
                form.insz.length != INSZ_LENGTH -> text("settings_err_11_digits_required")
                else -> null
            }
        if (problem != null) {
            own.update { it.copy(form = it.form.copy(error = problem)) }
            return
        }
        own.update { it.copy(form = it.form.copy(error = null)) }
        if (form.role == NewRole.Admin) {
            own.update { it.copy(pin = SettingsPinFlow(PinPurpose.NewManager, form.insz, name)) }
            return
        }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.createUser(CreateUserInput(form.insz, name, form.role.code, null, null)) } }
                .fold(
                    onSuccess = {
                        own.update { it.copy(form = it.form.copy(name = "", insz = "")) }
                        loadUsers()
                    },
                    onFailure = { failure -> own.update { it.copy(form = it.form.copy(error = describe(failure))) } },
                )
        }
    }

    // ── Deactivation (two taps) ───────────────────────────────────

    /**
     * The deactivate button of the employee [insz]: the first tap arms it, the second one deactivates. The
     * operator cannot deactivate himself (the screen shows no button).
     */
    fun deactivate(insz: String) {
        if (isSelf(insz)) return
        if (own.value.confirmDeactivate != insz) {
            own.update { it.copy(confirmDeactivate = insz) }
            return
        }
        own.update { it.copy(confirmDeactivate = null) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.deactivateUser(insz) } }.fold(
                onSuccess = { loadUsers() },
                onFailure = { failure -> own.update { it.copy(form = it.form.copy(error = describe(failure))) } },
            )
        }
    }

    /** Drops the armed state of the deactivate button. */
    fun cancelDeactivate() {
        own.update { it.copy(confirmDeactivate = null) }
    }

    // ── PIN flow ──────────────────────────────────────────────────

    /** The PIN button of the employee [user] (`openPin`): the own PIN or another employee's. */
    fun openPin(user: User) {
        val purpose = if (isSelf(user.insz)) PinPurpose.Own else PinPurpose.Other
        own.update { it.copy(pin = SettingsPinFlow(purpose, user.insz, user.name)) }
    }

    /** Closes the PIN dialogs without saving (`closePin`). */
    fun closePin() {
        own.update { it.copy(pin = null) }
    }

    /**
     * The PIN typed twice in the choose dialog. Your own PIN is saved at once (`set_pin`, no operator PIN) and
     * confirmed by a toast; another employee's or a new manager's is kept and the manager's own PIN is asked next.
     * A refusal is thrown for the dialog.
     */
    suspend fun submitChosenPin(pin: String) {
        val flow = own.value.pin ?: return
        if (flow.purpose == PinPurpose.Own) {
            entry.gateway().call { engine -> engine.setPin(flow.insz, pin, null) }
            entry.toasts().show(text("pin_changed"), ToastVariant.Success)
        } else {
            own.update { current -> current.copy(pin = current.pin?.copy(stage = PinStage.ConfirmWithManager, chosen = pin)) }
        }
    }

    /**
     * The manager's own PIN, typed once to confirm (`confirmWithManagerPin`): sets the PIN of another employee, or
     * creates the new manager with the chosen first PIN. A refusal (wrong PIN, locked) is thrown for the dialog.
     */
    suspend fun confirmWithManagerPin(operatorPin: String) {
        val flow = own.value.pin ?: return
        val chosen = flow.chosen.orEmpty()
        if (flow.purpose == PinPurpose.NewManager) {
            val input = CreateUserInput(flow.insz, flow.name, NewRole.Admin.code, chosen, operatorPin)
            entry.gateway().call { engine -> engine.createUser(input) }
            own.update { it.copy(form = it.form.copy(name = "", insz = "")) }
        } else {
            entry.gateway().call { engine -> engine.setPin(flow.insz, chosen, operatorPin) }
        }
    }

    /** A PIN dialog finished: closes the flow and lists the employees again (`closePin(); loadWorkers()`). */
    fun onPinDone() {
        own.update { it.copy(pin = null) }
        viewModelScope.launch { loadUsers() }
    }

    // ── Table plan ────────────────────────────────────────────────

    /** The table plan field. */
    fun setTablePlanInput(text: String) {
        own.update { it.copy(tablePlan = it.tablePlan.copy(input = text, error = null)) }
    }

    /** Saves the field as the table plan (managers only, the engine checks it); a refusal shows under the field. */
    fun saveTablePlan() {
        val input = own.value.tablePlan.input
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.setTablePlan(input) } }
                .onSuccess { plan ->
                    own.update { it.copy(tablePlan = TablePlanState(plan.plan, plan.tableCount, plan.plan)) }
                    entry.toasts().show(text("settings_tables_saved"), ToastVariant.Success)
                }.onFailure { error -> own.update { it.copy(tablePlan = it.tablePlan.copy(error = describe(error))) } }
        }
    }

    // ── HubRise switch ────────────────────────────────────────────

    /**
     * The HubRise button (`IntegrationSwitch.vue`). A tap while one is running is ignored. The engine refuses to
     * switch off while online orders are unsettled; the refusal is shown as a toast.
     */
    fun toggleHubrise() {
        if (own.value.hubrise.busy) return
        own.update { it.copy(hubrise = it.hubrise.copy(busy = true)) }
        viewModelScope.launch {
            attempt { integrations.setHubriseEnabled(!integrations.hubriseEnabled.value) }
                .onFailure { entry.errors().show(it, language()) }
            own.update { it.copy(hubrise = it.hubrise.copy(busy = false)) }
        }
    }

    // ── Texts ─────────────────────────────────────────────────────

    /** The screen text of a refusal, in the language of the till. */
    fun describe(error: Throwable): String = entry.errors().text(error, language())

    private fun isSelf(insz: String): Boolean = session.currentUser.value?.insz == insz

    private fun language(): AppLanguage = settings.language.value

    private fun text(key: String): String = entry.catalog().text(key, language())

    private fun currentShared(): Shared =
        Shared(session.currentUser.value, session.isManager.value, settings.settings.value, settings.trainingMode.value, language())

    private fun build(
        base: SettingsUiState,
        shared: Shared,
        pieces: Pieces,
        switch: Integrations?,
    ): SettingsUiState =
        base.copy(
            rows = visibleRows(entry.registry().settingsEntries, shared.manager),
            isManager = shared.manager,
            currentInsz = shared.user?.insz,
            training = shared.training,
            language = shared.language,
            productLabel = pieces.label,
            mailStatus = pieces.mail,
            info = shared.engine,
            hubrise =
                base.hubrise.copy(
                    enabled = switch?.hubriseEnabled ?: false,
                    visible = (switch?.hubriseEnabled ?: false) || (switch?.onlineOrdersOpen ?: 0L) > 0L,
                    onlineOrdersOpen = switch?.onlineOrdersOpen ?: 0L,
                    log = switch?.hubriseLog.orEmpty(),
                ),
        )
}

/** Runs [block]; the answer or the failure it threw (cancellation is rethrown). */
@Suppress("TooGenericExceptionCaught")
internal suspend fun <T> attempt(block: suspend () -> T): Result<T> =
    try {
        Result.success(block())
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Result.failure(e)
    }
