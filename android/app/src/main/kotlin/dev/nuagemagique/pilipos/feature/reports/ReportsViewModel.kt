package dev.nuagemagique.pilipos.feature.reports

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.DEFAULT_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ERROR_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.OrderRefusedException
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
import uniffi.pos_ffi.CopyableTicket
import uniffi.pos_ffi.CorrectionDone
import uniffi.pos_ffi.MoneyDirection
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.RefundItem
import uniffi.pos_ffi.RefundableEvent
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.SalePayments
import uniffi.pos_ffi.TicketDeliveryRequest

/** How many recent sales the refund list asks for (`listRefundableEvents(20)` of `ReportsView.vue`). */
private const val REFUNDABLE_LIMIT: Long = 20L

/** The label of the event of a training ticket (`T` in `copyKind`). */
private const val LABEL_TRAINING = "T"

/** The label of the event of a pre-bill (`P` in `copyKind`). */
private const val LABEL_PRE_BILL = "P"

/** The smallest quantity of a ticked refund line. */
private const val MIN_REFUND_QUANTITY: Int = 1

/**
 * State holder of the reports screen, a port of the script of `ui/src/views/ReportsView.vue`.
 *
 * The engine owns every rule (what a report contains, who may copy or refund, the fiscal signing): this only
 * calls it through the gateway and keeps what the screen shows. Every engine failure is caught and shown as a
 * toast (and kept in the state where the screen has a panel for it); with the engine not ready the lists are
 * empty and no action crashes.
 *
 * No amount is computed here (PD208, UI-PD307): a cash amount is sent as entered and the refund exposes the
 * ticked lines with their quantity for the screen to price. The delivery request of a copy or a refund is a
 * parameter, so this does not depend on the delivery store.
 */
