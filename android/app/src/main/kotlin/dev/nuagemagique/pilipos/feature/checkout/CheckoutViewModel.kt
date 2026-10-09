package dev.nuagemagique.pilipos.feature.checkout

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import timber.log.Timber
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.SettlementPreview
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TicketDeliveryRequest
import java.util.concurrent.atomic.AtomicBoolean

/** How long the "slip not printed" warning stays, in milliseconds (`warnIfSlipFailed`: 8000). */
internal const val SLIP_TOAST_MS: Long = 8000L

/**
 * State holder of the checkout screen, a port of the script of `ui/src/views/CheckoutView.vue`.
 *
 * The engine owns every amount (PD208, UI-PD306). The total is [Order.totalCents]. What is left to pay and the
 * cash to hand over come from `EngineHelpers.settlePreview` over the parts taken plus a cash part without
 * amount: the rest is the amount of the cash row, the rounded due is `cashDueCents`. A cash part taken before
 * the last one is held in the preview as an exact amount, so the engine, not Kotlin, subtracts it. Equal shares,
 * the change and the quick amounts are the integer functions of `CashDisplay`; the digits typed are read as
 * cents by `AmountEntryState`.
 *
 * Nothing is signed until the whole sale is paid. The last part calls `checkout` with the non-cash parts, then a
 * cash part without amount (Vue `payload`), and the delivery request of `DeliveryStore`. A refusal goes back to
 * the pay step with its text. It never navigates by itself (PD205): [CheckoutEffect]s follow an action of the
 * operator.
 */
