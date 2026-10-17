package dev.nuagemagique.pilipos.feature.settings

import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.badge.BadgeCapture
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.pos_ffi.BadgeSettingsInput
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User

/** How long the enrolment dialog waits for the badge, in milliseconds (`CAPTURE_MS` of `BadgeEnrolDialog.vue`). */
internal const val BADGE_CAPTURE_MS: Long = 30_000L

/** Lifetime of the toast of a failed link or removal, in milliseconds (5000 in `SettingsView.vue`). */
private const val BADGE_ERROR_TOAST_MS: Long = 5000L

/** The longest number the timing pad takes (3 digits in `BadgeSettings.vue`). */
private const val TIMING_DIGITS = 3

/**
 * The badge half of the settings screen (`BadgeSettings.vue`, `BadgeEnrolDialog.vue` and the badge parts of
 * `SettingsView.vue`): the reader panel, the enrolment of a badge and its removal. It shares the screen state
 * [own] with [SettingsViewModel]; the engine owns every rule (who may link, the PIN check, the uniqueness).
 */
@Suppress("TooManyFunctions")
internal class SettingsBadgeModel(
    private val entry: SettingsFeatureEntryPoint,
    private val own: MutableStateFlow<SettingsUiState>,
    private val scope: CoroutineScope,
    private val reloadUsers: suspend () -> Unit,
) {
    private val badges = entry.badges()
    private var enrolJob: Job? = null
    private var capture: BadgeCapture? = null
    private var enrolFailed = false

    // ── The reader panel ──────────────────────────────────────────

    /** The head of the panel: folds and unfolds it. */
    fun toggle() {
        own.update { it.copy(badge = it.badge.copy(open = !it.badge.open)) }
    }

    /** The reader choice buttons ("Désactivé", "NFC intégré", "Lecteur USB"). */
    fun setReader(reader: String) {
        apply { current(reader = reader) }
    }

    /** The "Connexion par badge" switch. */
    fun toggleLogin() {
        apply { current(loginEnabled = !(settings()?.loginEnabled ?: false)) }
    }

    /** A timing field was tapped: the pad edits it. */
    fun editField(field: TimingField) {
        own.update { it.copy(badge = it.badge.copy(editing = field)) }
    }

    /** A key of the timing pad: digits up to [TIMING_DIGITS], clear and backspace. */
    fun onKey(key: NumPadKey) {
        own.update { state ->
            val panel = state.badge
            val field = panel.editing
            val draft = typedOrStored()
            if (field == null || draft == null) return@update state
            val typed = draft.of(field)
            val next =
                when (key) {
                    NumPadKey.Clear -> ""
                    NumPadKey.Backspace -> typed.dropLast(1)
                    is NumPadKey.Digit -> if (typed.length < TIMING_DIGITS) typed + key.digit else typed
                }
            state.copy(badge = panel.copy(typed = draft.with(field, next)))
        }
    }

    /** The save button of the timing: the three numbers go to the engine, which checks the ranges. */
    fun saveTiming() {
        val draft = typedOrStored() ?: return
        own.update { it.copy(badge = it.badge.copy(editing = null)) }
        apply {
            current(
                gap = draft.gap.toLongOrNull() ?: 0L,
                minLen = draft.minLen.toLongOrNull() ?: 0L,
                maxLen = draft.maxLen.toLongOrNull() ?: 0L,
            )
        }
    }

    private fun settings() = badges.settings.value

    /** What the timing fields hold: the typed numbers, else the stored ones. */
    private fun typedOrStored(): BadgeTimingDraft? = own.value.badge.typed ?: settings()?.let { BadgeTimingDraft.of(it) }

    private fun current(
        reader: String? = null,
        loginEnabled: Boolean? = null,
        gap: Long? = null,
        minLen: Long? = null,
        maxLen: Long? = null,
    ): BadgeSettingsInput {
        val s = checkNotNull(settings()) { "badge settings not loaded" }
        return BadgeSettingsInput(
            reader ?: s.reader,
            loginEnabled ?: s.loginEnabled,
            gap ?: s.wedgeMaxGapMs,
            minLen ?: s.wedgeMinLen,
            maxLen ?: s.wedgeMaxLen,
        )
    }

    /** Sends a change; a refusal shows under the panel and the fields go back to the stored values (`apply`). */
    private fun apply(input: () -> BadgeSettingsInput) {
        if (settings() == null) return
        own.update { it.copy(badge = it.badge.copy(error = null)) }
        scope.launch {
            val failure = attempt { badges.save(input()) }.exceptionOrNull()
            own.update { it.copy(badge = it.badge.copy(typed = null, error = failure?.let(::describe))) }
        }
    }

    // ── Enrolment (L5-D5): the badge, then the manager's own PIN ──

    /** The badge button of an employee row: "Retirer le badge" when they have one, else "Lier un badge". */
    fun onEmployee(user: User) {
        val linked = badges.overview.value?.badges?.any { it.insz == user.insz } == true
        if (linked) askRevoke(user) else startEnrol(user)
    }

    /** The "Lier un badge" button of [user]: the dialog waits for the badge (30 s). */
    fun startEnrol(user: User) {
        cancelJob()
        enrolFailed = false
        own.update { it.copy(enrol = BadgeEnrolFlow(user.insz, user.name)) }
        val waiting = badges.startCapture()
        capture = waiting
        enrolJob =
            scope.launch {
                val token = withTimeoutOrNull(BADGE_CAPTURE_MS) { waiting.token.await() }
                if (token == null) {
                    entry.toasts().show(text("badge_capture_timeout"), ToastVariant.Warning)
                    cancelEnrol()
                } else {
                    own.update { it.copy(enrol = it.enrol?.copy(token = token)) }
                }
            }
    }

    /** Annuler, or the end of the dialog: nothing is linked. */
    fun cancelEnrol() {
        cancelJob()
        own.update { it.copy(enrol = null) }
    }

    /**
     * The manager's own PIN confirms the link. A refusal about the PIN is thrown for the dialog to show and retry; a
     * refusal about the badge (in use, the employee already has one) ends the dialog with an error toast.
     */
    @Suppress("TooGenericExceptionCaught", "ThrowsCount")
    suspend fun confirmEnrolPin(pin: String) {
        val flow = own.value.enrol ?: return
        val token = flow.token ?: return
        try {
            badges.link(flow.insz, token, pin)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if ((e as? PosException.Coded)?.code?.startsWith("pin_") == true) throw e
            enrolFailed = true
            entry.errors().show(e, language(), ttlMs = BADGE_ERROR_TOAST_MS)
            cancelEnrol()
            throw e
        }
    }

    /** The PIN dialog succeeded: the badge is linked. */
    fun onEnrolDone() {
        if (enrolFailed) return
        val flow = own.value.enrol
        entry.toasts().show(text("badge_linked_done", "name" to (flow?.name ?: "")), ToastVariant.Success)
        cancelEnrol()
        scope.launch { reloadUsers() }
    }

    private fun cancelJob() {
        enrolJob?.cancel()
        enrolJob = null
        capture?.cancel()
        capture = null
    }

    /** The view model is going away: the next scan must be an ordinary scan again. */
    fun clear() = cancelJob()

    // ── Removal ───────────────────────────────────────────────────

    /** The "Retirer le badge" button of [user]: asks to confirm. */
    fun askRevoke(user: User) {
        own.update { it.copy(revokeBadge = BadgeRevokeAsk(user.insz, user.name)) }
    }

    /** The confirmation dialog was dismissed. */
    fun cancelRevoke() {
        own.update { it.copy(revokeBadge = null) }
    }

    /** The confirmation dialog was accepted: the engine removes the badge. */
    fun confirmRevoke() {
        val ask = own.value.revokeBadge ?: return
        own.update { it.copy(revokeBadge = null) }
        scope.launch {
            attempt { badges.revoke(ask.insz) }.fold(
                onSuccess = { entry.toasts().show(text("badge_revoked_done", "name" to ask.name), ToastVariant.Success) },
                onFailure = { entry.errors().show(it, language(), ttlMs = BADGE_ERROR_TOAST_MS) },
            )
        }
    }

    // ── Texts ─────────────────────────────────────────────────────

    /** The screen text of a refusal, in the language of the till. */
    fun describe(error: Throwable): String = entry.errors().text(error, language())

    private fun language(): AppLanguage = entry.settings().language.value

    private fun text(
        key: String,
        vararg vars: Pair<String, Any>,
    ): String = entry.catalog().text(key, language(), vars.toMap())
}