@Suppress("TooManyFunctions") // One entry per action of the screen, as the script of ReportsView.vue.
internal class ReportsViewModel(
    private val entry: ReportsEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(ReportsState())
    private var zDisarm: Job? = null

    /** What the screen draws. */
    val state: StateFlow<ReportsState> =
        combine(own, entry.session().currentUser, entry.session().isManager) { base, user, manager ->
            withOperator(base, user?.name, manager)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            withOperator(own.value, entry.session().currentUser.value?.name, entry.session().isManager.value),
        )

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

    /** Chooses a tab; leaving the day tab disarms the Z, opening the tickets tab loads the lists. */
    fun selectTab(tab: ReportsTab) {
        disarmZ()
        own.update { it.copy(tab = tab) }
        if (tab == ReportsTab.TICKETS) loadTickets()
    }

    private suspend fun loadDay() {
        loadHistory()
        loadZPending()
        loadUnsettled()
    }

    private suspend fun loadHistory() {
        // Silently degrade: the history is not critical.
        attempt { entry.gateway().call { engine -> engine.listZReports() } }.onSuccess { list ->
            own.update { it.copy(day = it.day.copy(zHistory = list)) }
        }
    }

    private suspend fun loadZPending() {
        // The notice is only a reminder: a failure means no notice.
        val pending = attempt { entry.gateway().call { engine -> engine.needsZReport() } }.getOrDefault(false)
        own.update { it.copy(day = it.day.copy(zPending = pending)) }
    }

    private suspend fun loadUnsettled() {
        // Only a warning: a failure means none, and the Z does not depend on it.
        val list = attempt { entry.gateway().call { engine -> engine.listUnsettledCardPayments() } }.getOrDefault(emptyList())
        own.update { it.copy(day = it.day.copy(unsettled = list)) }
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
            viewModelScope.launch { loadUnsettled() }
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
            entry.gateway().call { engine ->
                when (kind) {
                    ReportKind.X -> engine.generateXReport(insz)
                    ReportKind.USER_X -> engine.generateUserXReport(insz)
                    ReportKind.Z -> engine.generateZReport(insz)
                }
            }
        }.fold(
            onSuccess = { result ->
                if (result.success) {
                    toast(ToastVariant.Success, "${text("report_generated")} (${reportLabel(kind, result.reportNo)})", REPORT_TOAST_MS)
                    ReportOutcome(kind, result)
                } else {
                    ReportOutcome(kind, result, failure = errorRaw(result.error, ERROR_TOAST_TTL_MS))
                }
            },
            onFailure = { ReportOutcome(kind, null, failure = errorOf(it, ERROR_TOAST_TTL_MS)) },
        )

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

    /** The reprint button of a Z in the history: closes the list and opens the copy flow of that Z. */
    fun reprintZ(fiscalEventId: Long?) {
        closeZList()
        openCopy(fiscalEventId, CopyTargetKind.Z)
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

    /** Opens the "other amount" keypad of the cash form. */
    fun openOtherAmount() {
        own.update { it.copy(cash = it.cash.copy(otherAmountOpen = true)) }
    }

    /** Closes the "other amount" keypad without taking its amount. */
    fun closeOtherAmount() {
        own.update { it.copy(cash = it.cash.copy(otherAmountOpen = false)) }
    }

    /** The keypad confirmed: closes it and sets the amount as typed (`handleOtherAmount`); never computed. */
    fun pickOtherAmount(amountCents: Long) {
        own.update { it.copy(cash = it.cash.copy(otherAmountOpen = false, amountCents = amountCents)) }
    }

    /** Opens the drawer count keypad. */
    fun openDeclaration() {
        own.update { it.copy(cash = it.cash.copy(declarationOpen = true)) }
    }

    /** Closes the drawer count keypad without recording a count. */
    fun closeDeclaration() {
        own.update { it.copy(cash = it.cash.copy(declarationOpen = false)) }
    }

    /** The drawer count (Basic Set M133): any amount, 0 for an empty drawer. */
    fun declareDrawer(amountCents: Long) {
        own.update { it.copy(cash = it.cash.copy(declarationOpen = false)) }
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
            attempt { entry.gateway().call { engine -> engine.openDrawerFiscal(insz) } }.fold(
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
            attempt { entry.gateway().call { engine -> engine.moneyInOut(insz, amountCents, direction, reason) } }.fold(
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

    // ── Tickets tab ───────────────────────────────────────────────

    /** Loads the copyable tickets and the refundable sales (the tickets tab opens). */
    fun loadTickets() {
        viewModelScope.launch { fetchTickets() }
    }

    private suspend fun fetchTickets() {
        own.update { it.copy(tickets = it.tickets.copy(loading = true)) }
        val copyable =
            attempt { entry.gateway().call { engine -> engine.listCopyableTickets(null) } }.getOrElse { failure ->
                errorOf(failure, ERROR_TOAST_TTL_MS)
                emptyList()
            }
        val refundable =
            attempt { entry.gateway().call { engine -> engine.listRefundableEvents(REFUNDABLE_LIMIT) } }
                .getOrDefault(emptyList())
        val rows = mergeRows(copyable, refundable)
        own.update {
            it.copy(
                tickets =
                    TicketsState(
                        loaded = true,
                        loading = false,
                        copyable = copyable,
                        refundable = refundable,
                        rows = rows,
                        corrections = it.tickets.corrections,
                    ),
            )
        }
        loadCorrections(rows)
    }

    /** Best effort: which sales already have a payment correction, to mark them (a manager only). */
    private suspend fun loadCorrections(rows: List<TicketRow>) {
        if (!entry.session().isManager.value) return
        val found = LinkedHashMap<Long, CorrectionDone>()
        for (row in rows.filter { it.isSale }) {
            val payments = attempt { entry.gateway().call { engine -> engine.listSalePayments(row.fiscalEventId) } }.getOrNull()
            payments?.corrected?.let { found[row.fiscalEventId] = it }
        }
        own.update { it.copy(tickets = it.tickets.copy(corrections = found)) }
    }

    // ── Payment correction flow (event F) ─────────────────────────

    /** Opens the correction dialog on a sale and reads its payment lines; a failed read is shown in the dialog. */
    fun openCorrection(row: TicketRow) {
        own.update { it.copy(correctionFlow = CorrectionFlow(row.fiscalEventId, row.ticketNumber)) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.listSalePayments(row.fiscalEventId) } }.fold(
                onSuccess = { payments -> showPayments(row.fiscalEventId, payments) },
                onFailure = { failure ->
                    val message = entry.errors().text(failure, language())
                    updateCorrection(row.fiscalEventId) { it.copy(loadFailure = message) }
                },
            )
        }
    }

    private fun showPayments(
        fiscalEventId: Long,
        payments: SalePayments,
    ) {
        updateCorrection(fiscalEventId) { flow ->
            // The only line left is picked on its own.
            val free =
                payments.lines.filter {
                    it.amountType == CorrectionFlow.AMOUNT_PAYMENT && it.inputMethod != CorrectionFlow.INPUT_AUTOMATIC
                }
            val only = free.singleOrNull()?.index?.takeIf { payments.correctable }
            flow.copy(payments = payments, lineIndex = only)
        }
    }

    private fun updateCorrection(
        fiscalEventId: Long,
        change: (CorrectionFlow) -> CorrectionFlow,
    ) {
        own.update { current ->
            val flow = current.correctionFlow
            if (flow == null || flow.fiscalEventId != fiscalEventId) current else current.copy(correctionFlow = change(flow))
        }
    }

    /** Picks the payment line to correct; a card the linked terminal took cannot be picked. */
    fun pickCorrectionLine(index: Int) {
        own.update { current ->
            val flow = current.correctionFlow ?: return@update current
            val line = flow.lines.firstOrNull { it.index == index } ?: return@update current
            if (line.inputMethod == CorrectionFlow.INPUT_AUTOMATIC) return@update current
            // The method the line already has is not a choice.
            val method = flow.method?.takeIf { it != methodOf(line.paymentType) }
            current.copy(correctionFlow = flow.copy(lineIndex = index, method = method))
        }
    }

    /** Picks the method the payment really was made with. */
    fun pickCorrectionMethod(method: PaymentMethod) {
        own.update { current ->
            val flow = current.correctionFlow ?: return@update current
            val line = flow.picked ?: return@update current
            val allowed = method in CORRECTION_METHODS && method != methodOf(line.paymentType)
            if (allowed) current.copy(correctionFlow = flow.copy(method = method)) else current
        }
    }

    /** The confirm button: asks for the manager PIN. */
    fun askCorrectionPin() {
        own.update { current ->
            val flow = current.correctionFlow
            if (flow == null || !flow.canConfirm) current else current.copy(correctionFlow = flow.copy(askPin = true))
        }
    }

    /** Back from the PIN dialog to the choices. */
    fun cancelCorrectionPin() {
        own.update { current -> current.copy(correctionFlow = current.correctionFlow?.copy(askPin = false)) }
    }

    /**
     * The PIN typed in the dialog: the engine corrects the payment for the logged-in manager, or refuses and the
     * error is thrown to the dialog, which shows it on the pad.
     */
    suspend fun correctPayment(pin: String) {
        val insz = operatorInsz() ?: error("No user")
        val flow = own.value.correctionFlow ?: error("No correction")
        val line = flow.picked ?: error("No line")
        val method = flow.method ?: error("No method")
        val result = entry.gateway().call { engine -> engine.correctPayment(flow.fiscalEventId, line.index, method, insz, pin) }
        if (!result.success) throw OrderRefusedException(result.error.orEmpty())
    }

    /** The correction was signed: the dialog closes, the toast says so and the lists are read again. */
    fun correctionDone() {
        own.update { it.copy(correctionFlow = null) }
        toast(ToastVariant.Success, text("reports_pc_done"))
        loadTickets()
    }

    /** Closes the correction dialog without a correction. */
    fun closeCorrection() {
        own.update { it.copy(correctionFlow = null) }
    }

    // ── Copy flow (event C) ───────────────────────────────────────

    /** Opens the copy flow on the picker, before anything is signed; nothing happens without a fiscal event. */
    fun openCopy(
        fiscalEventId: Long?,
        kind: CopyTargetKind,
    ) {
        if (fiscalEventId == null) return
        own.update { it.copy(copyFlow = CopyFlow(fiscalEventId, kind)) }
    }

    /** The confirm button of the picker: signs the copy of the event the flow was opened on. */
    fun confirmCopy(request: TicketDeliveryRequest) {
        val flow = own.value.copyFlow ?: return
        copy(flow.fiscalEventId, request)
    }

    /**
     * Signs a copy (event C) of [fiscalEventId] for [request]; paper is the engine default, so a paper request is
     * sent as none, as the Vue screen does. A failed call closes the flow.
     */
    fun copy(
        fiscalEventId: Long,
        request: TicketDeliveryRequest,
    ) {
        val insz = operatorInsz() ?: return
        val flow = own.value.copyFlow
        if (flow != null && (flow.step != CopyStep.PICKER || flow.fiscalEventId != fiscalEventId)) return
        own.update { it.copy(copyFlow = (flow ?: CopyFlow(fiscalEventId, CopyTargetKind.TICKET)).copy(step = CopyStep.PROCESSING)) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.copyTicket(fiscalEventId, insz, deliveryOf(request)) } }.fold(
                onSuccess = { result ->
                    if (result.success) {
                        toast(ToastVariant.Success, text("report_reprint_ok"))
                    } else {
                        errorRaw(result.error, ERROR_TOAST_TTL_MS)
                    }
                    own.update { it.copy(copyFlow = it.copyFlow?.copy(step = CopyStep.RESULT, result = result)) }
                },
                onFailure = {
                    errorOf(it, ERROR_TOAST_TTL_MS)
                    own.update { current -> current.copy(copyFlow = null) }
                },
            )
        }
    }

    /** Closes the copy flow; a copy of a ticket that was signed refreshes the lists. */
    fun closeCopy() {
        val flow = own.value.copyFlow
        own.update { it.copy(copyFlow = null) }
        if (flow != null && flow.step == CopyStep.RESULT && flow.kind == CopyTargetKind.TICKET) loadTickets()
    }

    // ── Refund flow ───────────────────────────────────────────────

    /** Opens the refund flow on the list of refundable sales. */
    fun startRefund() {
        own.update { it.copy(refundFlow = RefundFlow(RefundStep.CHOOSE_EVENT)) }
        if (!own.value.tickets.loaded) loadTickets()
    }

    /** Chooses the sale to refund (the refund button of a row): loads its lines; a failed read gives no line. */
    fun openRefund(event: RefundableEvent) {
        own.update { it.copy(refundFlow = RefundFlow(RefundStep.CHOOSE_LINES, event = event)) }
        viewModelScope.launch {
            val lines = attempt { entry.gateway().call { engine -> engine.listOrderLines(event.orderId) } }.getOrDefault(emptyList())
            own.update { current ->
                val flow = current.refundFlow
                if (flow == null || flow.event != event) current else current.copy(refundFlow = flow.copy(lines = lines))
            }
        }
    }

    /** Ticks a line with its full quantity, or unticks it. */
    fun toggleRefundLine(lineId: Long) {
        updateLines { flow ->
            val line = flow.lines.firstOrNull { it.id == lineId } ?: return@updateLines flow
            val checked = LinkedHashMap(flow.checked)
            if (checked.containsKey(lineId)) checked.remove(lineId) else checked[lineId] = line.quantity
            flow.copy(checked = checked)
        }
    }

    /** Steps the quantity of a line by [delta] (-1 or 1), kept between 1 and the quantity of the line. */
    fun adjustRefundQuantity(
        lineId: Long,
        delta: Int,
    ) {
        updateLines { flow ->
            val line = flow.lines.firstOrNull { it.id == lineId } ?: return@updateLines flow
            val current = flow.checked[lineId] ?: 0
            val next = (current + delta).coerceIn(MIN_REFUND_QUANTITY, maxOf(MIN_REFUND_QUANTITY, line.quantity))
            val checked = LinkedHashMap(flow.checked)
            checked[lineId] = next
            flow.copy(checked = checked)
        }
    }

    private fun updateLines(change: (RefundFlow) -> RefundFlow) {
        own.update { current ->
            val flow = current.refundFlow
            if (flow == null || flow.step != RefundStep.CHOOSE_LINES) current else current.copy(refundFlow = change(flow))
        }
    }

    /** The next button of the lines step: opens the delivery step; nothing happens with no line ticked. */
    fun goToRefundDelivery() {
        updateLines { if (it.hasSelection) it.copy(step = RefundStep.DELIVERY) else it }
    }

    /** The back button of the delivery step. */
    fun backToRefundLines() {
        own.update { current ->
            val flow = current.refundFlow
            if (flow == null || flow.step != RefundStep.DELIVERY) {
                current
            } else {
                current.copy(refundFlow = flow.copy(step = RefundStep.CHOOSE_LINES))
            }
        }
    }

    /**
     * Signs the refund of the ticked lines for [request] (paper is sent as none, as the Vue screen does). A failed
     * call goes back to the lines step.
     */
    fun refund(request: TicketDeliveryRequest) {
        val insz = operatorInsz()
        val flow = own.value.refundFlow?.takeIf { it.step == RefundStep.DELIVERY && it.hasSelection }
        val event = flow?.event
        if (insz == null || flow == null || event == null) return
        val items = flow.selected.map { RefundItem(menuItemId = it.line.menuItemId, quantity = it.quantity, lineId = it.line.id) }
        val delivery = deliveryOf(request)
        own.update { it.copy(refundFlow = flow.copy(step = RefundStep.PROCESSING)) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.refundCheckout(event.fiscalEventId, items, insz, delivery) } }.fold(
                onSuccess = { result ->
                    if (result.success) {
                        toast(ToastVariant.Success, text("refund_success"))
                    } else {
                        errorRaw(result.error, DEFAULT_TOAST_TTL_MS)
                    }
                    own.update { it.copy(refundFlow = it.refundFlow?.copy(step = RefundStep.RESULT, result = result)) }
                },
                onFailure = {
                    errorOf(it, DEFAULT_TOAST_TTL_MS)
                    own.update { current -> current.copy(refundFlow = current.refundFlow?.copy(step = RefundStep.CHOOSE_LINES)) }
                },
            )
        }
    }

    /** Closes the refund flow; a refund that was signed refreshes the lists. */
    fun closeRefund() {
        val flow = own.value.refundFlow
        own.update { it.copy(refundFlow = null) }
        if (flow != null && flow.step == RefundStep.RESULT) loadTickets()
    }

    // ── Helpers ───────────────────────────────────────────────────

    /** The screen text of a refusal, in the language of the till. */
    fun describeError(error: Throwable): String = entry.errors().text(error, language())

    /** The screen text of the raw error string of a result that was not a success, in the language of the till. */
    fun describeRaw(raw: String?): String = entry.errors().textRaw(raw, language())

    private fun operatorInsz(): String? = entry.session().currentUser.value?.insz

    private fun language(): AppLanguage = entry.settings().language.value

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

        /** Paper is the engine default: the Vue screen sends no request for it. */
        fun deliveryOf(request: TicketDeliveryRequest): TicketDeliveryRequest? =
            if (request.medium == RequestedMedium.PAPER) null else request
    }
}