@Suppress("TooManyFunctions")
internal class CheckoutViewModel(
    private val entry: CheckoutEntryPoint,
    private val routeOrderId: Long,
) : ViewModel() {
    private val order = entry.order()
    private val session = entry.session()
    private val gateway = entry.gateway()
    private val helpers = entry.helpers()
    private val delivery = entry.delivery()
    private val settings = entry.settings()
    private val own = MutableStateFlow(Own())
    private val effectChannel = Channel<CheckoutEffect>(Channel.BUFFERED)

    // Computing the amounts talks to the engine: one computation at a time, always from the latest state.
    private val work = Mutex()
    private val loading = AtomicBoolean(false)

    /** What is local to the screen: the choices of the operator and the amounts derived from them. */
    private data class Own(
        val step: CheckoutStep = CheckoutStep.PAY,
        val splitCount: Int = 1,
        val method: PaymentMethod = PaymentMethod.CASH,
        val taken: List<TakenPart> = emptyList(),
        val digits: String = "",
        val fresh: Boolean = false,
        val error: String? = null,
        val done: CheckoutDone? = null,
        val amounts: CheckoutAmounts = CheckoutAmounts(),
        val summary: CheckoutSummary = CheckoutSummary(),
        val loadTried: Boolean = false,
    )

    private data class OrderInputs(
        val order: Order?,
        val canTakePayment: Boolean,
        val labelMode: ProductLabelMode,
        val itemCount: Int,
    )

    private val orderInputs: Flow<OrderInputs> =
        combine(order.current, order.lines, session.canTakePayment, settings.productLabel) { current, lines, may, label ->
            OrderInputs(
                order = current,
                canTakePayment = may,
                labelMode = if (label == ProductLabel.CODE) ProductLabelMode.CODE else ProductLabelMode.NAME,
                itemCount = lines.sumOf { it.quantity },
            )
        }

    private fun inputsNow(): OrderInputs =
        OrderInputs(
            order = order.current.value,
            canTakePayment = session.canTakePayment.value,
            labelMode = if (settings.productLabel.value == ProductLabel.CODE) ProductLabelMode.CODE else ProductLabelMode.NAME,
            itemCount = order.lines.value.sumOf { it.quantity },
        )

    private fun snapshot(): CheckoutUiState = build(own.value, inputsNow(), delivery.mailStatus.value, delivery.request.value)

    /** What the screen draws. */
    val state: StateFlow<CheckoutUiState> =
        combine(own, orderInputs, delivery.mailStatus, delivery.request) { local, inputs, mail, request ->
            build(local, inputs, mail, request)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            snapshot(),
        )

    /** One-shot moves, each delivered once. */
    val effects: Flow<CheckoutEffect> = effectChannel.receiveAsFlow()

    init {
        // The route's order: loaded through the store once the engine is ready and somebody is logged in.
        viewModelScope.launch {
            combine(gateway.ready, session.currentUser.map { it != null }) { ready, signedIn -> ready && signedIn }
                .distinctUntilChanged()
                .collect { usable ->
                    if (usable) {
                        ensureOrder()
                        refreshMail()
                    }
                }
        }
        // A new order total (the order was read again): the amounts and the entry start over (Vue `watch totalCents`).
        viewModelScope.launch {
            order.current
                .map { current -> current?.let { it.id to it.totalCents } }
                .distinctUntilChanged()
                .collect { if (own.value.step == CheckoutStep.PAY) recompute(prefill = true) }
        }
        viewModelScope.launch {
            order.lines.collect { lines -> if (own.value.step == CheckoutStep.PAY) summarize(lines) }
        }
    }

    // ── Choices of the operator ───────────────────────────────────

    /** The method buttons: cash is refused once a cash part is taken (Vue `selectMethod`). */
    fun selectMethod(method: PaymentMethod) {
        val local = own.value
        if (local.step != CheckoutStep.PAY || method !in TILL_METHODS) return
        if (method == PaymentMethod.CASH && local.taken.any { it.method == PaymentMethod.CASH }) return
        own.update { it.copy(method = method) }
        launchRecompute(prefill = true)
    }

    /** The split buttons: [parts] equal parts, only before the first part is taken (Vue `selectSplit`). */
    fun selectSplit(parts: Int) {
        val local = own.value
        val total = order.current.value?.totalCents
        val allowed =
            total != null &&
                local.step == CheckoutStep.PAY &&
                parts in 1..SPLIT_MAX &&
                local.taken.isEmpty() &&
                (parts == 1 || parts.toLong() <= total)
        if (!allowed) return
        own.update { it.copy(splitCount = parts) }
        launchRecompute(prefill = true)
    }

    /** A key of the pad: a shortcut or a prefilled amount is replaced by the first key, not appended to. */
    fun pressKey(key: NumPadKey) {
        own.update { local ->
            if (local.step != CheckoutStep.PAY) return@update local
            val base = if (local.fresh) AmountEntryState() else AmountEntryState(local.digits)
            local.copy(digits = base.press(key).digits, fresh = false)
        }
    }

    /** The "exact" shortcut: the rounded due for cash, the part for card and meal vouchers (Vue `setExact`). */
    fun setExact() {
        val local = own.value
        if (local.step != CheckoutStep.PAY) return
        val cents =
            if (local.method == PaymentMethod.CASH) local.amounts.roundedDueCents else local.amounts.dueCents
        own.update { it.copy(digits = digitsOf(cents), fresh = true) }
    }

    /** A quick amount (cash handed over), [cents] from `CashDisplay.quickAmountsCents`. */
    fun setQuickAmount(cents: Long) {
        if (own.value.step != CheckoutStep.PAY) return
        own.update { it.copy(digits = digitsOf(cents), fresh = true) }
    }

    /** Forgets the parts taken and goes back to cash (Vue `clearParts`). */
    fun clearParts() {
        if (own.value.step != CheckoutStep.PAY) return
        own.update { it.copy(taken = emptyList(), error = null, method = PaymentMethod.CASH) }
        launchRecompute(prefill = true)
    }

    /** The back button of the pay step: the order screen of the order being paid. */
    fun back() {
        effectChannel.trySend(CheckoutEffect.BackToOrder(routeOrderId))
    }

    /** The finish button of the done step: the tables, for the next customer. */
    fun finish() {
        own.update { it.copy(done = null) }
        effectChannel.trySend(CheckoutEffect.NewOrder)
    }

    // ── Taking a payment ──────────────────────────────────────────

    /**
     * Takes the current part: the sale is signed once nothing is left to pay, otherwise on to the next part
     * (Vue `validate`). Only for a role that may take a payment.
     */
    fun validate() {
        val snapshot = snapshot()
        if (!snapshot.canValidate) return
        val isCash = snapshot.method == PaymentMethod.CASH
        val part =
            if (isCash) {
                TakenPart(
                    method = PaymentMethod.CASH,
                    cents = snapshot.amounts.dueCents,
                    changeCents = snapshot.changeCents,
                )
            } else {
                TakenPart(snapshot.method, snapshot.entryCents, 0L)
            }
        if (part.cents >= snapshot.amounts.restCents) {
            startCheckout(snapshot.taken + part, snapshot.totalCents)
            return
        }
        val next = snapshot.taken + part
        // One cash part at most; otherwise back to cash, the default.
        val nextMethod =
            if (next.any { it.method == PaymentMethod.CASH }) {
                if (isCash) PaymentMethod.CARD else snapshot.method
            } else {
                PaymentMethod.CASH
            }
        own.update { it.copy(taken = next, method = nextMethod) }
        launchRecompute(prefill = true)
    }

    private fun startCheckout(
        parts: List<TakenPart>,
        totalCents: Long,
    ) {
        val current = order.current.value ?: return
        own.update { it.copy(error = null, step = CheckoutStep.PROCESSING) }
        viewModelScope.launch {
            val request = delivery.request.value.takeUnless { it.medium == RequestedMedium.PAPER }
            val outcome = attempt { gateway.call { engine -> engine.checkout(current.id, payload(parts), request) } }
            outcome
                .onSuccess { result -> onResult(result, parts, totalCents) }
                .onFailure { failure -> failCheckout(entry.errors().text(failure, settings.language.value)) }
        }
    }

    private suspend fun onResult(
        result: CheckoutResult,
        parts: List<TakenPart>,
        totalCents: Long,
    ) {
        if (!result.success) {
            failCheckout(entry.errors().textRaw(result.error, settings.language.value))
            return
        }
        if (result.kitchenSlip == SlipOutcome.FAILED) {
            val text = entry.catalog().text("kitchen_slip_failed", settings.language.value)
            entry.toasts().show(text, ToastVariant.Warning, SLIP_TOAST_MS)
        }
        val done =
            CheckoutDone(
                ticketLabel = result.ticket?.ticketTypeLabel,
                totalCents = totalCents,
                // One cash part at most: the change is that part's, no sum.
                changeCents = parts.firstOrNull { it.method == PaymentMethod.CASH }?.changeCents ?: 0L,
                fdmId = result.ticket?.fdmRef?.fdmId,
                fdmConnected = result.fdmConnected,
                delivery = result.delivery,
                fiscalEventId = result.fiscalEventId,
            )
        // The order leaves the store at once so it cannot be submitted twice; the done step reads from `done`.
        order.clearOrder()
        // The next customer starts from paper.
        delivery.reset()
        own.update { it.copy(step = CheckoutStep.DONE, taken = emptyList(), done = done) }
    }

    private fun failCheckout(text: String) {
        own.update { it.copy(error = text, step = CheckoutStep.PAY) }
        launchRecompute(prefill = false)
        viewModelScope.launch { refreshMail() }
    }

    /** The parts as `checkout` takes them: the non-cash parts with their amount, then the cash part without one. */
    private fun payload(parts: List<TakenPart>): List<PaymentPart> {
        val list = parts.filter { it.method != PaymentMethod.CASH }.map { PaymentPart(it.method, it.cents) }
        return if (parts.any { it.method == PaymentMethod.CASH }) list + PaymentPart(PaymentMethod.CASH, null) else list
    }

    // ── Delivery ──────────────────────────────────────────────────

    /** Chooses paper only. */
    fun chooseDeliveryPaper() = delivery.setPaper()

    /** Chooses an e-mail delivery to [email]. */
    fun chooseDeliveryEmail(
        medium: RequestedMedium,
        email: String,
    ) = delivery.setEmail(medium, email)

    /** Prints the paper original that an e-mail-only ticket still lacks (the one print action of the done step). */
    suspend fun printMissingOriginal(fiscalEventId: Long): Boolean = delivery.printMissingOriginal(fiscalEventId)

    private suspend fun refreshMail() {
        if (gateway.ready.value) delivery.refreshMailStatus()
    }

    // ── Loading and amounts ───────────────────────────────────────

    /** Loads the route's order when the store holds another one (the screen opened from a deep link or after a restart). */
    private suspend fun ensureOrder() {
        if (routeOrderId < 0L || own.value.step != CheckoutStep.PAY) return
        if (order.current.value?.id == routeOrderId) {
            own.update { it.copy(loadTried = true) }
            return
        }
        if (loading.compareAndSet(false, true)) {
            try {
                attempt { order.loadOrder(routeOrderId) }.onFailure { entry.errors().show(it, settings.language.value) }
            } finally {
                loading.set(false)
                own.update { it.copy(loadTried = true) }
            }
        }
    }

    private fun launchRecompute(prefill: Boolean) {
        viewModelScope.launch { recompute(prefill) }
    }

    private suspend fun summarize(lines: List<CartLine>) {
        if (lines.isEmpty()) {
            own.update { it.copy(summary = CheckoutSummary()) }
            return
        }
        val amounts = entry.amounts()
        val summary =
            attempt {
                CheckoutSummary(lines, lines.map { amounts.lineAmount(it) }, amounts.vatRows(lines))
            }.getOrElse {
                Timber.d(it, "Checkout recap amounts not read")
                CheckoutSummary(lines, lines.map { line -> LineAmount.PerUnit(line.quantity, line.unitPriceCents) }, emptyList())
            }
        own.update { it.copy(summary = summary) }
    }

    /** Reads the amounts again from the engine; with [prefill] the entry starts over (Vue `prefill`). */
    private suspend fun recompute(prefill: Boolean) =
        work.withLock {
            val total = order.current.value?.totalCents
            val local = own.value
            if (total == null || local.step == CheckoutStep.DONE) {
                own.update { it.copy(amounts = CheckoutAmounts()) }
                return@withLock
            }
            val amounts =
                attempt { amountsFor(total, local) }.getOrElse {
                    Timber.d(it, "Checkout amounts not read")
                    return@withLock
                }
            own.update { current ->
                val next = current.copy(amounts = amounts)
                if (!prefill) next else next.prefilled()
            }
        }

    private fun Own.prefilled(): Own =
        // Cash starts empty (tap Exact or an amount); card and meal vouchers start at the amount due.
        if (method == PaymentMethod.CASH || amounts.dueCents <= 0L) {
            copy(digits = "", fresh = false)
        } else {
            copy(digits = digitsOf(amounts.dueCents), fresh = true)
        }

    @Suppress("LongMethod")
    private suspend fun amountsFor(
        total: Long,
        local: Own,
    ): CheckoutAmounts {
        val shares = CashDisplay.equalShares(total, local.splitCount)
        val partIndex = local.taken.size
        val isLast = partIndex >= local.splitCount - 1
        val preview = previewAfter(total, local.taken, emptyList())
        val rest = cashRowOf(preview)
        val due = if (isLast) rest else minOf(shares[partIndex], rest)
        val cash = local.method == PaymentMethod.CASH
        // The engine rounds the cash rest; a cash part before the last one is its share, rounded on its own.
        val rounded =
            when {
                !cash -> due
                isLast -> preview.cashDueCents
                else -> helpers.settlePreview(due, listOf(CASH_PART)).cashDueCents
            }
        return CheckoutAmounts(
            restCents = rest,
            dueCents = due,
            roundedDueCents = rounded,
            restShownCents = if (cash && isLast) preview.cashDueCents else rest,
            shares = shares,
            chips = chipsFor(total, local, shares, rest),
            quickAmountsCents = if (cash) CashDisplay.quickAmountsCents(rounded) else emptyList(),
        )
    }

    /** Every part: taken, the current one, and the ones to come (Vue `partChips`). */
    private suspend fun chipsFor(
        total: Long,
        local: Own,
        shares: List<Long>,
        rest: Long,
    ): List<PartChip> {
        val count = maxOf(local.splitCount, local.taken.size + if (rest > 0L) 1 else 0)
        if (count <= 1) return emptyList()
        val chips = mutableListOf<PartChip>()
        val consumed = mutableListOf<Long>()
        var left = rest
        for (index in 0 until count) {
            val taken = local.taken.getOrNull(index)
            if (taken != null) {
                chips += PartChip(ChipState.PAID, taken.method, taken.cents)
                continue
            }
            val amount = if (index >= local.splitCount - 1) left else minOf(shares[index], left)
            val state = if (index == local.taken.size) ChipState.CURRENT else ChipState.NEXT
            chips += PartChip(state, if (state == ChipState.CURRENT) local.method else null, amount)
            if (amount > 0L) consumed += amount
            // What is left after this chip: the engine's cash rest with the chips so far held as exact parts.
            left = cashRowOf(previewAfter(total, local.taken, consumed))
        }
        return chips
    }

    /**
     * The preview of the sale with the parts taken, [exact] amounts held as exact card parts (the chips so far),
     * and a cash part without amount that pays the rest. A cash part already taken is held as an exact card part
     * too: the preview only tells how much is left, the sale itself is sent as [payload].
     */
    private suspend fun previewAfter(
        total: Long,
        taken: List<TakenPart>,
        exact: List<Long>,
    ): SettlementPreview {
        val held = taken.map { PaymentPart(if (it.method == PaymentMethod.CASH) PaymentMethod.CARD else it.method, it.cents) }
        val standIns = exact.map { PaymentPart(PaymentMethod.CARD, it) }
        return helpers.settlePreview(total, held + standIns + CASH_PART)
    }

    /** The cash row of the preview is what is left; no row when nothing is left. */
    private fun cashRowOf(preview: SettlementPreview): Long =
        preview.payments.firstOrNull { it.method == PaymentMethod.CASH }?.amountCents ?: 0L

    private fun build(
        local: Own,
        inputs: OrderInputs,
        mail: MailStatus?,
        request: TicketDeliveryRequest?,
    ): CheckoutUiState {
        val current = inputs.order
        return CheckoutUiState(
            orderId = routeOrderId,
            hasOrder = current != null && current.id == routeOrderId,
            loadTried = local.loadTried,
            orderType = current?.orderType,
            tableNumber = current?.tableNumber,
            totalCents = current?.totalCents ?: 0L,
            canTakePayment = inputs.canTakePayment,
            step = local.step,
            splitCount = local.splitCount,
            method = local.method,
            taken = local.taken,
            entry = AmountEntryState(local.digits),
            fresh = local.fresh,
            amounts = local.amounts,
            summary = local.summary,
            error = local.error,
            done = local.done,
            mailStatus = mail,
            deliveryRequest = request,
            labelMode = inputs.labelMode,
            itemCount = inputs.itemCount,
        )
    }

    private companion object {
        /** A cash part: it has no amount and pays what is left. */
        val CASH_PART = PaymentPart(PaymentMethod.CASH, null)

        /** The digits of [cents] for the entry (the digits are the cents, no arithmetic). */
        fun digitsOf(cents: Long): String = if (cents > 0L) cents.toString() else ""
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
