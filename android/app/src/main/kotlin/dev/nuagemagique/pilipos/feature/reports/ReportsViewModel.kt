package dev.nuagemagique.pilipos.feature.reports

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.DEFAULT_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ERROR_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.MoneyDirection
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.PosEvent

/**
 * State holder of the reports screen, a port of the script of `ui/src/views/ReportsView.vue`.
 *
 * The engine owns every rule (what a report contains, who may copy or refund, the fiscal signing): this only
 * calls it through the gateway and keeps what the screen shows. Every engine failure is caught and shown as a
 * toast (and kept in the state where the screen has a panel for it); with the engine not ready the lists are
 * empty and no action crashes.
 *
 * No amount is computed here (PD208): a cash amount is sent as entered.
 */
@Suppress("TooManyFunctions") // One entry per action of the screen, as the script of ReportsView.vue.
internal class ReportsViewModel(
    private val entry: ReportsEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(ReportsState())
    private var zDisarm: Job? = null

    /** What the screen draws. */
    val state: StateFlow<ReportsState> =
        combine(own, entry.session().currentUser) { base, user -> base.copy(signedIn = user != null) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(signedIn = entry.session().currentUser.value != null))

    init {
        // Subscribed right here (no replay): a Resync reloads the day tab.
        viewModelScope.launch(start = CoroutineStart.UNDISPATCHED) {
            entry.events().events.collect { event -> if (event is PosEvent.Resync) load() }
        }
    }

    // ── Tabs and loading ──────────────────────────────────────────

    /** Loads the day tab: the Z history and the reminder that a Z is due (`onMounted`). */
    fun load() {
        viewModelScope.launch { loadDay() }
    }

    /** Chooses a tab; leaving the day tab disarms the Z. */
    fun selectTab(tab: ReportsTab) {
        disarmZ()
        own.update { it.copy(tab = tab) }
    }

    private suspend fun loadDay() {
        loadHistory()
        loadZPending()
    }

    private suspend fun loadHistory() {
        // Silently degrade: the history is not critical.
        attempt { gateway { engine -> engine.listZReports() } }.onSuccess { list ->
            own.update { it.copy(day = it.day.copy(zHistory = list)) }
        }
    }

    private suspend fun loadZPending() {
        // The notice is only a reminder: a failure means no notice.
        val pending = attempt { gateway { engine -> engine.needsZReport() } }.getOrDefault(false)
        own.update { it.copy(day = it.day.copy(zPending = pending)) }
    }

    // ── Day: X, user X, Z ─────────────────────────────────────────

    /** The X button. */
    fun generateX() {
        generate(ReportKind.X)
    }

    /** The user X button. */
    fun generateUserX() {
        generate(ReportKind.USER_X)
    }

    /**
     * The Z button. A Z closes the day for good: the first tap only arms the button, the second one within
     * [DayState.Z_ARM_WINDOW_MS] generates it.
     */
    fun generateZ() {
        if (operatorInsz() == null || own.value.day.busy) return
        if (!own.value.day.zArmed) {
            own.update { it.copy(day = it.day.copy(zArmed = true)) }
            zDisarm?.cancel()
            zDisarm =
                viewModelScope.launch {
                    delay(DayState.Z_ARM_WINDOW_MS)
                    own.update { it.copy(day = it.day.copy(zArmed = false)) }
                }
            return
        }
        generate(ReportKind.Z)
    }

    private fun disarmZ() {
        zDisarm?.cancel()
        zDisarm = null
        own.update { it.copy(day = it.day.copy(zArmed = false)) }
    }

    private fun generate(kind: ReportKind) {
        val insz = operatorInsz() ?: return
        if (own.value.day.busy) return
        if (kind == ReportKind.Z) disarmZ()
        own.update { it.copy(day = it.day.copy(loadingKind = kind)) }
        viewModelScope.launch {
            val outcome = runReport(kind, insz)
            own.update { it.copy(day = it.day.copy(loadingKind = null, lastReport = outcome)) }
            if (kind == ReportKind.Z && outcome.generated) loadDay()
        }
    }

    private suspend fun runReport(
        kind: ReportKind,
        insz: String,
    ): ReportOutcome =
        attempt {
            gateway { engine ->
                when (kind) {
                    ReportKind.X -> engine.generateXReport(insz)
                    ReportKind.USER_X -> engine.generateUserXReport(insz)
                    ReportKind.Z -> engine.generateZReport(insz)
                }
            }
        }.fold(
            onSuccess = { result ->
                if (result.success) {
                    toast(ToastVariant.Success, "${text("report_generated")} (${label(kind, result.reportNo)})", REPORT_TOAST_MS)
                    ReportOutcome(kind, result)
                } else {
                    ReportOutcome(kind, result, failure = errorRaw(result.error, ERROR_TOAST_TTL_MS))
                }
            },
            onFailure = { ReportOutcome(kind, null, failure = errorOf(it, ERROR_TOAST_TTL_MS)) },
        )

    private fun label(
        kind: ReportKind,
        reportNo: Long?,
    ): String =
        when (kind) {
            ReportKind.X -> "X"
            ReportKind.USER_X -> "X util."
            ReportKind.Z -> if (reportNo != null && reportNo != 0L) "Z #$reportNo" else "Z"
        }

    // ── Z history ─────────────────────────────────────────────────

    /** Opens the Z history dialog and refreshes it. */
    fun openZList() {
        own.update { it.copy(day = it.day.copy(zListOpen = true)) }
        viewModelScope.launch { loadHistory() }
    }

    /** Closes the Z history dialog. */
    fun closeZList() {
        own.update { it.copy(day = it.day.copy(zListOpen = false)) }
    }

    // ── Cash ──────────────────────────────────────────────────────

    /** Chooses money in or out; a reason that does not fit the direction becomes the first one that does. */
    fun setCashDirection(direction: CashDirection) {
        own.update { current ->
            val cash = current.cash
            val reason = if (cash.reason in CashRules.reasonsOf(direction)) cash.reason else CashRules.reasonsOf(direction).first()
            current.copy(cash = cash.copy(direction = direction, reason = reason))
        }
    }

    /** Chooses the reason of the movement. */
    fun setCashReason(reason: String) {
        own.update { it.copy(cash = it.cash.copy(reason = reason)) }
    }

    /** Sets the amount as entered (a preset or the keypad), in cents; never computed. */
    fun setCashAmount(amountCents: Long) {
        own.update { it.copy(cash = it.cash.copy(amountCents = amountCents)) }
    }

    /** The record button: puts money in or out of the drawer with the amount, direction and reason of the form. */
    fun submitCash() {
        val insz = operatorInsz() ?: return
        val cash = own.value.cash
        if (!cash.canSubmit) return
        val direction = if (cash.direction == CashDirection.IN) MoneyDirection.IN else MoneyDirection.OUT
        viewModelScope.launch {
            val ok = moveCash(insz, cash.amountCents, direction, cash.reason, "cash_success")
            if (ok) own.update { it.copy(cash = it.cash.copy(amountCents = 0L)) }
        }
    }

    /** The drawer count (Basic Set M133): any amount, 0 for an empty drawer. */
    fun declareDrawer(amountCents: Long) {
        val insz = operatorInsz() ?: return
        if (amountCents < 0L) return
        viewModelScope.launch {
            moveCash(insz, amountCents, MoneyDirection.DECLARATION, CashRules.DECLARATION_REASON, "cash_declared")
        }
    }

    /** Records a fiscal drawer opening outside of a sale. */
    fun openDrawer() {
        val insz = operatorInsz() ?: return
        viewModelScope.launch {
            attempt { gateway { engine -> engine.openDrawerFiscal(insz) } }.fold(
                onSuccess = { result -> cashAnswer(result.success, result.error, "cash_drawer_opened") },
                onFailure = { cashError(it) },
            )
        }
    }

    private suspend fun moveCash(
        insz: String,
        amountCents: Long,
        direction: MoneyDirection,
        reason: String,
        successKey: String,
    ): Boolean {
        own.update { it.copy(cash = it.cash.copy(loading = true)) }
        val answer =
            attempt { gateway { engine -> engine.moneyInOut(insz, amountCents, direction, reason) } }.fold(
                onSuccess = { result -> cashAnswer(result.success, result.error, successKey) },
                onFailure = { cashError(it) },
            )
        own.update { it.copy(cash = it.cash.copy(loading = false)) }
        return answer
    }

    /** Toast and state for the answer of a cash command; returns whether it was recorded. */
    private fun cashAnswer(
        success: Boolean,
        error: String?,
        successKey: String,
    ): Boolean {
        if (success) {
            toast(ToastVariant.Success, text(successKey))
            recordCash(CashOutcome(true))
            return true
        }
        val failure =
            if (error.isNullOrEmpty()) {
                text("cash_failed").also { toast(ToastVariant.Error, it, ERROR_TOAST_TTL_MS) }
            } else {
                errorRaw(error, ERROR_TOAST_TTL_MS)
            }
        recordCash(CashOutcome(false, failure))
        return false
    }

    /** Toast and state for a cash command that threw; always false. */
    private fun cashError(error: Throwable): Boolean {
        recordCash(CashOutcome(false, errorOf(error, DEFAULT_TOAST_TTL_MS)))
        return false
    }

    private fun recordCash(outcome: CashOutcome) {
        own.update { it.copy(cash = it.cash.copy(lastOutcome = outcome)) }
    }

    // ── Helpers ───────────────────────────────────────────────────

    /** The screen text of a refusal, in the language of the till. */
    fun describeError(error: Throwable): String = entry.errors().text(error, language())

    private fun operatorInsz(): String? = entry.session().currentUser.value?.insz

    private fun language(): AppLanguage = entry.settings().language.value

    private suspend fun <T> gateway(block: suspend (PosEngineInterface) -> T): T = entry.gateway().call(block)

    private fun text(key: String): String = entry.catalog().text(key, language())

    private fun toast(
        variant: ToastVariant,
        message: String,
        ttlMs: Long = DEFAULT_TOAST_TTL_MS,
    ) {
        entry.toasts().show(message, variant, ttlMs)
    }

    /** Shows the refusal [error] as an error toast and returns its text. */
    private fun errorOf(
        error: Throwable,
        ttlMs: Long,
    ): String {
        val message = entry.errors().text(error, language())
        entry.toasts().show(message, ToastVariant.Error, ttlMs)
        return message
    }

    /** Shows the raw error string of a result as an error toast and returns its text. */
    private fun errorRaw(
        raw: String?,
        ttlMs: Long,
    ): String {
        val message = entry.errors().textRaw(raw, language())
        entry.toasts().show(message, ToastVariant.Error, ttlMs)
        return message
    }

    private companion object {
        /** Lifetime of the toast of a generated report, in milliseconds (4000 in `ReportsView.vue`). */
        const val REPORT_TOAST_MS: Long = 4000L
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