/** [base] with what the session says: the operator's name (null when nobody is in) and whether they are a manager. */
private fun withOperator(
    base: ReportsState,
    userName: String?,
    manager: Boolean,
): ReportsState = base.copy(signedIn = userName != null, userName = userName.orEmpty(), tickets = base.tickets.copy(canCorrect = manager))

/** Both ticket lists in one, newest first (`ticketRows` of `ReportsView.vue`; fiscal event ids grow with time). */
internal fun mergeRows(
    copyable: List<CopyableTicket>,
    refundable: List<RefundableEvent>,
): List<TicketRow> {
    val rows = LinkedHashMap<Long, TicketRow>()
    for (ticket in copyable) {
        rows[ticket.fiscalEventId] =
            TicketRow(
                fiscalEventId = ticket.fiscalEventId,
                ticketNumber = ticket.ticketNumber,
                tableNumber = ticket.tableNumber,
                totalCents = ticket.totalCents,
                createdAt = ticket.createdAt,
                kind = kindOf(ticket),
                copyable = ticket,
                refundable = null,
            )
    }
    for (event in refundable) {
        val row = rows[event.fiscalEventId]
        rows[event.fiscalEventId] =
            row?.copy(refundable = event)
                ?: TicketRow(
                    fiscalEventId = event.fiscalEventId,
                    ticketNumber = event.ticketNumber,
                    tableNumber = event.tableNumber,
                    totalCents = event.totalCents,
                    createdAt = event.createdAt,
                    kind = TicketKind.SALE,
                    copyable = null,
                    refundable = event,
                )
    }
    return rows.values.sortedByDescending { it.fiscalEventId }
}

private fun kindOf(ticket: CopyableTicket): TicketKind =
    when {
        ticket.eventLabel == LABEL_TRAINING -> TicketKind.TRAINING
        ticket.eventLabel == LABEL_PRE_BILL -> TicketKind.PRE_BILL
        ticket.totalCents < 0L -> TicketKind.REFUND
        else -> TicketKind.SALE
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
