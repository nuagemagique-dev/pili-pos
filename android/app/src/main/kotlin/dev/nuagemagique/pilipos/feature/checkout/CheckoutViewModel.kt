package dev.nuagemagique.pilipos.feature.checkout

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.components.GIFT_VOUCHER_CODE_LENGTH
import dev.nuagemagique.pilipos.ui.components.GIFT_VOUCHER_DEPARTMENT
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.pressVoucherKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.state.DishUnits
import dev.nuagemagique.pilipos.ui.state.HeldPart
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import dev.nuagemagique.pilipos.ui.state.isOpen
import dev.nuagemagique.pilipos.ui.state.isUsable
import dev.nuagemagique.pilipos.ui.state.tableLabel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
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
import uniffi.pos_ffi.EftPayment
import uniffi.pos_ffi.GiftVoucherView
import uniffi.pos_ffi.HandoverRequest
import uniffi.pos_ffi.HeldCourse
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.SettlementPreview
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TicketDeliveryRequest
import java.util.concurrent.atomic.AtomicBoolean

/** How long the "slip not printed" warning stays, in milliseconds (`warnIfSlipFailed`: 8000). */
internal const val SLIP_TOAST_MS: Long = 8000L

/** How long the done step stays before it returns to the tables by itself (`DONE_AUTO_RETURN_MS` of the Vue view). */
internal const val DONE_AUTO_RETURN_MS: Long = 2000L

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
@Suppress("TooManyFunctions", "LargeClass") // One screen state holder: the parts, the terminal, the sale and the online mode.
internal class CheckoutViewModel(
    private val entry: CheckoutEntryPoint,
    private val routeOrderId: Long,
    /** An unpaid online order collected from the board (UX audit O2): the sale is its handover. */
    private val online: Boolean = false,
    /** An online collection starts on the card method (the "Encaisser carte" button of the board). */
    startOnCard: Boolean = false,
) : ViewModel() {
    private val order = entry.order()
    private val session = entry.session()
    private val gateway = entry.gateway()
    private val helpers = entry.helpers()
    private val delivery = entry.delivery()
    private val settings = entry.settings()
    private val eft = entry.eft()
    private val own = MutableStateFlow(Own(method = if (online && startOnCard) PaymentMethod.CARD else PaymentMethod.CASH))

    // The parts typed by hand are kept with the order once they were restored (UX audit P4).
    @Volatile
    private var restored = false
    private val effectChannel = Channel<CheckoutEffect>(Channel.BUFFERED)

    // Computing the amounts talks to the engine: one computation at a time, always from the latest state.
    private val work = Mutex()
    private val loading = AtomicBoolean(false)

    // One terminal payment at a time: a second tap while the overlay is up does nothing.
    private val eftBusy = AtomicBoolean(false)

    /** What is local to the screen: the choices of the operator and the amounts derived from them. */
    private data class Own(
        val step: CheckoutStep = CheckoutStep.PAY,
        val splitCount: Int = 1,
        val splitMode: SplitMode = SplitMode.EQUAL,
        val pick: Map<Long, Int> = emptyMap(),
        val otherSplitOpen: Boolean = false,
        val method: PaymentMethod = PaymentMethod.CASH,
        val taken: List<TakenPart> = emptyList(),
        val digits: String = "",
        val fresh: Boolean = false,
        val error: String? = null,
        val done: CheckoutDone? = null,
        val amounts: CheckoutAmounts = CheckoutAmounts(),
        val summary: CheckoutSummary = CheckoutSummary(),
        val loadTried: Boolean = false,
        val cardManual: Boolean = false,
        val sendingHeld: Boolean = false,
        val giftVouchersEnabled: Boolean = false,
        val voucher: VoucherDialogState? = null,
    )

    private data class OrderInputs(
        val order: Order?,
        val canTakePayment: Boolean,
        val labelMode: ProductLabelMode,
        val itemCount: Int,
        val eftActive: Boolean,
        val nextHeld: Int? = null,
        val sellsVoucher: Boolean = false,
        val zones: Map<Int, String> = emptyMap(),
        val covers: Int? = null,
    )

    // A terminal is linked and the till is not in training: card parts go to the terminal.
    private val eftActive: Flow<Boolean> =
        combine(settings.settings, settings.trainingMode) { current, training -> current?.paymentTerminal != null && !training }

    private data class Look(
        val label: ProductLabel,
        val terminal: Boolean,
        val nextHeld: Int?,
        val covers: Int?,
    )

    // The label, the terminal and the next held course travel together, so that the inputs combine four flows.
    private val labelAndTerminal: Flow<Look> =
        combine(
            settings.productLabel,
            eftActive,
            order.courseSettings,
            order.heldCourses,
            order.covers,
        ) { label, active, courses, held, covers ->
            Look(label, active, nextHeldOf(courses.enabled, held), covers)
        }

    private val orderInputs: Flow<OrderInputs> =
        combine(
            order.current,
            order.lines,
            session.canTakePayment,
            labelAndTerminal,
            order.tableZones,
        ) { current, lines, may, look, zones ->
            val label = look.label
            OrderInputs(
                order = current,
                canTakePayment = may,
                labelMode = if (label == ProductLabel.CODE) ProductLabelMode.CODE else ProductLabelMode.NAME,
                itemCount = lines.sumOf { it.quantity },
                eftActive = look.terminal,
                nextHeld = look.nextHeld,
                sellsVoucher = lines.any { it.departmentId == GIFT_VOUCHER_DEPARTMENT },
                zones = zones,
                covers = look.covers,
            )
        }

    private fun inputsNow(): OrderInputs =
        OrderInputs(
            order = order.current.value,
            canTakePayment = session.canTakePayment.value,
            labelMode = if (settings.productLabel.value == ProductLabel.CODE) ProductLabelMode.CODE else ProductLabelMode.NAME,
            itemCount = order.lines.value.sumOf { it.quantity },
            eftActive = eft.active(),
            nextHeld = nextHeldOf(order.courseSettings.value.enabled, order.heldCourses.value),
            sellsVoucher = order.lines.value.any { it.departmentId == GIFT_VOUCHER_DEPARTMENT },
            zones = order.tableZones.value,
            covers = order.covers.value,
        )

    private fun snapshot(): CheckoutUiState =
        build(own.value, inputsNow(), delivery.mailStatus.value, delivery.request.value, eft.payments.value)

    /** What the screen draws. */
    val state: StateFlow<CheckoutUiState> =
        combine(own, orderInputs, delivery.mailStatus, delivery.request, eft.payments) { local, inputs, mail, request, payments ->
            build(local, inputs, mail, request, payments)
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
                        loadGiftVoucherSetting()
                        refreshEft()
                        restoreHeld()
                    }
                }
        }
        // The terminal's payments of the order (recovery after a crash): once the settings say a terminal is linked.
        viewModelScope.launch {
            combine(
                gateway.ready,
                session.currentUser.map { it != null },
                settings.settings.map { it?.paymentTerminal != null },
            ) { ready, signedIn, linked -> ready && signedIn && linked }
                .distinctUntilChanged()
                .collect { if (it) refreshEft() }
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
        // The parts taken by hand stay with the order when the operator goes back to it (UX audit P4).
        viewModelScope.launch {
            own
                .map { local -> local.taken.filterNot { it.locked } }
                .distinctUntilChanged()
                .collect { manual ->
                    if (restored) order.holdParts(routeOrderId, manual.map { it.toHeld() })
                }
        }
        // An approved payment the engine knows (after a crash, a restart or a check) must be used by this sale: it
        // becomes a locked card part (`watch eft.approved` of the Vue view).
        viewModelScope.launch {
            eft.payments.collect { list -> list.filter { it.isUsable() }.forEach { adopt(it) } }
        }
    }

    override fun onCleared() {
        // The payments are the order's: the next screen reads them again from the engine.
        eft.clear()
    }

    // ── Choices of the operator ───────────────────────────────────

    /** The method buttons (Vue `selectMethod`); several cash parts are allowed, they are merged into one cash line. */
    fun selectMethod(method: PaymentMethod) {
        val local = own.value
        val usable = local.step == CheckoutStep.PAY && method in snapshot().methods
        if (!usable) return
        if (method == PaymentMethod.GIFT_VOUCHER) {
            // The gift voucher method opens its dialog; the selected method stays as it is.
            own.update { it.copy(voucher = VoucherDialogState()) }
        } else {
            own.update { it.copy(method = method) }
            launchRecompute(prefill = true)
        }
    }

    /** The split buttons: [parts] equal parts (1 to 20), only before the first part is taken (Vue `selectSplit`). */
    fun selectSplit(parts: Int) {
        val local = own.value
        val total = order.current.value?.totalCents
        val allowed =
            total != null &&
                local.step == CheckoutStep.PAY &&
                parts in 1..SPLIT_OTHER_MAX &&
                local.taken.isEmpty() &&
                (parts == 1 || parts.toLong() <= total)
        if (!allowed) return
        own.update { it.copy(splitCount = parts, splitMode = SplitMode.EQUAL, pick = emptyMap(), otherSplitOpen = false) }
        launchRecompute(prefill = true)
    }

    /** "Autre": opens the pad for any number of equal parts, only before the first part is taken. */
    fun openOtherSplit() {
        val local = own.value
        if (local.step != CheckoutStep.PAY || !(local.taken.isEmpty() || local.splitCount > SPLIT_QUICK_MAX)) return
        own.update { it.copy(otherSplitOpen = true) }
    }

    /** The pad of "Autre" was closed without a number. */
    fun closeOtherSplit() {
        own.update { it.copy(otherSplitOpen = false) }
    }

    /** The pad of "Autre" gave [parts] equal parts (2 to 20). */
    fun confirmOtherSplit(parts: Int) {
        own.update { it.copy(otherSplitOpen = false) }
        if (parts in SPLIT_OTHER_MIN..SPLIT_OTHER_MAX) selectSplit(parts)
    }

    /** "Par plats": the cart lines are checked unit by unit for each part, only before the first part is taken. */
    fun selectDishes() {
        val local = own.value
        if (local.step != CheckoutStep.PAY || local.taken.isNotEmpty() || local.splitMode == SplitMode.DISHES) return
        own.update { it.copy(splitMode = SplitMode.DISHES, splitCount = 1, pick = emptyMap()) }
        launchRecompute(prefill = true)
    }

    /** A tap on a dish: every unit left is checked, or none when all of them already were. */
    fun toggleDish(lineId: Long) = changeDish(lineId) { pick, left -> toggledUnits(pick, lineId, left) }

    /** The minus and plus of a dish of several units: one unit more or less for this part. */
    fun stepDish(
        lineId: Long,
        delta: Int,
    ) = changeDish(lineId) { pick, left -> withUnits(pick, lineId, left, (pick[lineId] ?: 0) + delta) }

    private fun changeDish(
        lineId: Long,
        change: (Map<Long, Int>, Int) -> Map<Long, Int>,
    ) {
        val local = own.value
        if (local.step != CheckoutStep.PAY || local.splitMode != SplitMode.DISHES) return
        val line = dishLinesOf(order.lines.value).firstOrNull { it.id == lineId } ?: return
        val left = unitsLeft(line, paidUnits(local.taken))
        own.update { it.copy(pick = change(it.pick, left)) }
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

    /** Forgets the parts taken and goes back to cash (Vue `clearParts`); a card the terminal charged stays. */
    fun clearParts() {
        if (own.value.step != CheckoutStep.PAY) return
        own.update {
            it.copy(taken = it.taken.filter { part -> part.locked }, pick = emptyMap(), error = null, method = PaymentMethod.CASH)
        }
        launchRecompute(prefill = true)
    }

    /** "Terminal" or "Saisie manuelle" for a card part: the card is charged on the terminal, or typed by hand. */
    fun setCardManual(manual: Boolean) {
        if (own.value.step != CheckoutStep.PAY) return
        own.update { it.copy(cardManual = manual) }
    }

    /** "Verifier": asks the terminal where the waiting payment [payment] stands; an approval becomes a locked part. */
    fun verifyOpen(payment: EftPayment) {
        viewModelScope.launch {
            eft.verify(payment)?.let { adopt(it) }
            recompute(prefill = true)
        }
    }

    /**
     * "Envoyer maintenant" of the held-course warning (`sendHeldNow`): the engine signs what is unsent, releases the
     * next held course and prints it; a refusal is shown in the footer.
     */
    fun sendHeldNow() {
        if (own.value.sendingHeld || own.value.step != CheckoutStep.PAY) return
        own.update { it.copy(sendingHeld = true, error = null) }
        viewModelScope.launch {
            attempt { order.sendNextCourse() }.onFailure { failure ->
                own.update { it.copy(error = entry.errors().text(failure, settings.language.value)) }
            }
            own.update { it.copy(sendingHeld = false) }
        }
    }

    // ── Gift voucher ──────────────────────────────────────────────

    /** Reads the gift voucher switch (`load` of `giftVouchers.ts`); a till that does not answer has vouchers off. */
    private suspend fun loadGiftVoucherSetting() {
        val enabled = attempt { gateway.call { engine -> engine.getGiftVoucherSettings() } }.map { it.enabled }.getOrDefault(false)
        own.update { it.copy(giftVouchersEnabled = enabled) }
    }

    /** A key of the code pad of the dialog: a change of the digits forgets the voucher found and the refusal. */
    fun voucherKey(key: NumPadKey) {
        own.update { local ->
            val dialog = local.voucher ?: return@update local
            local.copy(voucher = dialog.copy(digits = pressVoucherKey(dialog.digits, key), found = null, error = null))
        }
    }

    /** "Chercher": the engine finds the voucher by its code (5 failures a minute, then it makes the operator wait). */
    fun lookupVoucher() {
        val dialog = own.value.voucher ?: return
        if (dialog.digits.length != GIFT_VOUCHER_CODE_LENGTH || dialog.busy) return
        own.update { it.copy(voucher = dialog.copy(busy = true, error = null)) }
        viewModelScope.launch {
            attempt { gateway.call { engine -> engine.lookupGiftVoucher(dialog.digits) } }.fold(
                onSuccess = { view -> updateVoucher(dialog.digits) { it.copy(found = view, busy = false) } },
                onFailure = { failure ->
                    val text = entry.errors().text(failure, settings.language.value)
                    updateVoucher(dialog.digits) { it.copy(found = null, error = text, busy = false) }
                },
            )
        }
    }

    /** Changes the open dialog if it still shows [digits] (a dialog closed or retyped meanwhile is left alone). */
    private fun updateVoucher(
        digits: String,
        change: (VoucherDialogState) -> VoucherDialogState,
    ) {
        own.update { local ->
            val dialog = local.voucher
            if (dialog != null && dialog.digits == digits) local.copy(voucher = change(dialog)) else local
        }
    }

    /** Closes the dialog without taking anything. */
    fun closeVoucher() {
        own.update { it.copy(voucher = null) }
    }

    /** "Utiliser ce bon": the voucher pays the smaller of its balance and what is left (`takeVoucher`). */
    fun applyVoucher() {
        val found = own.value.voucher?.found
        val cents = if (found?.status == "active") minOf(found.balanceCents, snapshot().amounts.restCents) else 0L
        if (found != null && cents > 0L) {
            own.update { it.copy(voucher = null) }
            takeVoucher(found, cents)
        }
    }

    private fun takeVoucher(
        voucher: GiftVoucherView,
        cents: Long,
    ) {
        val snapshot = snapshot()
        if (snapshot.taken.any { it.giftVoucherId == voucher.id }) return
        val part =
            TakenPart(
                method = PaymentMethod.GIFT_VOUCHER,
                cents = cents,
                changeCents = 0L,
                giftVoucherId = voucher.id,
                codeMasked = voucher.codeMasked,
                units = currentUnits(snapshot),
            )
        if (cents >= snapshot.amounts.restCents) {
            startCheckout(snapshot.taken + part, snapshot.totalCents)
            return
        }
        // The method selected stays (a voucher is not a method of its own).
        own.update { it.copy(taken = snapshot.taken + part, pick = emptyMap()) }
        launchRecompute(prefill = true)
    }

    /** The back button of the pay step: the order screen of the order being paid. */
    fun back() {
        effectChannel.trySend(if (online) CheckoutEffect.BackToBoard else CheckoutEffect.BackToOrder(routeOrderId))
    }

    /**
     * The finish button, a tap on the done step, or its own timer: the tables, for the next customer. Only once per
     * sale, whichever comes first.
     */
    fun finish() {
        var leaving = false
        own.update { local ->
            leaving = local.step == CheckoutStep.DONE && local.done != null
            if (leaving) local.copy(done = null) else local
        }
        // An online order goes back to the board it came from, the others to the tables.
        if (leaving) effectChannel.trySend(if (online) CheckoutEffect.BoardAfterSale else CheckoutEffect.NewOrder)
    }

    /** The done step leaves by itself after [DONE_AUTO_RETURN_MS] unless it holds something to read or decide. */
    private fun scheduleAutoReturn(done: CheckoutDone) {
        if (done.needsAttention) return
        viewModelScope.launch {
            delay(DONE_AUTO_RETURN_MS)
            if (own.value.done === done) finish()
        }
    }

    // ── Taking a payment ──────────────────────────────────────────

    /**
     * Takes the current part: the sale is signed once nothing is left to pay, otherwise on to the next part
     * (Vue `validate`). Only for a role that may take a payment.
     */
    @Suppress("ReturnCount")
    fun validate() {
        val snapshot = snapshot()
        if (!snapshot.canValidate) return
        if (snapshot.amounts.allPaid) {
            startCheckout(snapshot.taken, snapshot.totalCents)
            return
        }
        val units = currentUnits(snapshot)
        val part =
            if (snapshot.method == PaymentMethod.CASH) {
                TakenPart(
                    method = PaymentMethod.CASH,
                    cents = snapshot.amounts.dueCents,
                    changeCents = snapshot.changeCents,
                    givenCents = snapshot.entryCents,
                    units = units,
                )
            } else {
                TakenPart(snapshot.method, snapshot.entryCents, 0L, units = units)
            }
        if (snapshot.viaTerminal) {
            takeOnTerminal(snapshot, part.cents, units)
            return
        }
        if (part.cents >= snapshot.amounts.restCents) {
            startCheckout(snapshot.taken + part, snapshot.totalCents)
            return
        }
        // The method stays selected for the next part (UX audit P3); the "Reste" row offers the others.
        own.update { it.copy(taken = snapshot.taken + part, pick = emptyMap()) }
        launchRecompute(prefill = true)
    }

    /**
     * Charges the card part on the terminal (the overlay waits for the card). An approved payment becomes a locked
     * part, and the sale is signed when nothing is left; a refusal leaves the part to be typed again (or entered by
     * hand with "Saisie manuelle").
     */
    private fun takeOnTerminal(
        snapshot: CheckoutUiState,
        cents: Long,
        units: List<DishUnits>,
    ) {
        if (!eftBusy.compareAndSet(false, true)) return
        viewModelScope.launch {
            try {
                val count = if (snapshot.splitCount > 1) snapshot.splitCount else 1
                val paid = eft.charge(routeOrderId, cents, snapshot.partIndex + 1, count) ?: return@launch
                adopt(paid)
                // The part by dishes keeps the units it pays.
                own.update { local ->
                    local.copy(
                        taken = local.taken.map { if (it.eftPaymentId == paid.id && units.isNotEmpty()) it.copy(units = units) else it },
                        pick = emptyMap(),
                    )
                }
                recompute(prefill = true)
                val now = snapshot()
                // Otherwise the method stays selected for the next part.
                if (now.amounts.allPaid) startCheckout(now.taken, now.totalCents)
            } finally {
                eftBusy.set(false)
            }
        }
    }

    /** An approved payment becomes a locked card part, once. */
    private fun adopt(payment: EftPayment) {
        if (!payment.isUsable()) return
        var added = false
        own.update { local ->
            if (local.taken.any { it.eftPaymentId == payment.id }) {
                added = false
                local
            } else {
                added = true
                local.copy(
                    taken =
                        local.taken +
                            TakenPart(PaymentMethod.CARD, payment.amountCents, 0L, payment.id, payment.brand, payment.mealVoucher),
                )
            }
        }
        if (added && own.value.step == CheckoutStep.PAY) launchRecompute(prefill = true)
    }

    /**
     * Brings back the parts typed by hand that were kept with the order (UX audit P4), once the card payments the
     * terminal approved are known. If the order changed so that a part no longer fits, they are all dropped with a
     * warning. From then on every change of the parts is kept again.
     */
    private suspend fun restoreHeld() {
        if (restored) return
        val total = order.current.value?.takeIf { it.id == routeOrderId }?.totalCents
        if (total == null) {
            restored = true
            return
        }
        eft.payments.value.filter { it.isUsable() }.forEach { adopt(it) }
        val locked = CashDisplay.sumCents(own.value.taken.map { it.cents })
        val reconciled = reconcileHeld(order.heldPartsFor(routeOrderId), total, locked)
        // A part by dishes needs its units to still be in the cart.
        val unitsGone = !unitsFit(dishLinesOf(order.lines.value), reconciled.parts.map { it.units })
        val kept = if (unitsGone) ReconciledParts(emptyList(), dropped = true) else reconciled
        if (kept.dropped) {
            entry.toasts().show(entry.catalog().text("parts_dropped", settings.language.value), ToastVariant.Warning, SLIP_TOAST_MS)
        }
        own.update { local ->
            val byDishes = kept.parts.any { it.units.isNotEmpty() }
            local.copy(
                taken = local.taken + kept.parts.map { it.toTaken() },
                splitMode = if (byDishes) SplitMode.DISHES else local.splitMode,
            )
        }
        restored = true
        if (kept.dropped) order.holdParts(routeOrderId, emptyList())
        recompute(prefill = true)
    }

    private suspend fun refreshEft() {
        if (gateway.ready.value && routeOrderId >= 0L && eft.linked()) eft.load(routeOrderId)
    }

    private fun startCheckout(
        parts: List<TakenPart>,
        totalCents: Long,
    ) {
        val current = order.current.value ?: return
        own.update { it.copy(error = null, step = CheckoutStep.PROCESSING) }
        viewModelScope.launch {
            val request = delivery.request.value.takeUnless { it.medium == RequestedMedium.PAPER }
            // An online order is sold by its handover (its P is closed by the sale N); the parts are the same.
            val outcome =
                attempt {
                    gateway.call { engine ->
                        if (online) {
                            engine.webshopHandoverOrder(HandoverRequest(current.id, payload(parts), request))
                        } else {
                            engine.checkout(current.id, payload(parts), request)
                        }
                    }
                }
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
                // The change of the last cash part: the earlier ones were handed back when they were taken.
                changeCents = parts.lastOrNull { it.method == PaymentMethod.CASH }?.changeCents ?: 0L,
                fdmId = result.ticket?.fdmRef?.fdmId,
                fdmConnected = result.fdmConnected,
                delivery = result.delivery,
                fiscalEventId = result.fiscalEventId,
                voucherPrints = result.giftVoucherPrints,
            )
        // The order leaves the store at once so it cannot be submitted twice; the done step reads from `done`.
        order.clearOrder()
        eft.clear()
        // The next customer starts from paper.
        delivery.reset()
        own.update { it.copy(step = CheckoutStep.DONE, taken = emptyList(), done = done) }
        scheduleAutoReturn(done)
    }

    private fun failCheckout(text: String) {
        own.update { it.copy(error = text, step = CheckoutStep.PAY) }
        launchRecompute(prefill = false)
        viewModelScope.launch {
            refreshMail()
            refreshEft()
        }
    }

    /**
     * The parts as `checkout` takes them: the non-cash parts with their amount, then ONE cash part without amount for
     * all the cash parts taken: the signed sale has a single cash line, rounded once (spec v2.2 section 7.1). The
     * units of a split by dishes are not part of the request.
     */
    private fun payload(parts: List<TakenPart>): List<PaymentPart> {
        val list =
            parts
                .filter { it.method != PaymentMethod.CASH }
                .map { PaymentPart(it.method, it.cents, it.eftPaymentId, it.giftVoucherId) }
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
        if (order.current.value?.id == routeOrderId && (!online || order.onlineCollect.value != null)) {
            own.update { it.copy(loadTried = true) }
            return
        }
        if (loading.compareAndSet(false, true)) {
            try {
                attempt {
                    // An online order is shown without becoming the engine's active order.
                    if (online) order.beginOnlineCollect(routeOrderId, own.value.method) else order.loadOrder(routeOrderId)
                }.onFailure { entry.errors().show(it, settings.language.value) }
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
                CheckoutSummary(
                    lines,
                    lines.map { amounts.lineAmount(it) },
                    amounts.vatRows(lines),
                    CashDisplay.sumCents(lines.map { it.discountCents }),
                )
            }.getOrElse {
                Timber.d(it, "Checkout recap amounts not read")
                CheckoutSummary(
                    lines,
                    lines.map { line -> LineAmount.PerUnit(line.quantity, line.weighedAmountCents ?: line.unitPriceCents) },
                    emptyList(),
                    CashDisplay.sumCents(lines.map { line -> line.discountCents }),
                )
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
        // Every part starts at the exact amount: the rounded cash due for cash, the part for card and meal vouchers.
        // The first key (or a quick amount) replaces it.
        (if (method == PaymentMethod.CASH) amounts.roundedDueCents else amounts.dueCents).let { exact ->
            if (exact <= 0L) copy(digits = "", fresh = false) else copy(digits = digitsOf(exact), fresh = true)
        }

    @Suppress("LongMethod")
    private suspend fun amountsFor(
        total: Long,
        local: Own,
    ): CheckoutAmounts {
        val shares = CashDisplay.equalShares(total, local.splitCount)
        val partIndex = local.taken.size
        val preview = previewAfter(total, local.taken, emptyList())
        val rest = cashRowOf(preview)
        val dishes = local.splitMode == SplitMode.DISHES
        val lines = dishLinesOf(order.lines.value)
        val paid = paidUnits(local.taken)
        val everything = dishes && local.pick.values.sum() > 0 && allLeftPicked(lines, paid, local.pick)
        val isLast = if (dishes) everything else partIndex >= local.splitCount - 1
        val due =
            when {
                dishes -> if (everything) rest else minOf(pickedCents(lines, paid, local.pick), rest)
                isLast -> rest
                else -> minOf(shares[partIndex], rest)
            }
        val cash = local.method == PaymentMethod.CASH
        // The sale is signed with ONE cash line, rounded once by the engine: each cash part hands over its share of
        // that rounding (the difference of the rounded running cash totals), so the parts add up to the cash line.
        val cashTaken = CashDisplay.sumCents(local.taken.filter { it.method == PaymentMethod.CASH }.map { it.cents })
        val rounded = if (cash) cashDueAfter(cashTaken, due) else due
        return CheckoutAmounts(
            restCents = rest,
            dueCents = due,
            roundedDueCents = rounded,
            restShownCents = if (cash && isLast) cashDueAfter(cashTaken, rest) else rest,
            shares = shares,
            chips = if (dishes) dishChips(local, due, rest) else chipsFor(total, local, shares, rest),
            quickAmountsCents = if (cash) CashDisplay.quickAmountsCents(rounded) else emptyList(),
            allPaid = local.taken.isNotEmpty() && rest == 0L,
            everythingPicked = everything,
        )
    }

    /**
     * The cash to hand over for a cash part of [partCents] when [takenCents] of cash parts were taken before: the
     * engine's rounding of the running cash total with the part, minus its rounding without it.
     */
    private suspend fun cashDueAfter(
        takenCents: Long,
        partCents: Long,
    ): Long {
        val before = if (takenCents > 0L) helpers.settlePreview(takenCents, listOf(CASH_PART)).cashDueCents else 0L
        val running = CashDisplay.sumCents(listOf(takenCents, partCents))
        val after = if (running > 0L) helpers.settlePreview(running, listOf(CASH_PART)).cashDueCents else 0L
        return CashDisplay.differenceCents(after, before)
    }

    /** By dishes the parts to come are not known: the parts taken, the one being taken, and what stays after it. */
    private fun dishChips(
        local: Own,
        due: Long,
        rest: Long,
    ): List<PartChip> {
        val chips = local.taken.map { PartChip(ChipState.PAID, it.displayMethod, it.cents) }.toMutableList()
        if (rest > 0L) {
            chips += PartChip(ChipState.CURRENT, local.method, due)
            val after = CashDisplay.differenceCents(rest, due)
            if (after > 0L) chips += PartChip(ChipState.NEXT, null, after)
        }
        return if (chips.size > 1) chips else emptyList()
    }

    /** The units checked, as the list a part keeps; empty unless the sale is split by dishes. */
    private fun currentUnits(snapshot: CheckoutUiState): List<DishUnits> =
        if (snapshot.dishMode) pickedUnits(dishLinesOf(order.lines.value), snapshot.pick) else emptyList()

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
                chips += PartChip(ChipState.PAID, taken.displayMethod, taken.cents)
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
        val held =
            taken.map {
                PaymentPart(if (it.method == PaymentMethod.CASH) PaymentMethod.CARD else it.method, it.cents, null, it.giftVoucherId)
            }
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
        payments: List<EftPayment>,
    ): CheckoutUiState {
        val current = inputs.order
        return CheckoutUiState(
            orderId = routeOrderId,
            hasOrder = current != null && current.id == routeOrderId,
            loadTried = local.loadTried,
            orderType = current?.orderType,
            tableNumber = current?.tableNumber,
            tableLabel = current?.tableNumber?.let { tableLabel(inputs.zones[it], it) },
            covers = if (current?.orderType == OrderType.DINE_IN) inputs.covers else null,
            totalCents = current?.totalCents ?: 0L,
            canTakePayment = inputs.canTakePayment,
            step = local.step,
            splitCount = local.splitCount,
            splitMode = local.splitMode,
            pick = local.pick,
            otherSplitOpen = local.otherSplitOpen,
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
            eftActive = inputs.eftActive,
            cardManual = local.cardManual,
            eftOpen = payments.filter { it.isOpen() },
            nextHeldCourse = inputs.nextHeld,
            sendingHeld = local.sendingHeld,
            giftVouchersEnabled = local.giftVouchersEnabled,
            sellsVoucher = inputs.sellsVoucher,
            voucher = local.voucher,
            online = online,
        )
    }

    private companion object {
        /** The next held course, when courses are on and one waits. */
        fun nextHeldOf(
            enabled: Boolean,
            held: List<HeldCourse>,
        ): Int? = if (enabled) held.minOfOrNull { it.course } else null

        fun TakenPart.toHeld(): HeldPart = HeldPart(method, cents, changeCents, giftVoucherId, codeMasked, givenCents, units)

        fun HeldPart.toTaken(): TakenPart =
            TakenPart(
                method,
                cents,
                changeCents,
                giftVoucherId = giftVoucherId,
                codeMasked = codeMasked,
                givenCents = givenCents,
                units = units,
            )

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
