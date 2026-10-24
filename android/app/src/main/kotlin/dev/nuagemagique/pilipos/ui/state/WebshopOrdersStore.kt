package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ERROR_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import timber.log.Timber
import uniffi.pos_ffi.AcceptRequest
import uniffi.pos_ffi.ApplyOutcome
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.HandoverRequest
import uniffi.pos_ffi.PatchStatusRequest
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RefundRequest
import uniffi.pos_ffi.SetConfirmedTimeRequest
import uniffi.pos_ffi.WebshopOrderSummary
import java.time.Clock
import java.time.Duration
import java.time.Instant
import java.time.OffsetDateTime
import java.time.format.DateTimeParseException
import javax.inject.Inject
import javax.inject.Singleton

/** The engine answered a sale-signing command with `success = false`; [raw] is the error text it sent. */
private class SaleRefusedException(
    val raw: String,
) : Exception(raw)

/**
 * The online orders board (port of `ui/src/stores/webshopOrders.ts`).
 *
 * The engine owns the orders. This keeps the last list it answered, refetches it when the engine turns ready
 * with HubRise visible and on `WebshopOrderReceived`, `WebshopOrderUpdated` and `Resync`, and runs the board
 * actions. Each action is one engine command followed by a refetch (the Vue store patches some orders
 * optimistically; the list is the engine's here). Actions answer false when the engine refused or the order is
 * busy; a refusal is shown as an error toast and kept in [lastError]. No amount is computed here: money is only
 * passed through from the engine's records, and the times are instants (preparation window, delays).
 */
@Suppress("TooManyFunctions", "LongParameterList")
@Singleton
class WebshopOrdersStore
    @Inject
    constructor(
        private val gateway: EngineGateway,
        events: EngineEvents,
        private val errors: ErrorPresenter,
        private val toasts: ToastStore,
        integrations: IntegrationsStore,
        private val settings: PosSettingsStore,
        private val catalog: MessageCatalog,
        private val eft: EftStore,
        private val helpers: EngineHelpers,
        @ApplicationScope scope: CoroutineScope,
    ) {
        private val mutableOrders = MutableStateFlow<List<WebshopOrderSummary>>(emptyList())
        private val mutableLoading = MutableStateFlow(false)
        private val mutableError = MutableStateFlow<String?>(null)
        private val mutableBusy = MutableStateFlow<Set<Long>>(emptySet())
        private val mutablePending = MutableStateFlow(0)
        private val mutableInProgress = MutableStateFlow(0)
        private val mutableReady = MutableStateFlow(0)
        private val mutableLoaded = MutableStateFlow(false)
        private val active = MutableStateFlow(false)
        private val fetchLock = Mutex()

        /** The time source of the preparation window and the delays; the seam of the tests. */
        internal var clock: Clock = Clock.systemUTC()

        /** The orders of the board, as the engine listed them last; empty before the first load. */
        val orders: StateFlow<List<WebshopOrderSummary>> = mutableOrders.asStateFlow()

        /** True while the list is being fetched. */
        val loading: StateFlow<Boolean> = mutableLoading.asStateFlow()

        /** The text of the last failure (fetch or action); a successful fetch clears a fetch failure. */
        val lastError: StateFlow<String?> = mutableError.asStateFlow()

        /** The ids of the orders an action is running on: one action at a time per order. */
        val busyOrderIds: StateFlow<Set<Long>> = mutableBusy.asStateFlow()

        /**
         * Orders waiting for the operator: to accept or reject, or signed and cancelled by HubRise (refund or
         * closing). The header badge and the order chime follow this count.
         */
        val pendingCount: StateFlow<Int> = mutablePending.asStateFlow()

        /** Orders accepted or in preparation. */
        val inProgressCount: StateFlow<Int> = mutableInProgress.asStateFlow()

        /** Orders waiting to be collected. */
        val readyCount: StateFlow<Int> = mutableReady.asStateFlow()

        /**
         * True once the list was fetched for the engine that is running: the first count is then a starting
         * point, not an arrival (the chime stays quiet for it). False again when the engine goes away or HubRise
         * is no longer visible.
         */
        val loaded: StateFlow<Boolean> = mutableLoaded.asStateFlow()

        init {
            // Undispatched: the collector is subscribed when the constructor returns (events have no replay).
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event -> onEvent(event) }
            }
            scope.launch {
                combine(gateway.ready, integrations.hubriseVisible) { ready, visible -> ready && visible }
                    .distinctUntilChanged()
                    .collect { on ->
                        active.value = on
                        if (on) fetchOrders() else forget()
                    }
            }
        }

        /** Fetches the orders from the engine. @return false when it failed (see [lastError]) */
        @Suppress("TooGenericExceptionCaught")
        suspend fun fetchOrders(): Boolean =
            fetchLock.withLock {
                mutableLoading.value = true
                mutableError.value = null
                try {
                    publish(gateway.call { engine -> engine.hubriseListWebshopOrders() })
                    true
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Timber.w(e, "Webshop orders not loaded")
                    mutableError.value = errors.text(e, settings.language.value)
                    false
                } finally {
                    mutableLoading.value = false
                }
            }

        /**
         * Accepts [order]: the engine signs it (N if prepaid, P if unpaid) and tells HubRise `accepted` with the
         * time promised to the customer, HubRise's own when it set one, else now plus the default preparation
         * window.
         */
        suspend fun acceptOrder(order: WebshopOrderSummary): Boolean {
            val window = Duration.ofMinutes(defaultPrepMinutes(order))
            val confirmed = order.confirmedTime ?: clock.instant().plus(window).toString()
            return run(order.id) {
                val result = gateway.call { engine -> engine.webshopAcceptOrder(AcceptRequest(order.id, confirmed)) }
                if (!result.kitchenPrinted) warnPrinterMissing()
            }
        }

        /** Refuses [order]. */
        suspend fun rejectOrder(order: WebshopOrderSummary): Boolean =
            run(order.id) { gateway.call { engine -> engine.webshopRejectOrder(order.id) } }

        /** The kitchen starts on the order of HubRise id [hubriseOrderId]. */
        suspend fun startPreparation(hubriseOrderId: String): Boolean = patchStatus(hubriseOrderId, STATUS_IN_PREPARATION)

        /** The order of HubRise id [hubriseOrderId] is ready to be collected. */
        suspend fun markReady(hubriseOrderId: String): Boolean = patchStatus(hubriseOrderId, STATUS_AWAITING_COLLECTION)

        /** The order of HubRise id [hubriseOrderId] left for delivery (the restaurant's own webshop). */
        suspend fun markOutForDelivery(hubriseOrderId: String): Boolean = patchStatus(hubriseOrderId, STATUS_IN_DELIVERY)

        /**
         * Pushes the promised time of the order of HubRise id [hubriseOrderId] forward by [minutes]. The increment
         * stacks on the current time (+15 twice moves it by 30 minutes) and starts from now when that time is
         * already past, so a stale target is never reinforced.
         */
        suspend fun delayByMinutes(
            hubriseOrderId: String,
            minutes: Int,
        ): Boolean {
            val now = clock.instant()
            val known = mutableOrders.value.firstOrNull { it.hubriseOrderId == hubriseOrderId }
            val current = known?.confirmedTime?.let(::parseInstant)
            val base = if (current != null && current.isAfter(now)) current else now
            val target = base.plus(Duration.ofMinutes(minutes.toLong())).toString()
            return run(null) {
                gateway.call { engine -> engine.hubriseSetConfirmedTime(SetConfirmedTimeRequest(hubriseOrderId, target)) }
                toasts.show("${text("webshop_toast_delay_ok")} +$minutes min", ToastVariant.Success)
            }
        }

        /**
         * Collects an unpaid order with [method]: cash for its total (the engine rounds), or the other method
         * for the total the engine listed.
         */
        suspend fun collectOrder(
            order: WebshopOrderSummary,
            method: PaymentMethod,
        ): Boolean {
            // A linked terminal charges the card first (the overlay of the checkout); the handover then carries it.
            if (method == PaymentMethod.CARD && eft.active()) return collectOnTerminal(order)
            val part = PaymentPart(method, if (method == PaymentMethod.CASH) null else order.totalCents)
            val request = HandoverRequest(order.id, listOf(part), null)
            return runSale(order.id) { gateway.call { engine -> engine.webshopHandoverOrder(request) } }
        }

        /**
         * Collects an unpaid order by card on the linked terminal. Recovery first: a payment of the order still
         * waiting for its answer is checked, an approved unused one is used as is (a charged card is never
         * forgotten); only what is left is asked of the terminal. Declined, cancelled or unanswered: nothing is
         * signed and the operator may try again.
         */
        private suspend fun collectOnTerminal(order: WebshopOrderSummary): Boolean =
            try {
                eft.load(order.id)
                if (chargeWhatIsLeft(order)) {
                    val parts = eft.approved().map { PaymentPart(PaymentMethod.CARD, it.amountCents, it.id) }
                    val request = HandoverRequest(order.id, parts, null)
                    runSale(order.id) { gateway.call { engine -> engine.webshopHandoverOrder(request) } }
                } else {
                    false
                }
            } finally {
                eft.clear()
            }

        /** True when the card payments of [order] cover its total: waiting ones checked, then the rest charged. */
        private suspend fun chargeWhatIsLeft(order: WebshopOrderSummary): Boolean {
            val checked = eft.open().all { waiting -> eft.verify(waiting) != null }
            if (!checked) return false
            // The UI computes no money: what is left is the cash row of the engine's settlement over the approved
            // card payments plus a cash part without amount.
            val held = eft.approved().map { PaymentPart(PaymentMethod.CARD, it.amountCents) }
            val preview = helpers.settlePreview(order.totalCents, held + PaymentPart(PaymentMethod.CASH, null))
            val rest = preview.payments.firstOrNull { it.method == PaymentMethod.CASH }?.amountCents ?: 0L
            return rest == 0L || eft.charge(order.id, rest) != null
        }

        /** A prepaid order leaves: its sale is already signed. */
        suspend fun completeOrder(order: WebshopOrderSummary): Boolean =
            run(order.id) { gateway.call { engine -> engine.webshopCompleteOrder(order.id) } }

        /** Refunds a signed order HubRise cancelled, confirmed by the logged-in manager [insz]. */
        suspend fun refundOrder(
            order: WebshopOrderSummary,
            insz: String,
        ): Boolean = runSale(order.id) { gateway.call { engine -> engine.webshopRefundOrder(RefundRequest(order.id, insz, null)) } }

        /** Closes an unpaid order cancelled after its P. */
        suspend fun closeCancelledOrder(order: WebshopOrderSummary): Boolean =
            runSale(order.id) { gateway.call { engine -> engine.webshopCloseCancelledOrder(order.id) } }

        /** Reprints the kitchen ticket of the order [orderId]. */
        suspend fun printKitchen(orderId: Long): Boolean = print { gateway.call { engine -> engine.printWebshopKitchen(orderId) } }

        /** Reprints the customer ticket of the order [orderId]. */
        suspend fun printCustomer(orderId: Long): Boolean = print { gateway.call { engine -> engine.printWebshopCustomer(orderId) } }

        /** Fetches the HubRise catalog again, then the orders it resolves. A refused catalog is reported. */
        suspend fun resync(): Boolean =
            run(null) {
                val outcome = gateway.call { engine -> engine.hubriseSyncCatalog() }
                if (outcome is ApplyOutcome.Rejected) {
                    toasts.show(text("hubrise_catalog_rejected"), ToastVariant.Error, ERROR_TOAST_TTL_MS)
                }
            }

        private suspend fun onEvent(event: PosEvent) {
            val concerned = event is PosEvent.WebshopOrderReceived || event is PosEvent.WebshopOrderUpdated || event is PosEvent.Resync
            if (concerned && active.value) fetchOrders()
        }

        private suspend fun patchStatus(
            hubriseOrderId: String,
            newStatus: String,
        ): Boolean {
            val localId = mutableOrders.value.firstOrNull { it.hubriseOrderId == hubriseOrderId }?.id
            return run(localId) {
                gateway.call { engine -> engine.hubrisePatchOrderStatus(PatchStatusRequest(hubriseOrderId, newStatus)) }
            }
        }

        /** Runs a command that signs a sale; a result without success is a refusal like a thrown one. */
        private suspend fun runSale(
            orderId: Long,
            command: suspend () -> CheckoutResult,
        ): Boolean =
            run(orderId) {
                val result = command()
                if (!result.success) throw SaleRefusedException(result.error ?: "internal")
            }

        /**
         * Runs a board [action] on [orderId] (null: no order is locked), shows its refusal, and reads the board
         * again whatever happened. An order with an action running refuses a second one.
         */
        @Suppress("TooGenericExceptionCaught")
        private suspend fun run(
            orderId: Long?,
            action: suspend () -> Unit,
        ): Boolean {
            if (orderId != null && !takeBusy(orderId)) return false
            var refusal: String? = null
            return try {
                action()
                true
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                refusal = report(e)
                false
            } finally {
                try {
                    fetchOrders()
                    // The refetch clears the error of a fetch; the refusal of the action stays.
                    if (refusal != null) mutableError.value = refusal
                } finally {
                    if (orderId != null) releaseBusy(orderId)
                }
            }
        }

        /** A reprint: a missing printer is a warning, any other refusal an error. */
        @Suppress("TooGenericExceptionCaught")
        private suspend fun print(action: suspend () -> Unit): Boolean =
            try {
                action()
                toasts.show(text("webshop_toast_print_ok"), ToastVariant.Success)
                true
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if ((e as? PosException.Coded)?.code == PRINTER_NOT_CONFIGURED) warnPrinterMissing() else report(e)
                false
            }

        private fun warnPrinterMissing() {
            toasts.show(text("webshop_toast_printer_missing"), ToastVariant.Warning)
        }

        /** Shows [error] as an error toast, keeps its text in [lastError] and answers it. */
        private fun report(error: Throwable): String {
            val lang = settings.language.value
            val shown = if (error is SaleRefusedException) errors.textRaw(error.raw, lang) else errors.text(error, lang)
            mutableError.value = shown
            toasts.show(shown, ToastVariant.Error, ERROR_TOAST_TTL_MS)
            return shown
        }

        private fun text(key: String): String = catalog.text(key, settings.language.value)

        private fun publish(list: List<WebshopOrderSummary>) {
            mutableOrders.value = list
            mutablePending.value = list.count(::isPending)
            mutableInProgress.value = list.count { it.hubriseStatus == STATUS_ACCEPTED || it.hubriseStatus == STATUS_IN_PREPARATION }
            mutableReady.value = list.count { it.hubriseStatus == STATUS_AWAITING_COLLECTION }
            mutableLoaded.value = true
        }

        private fun forget() {
            mutableOrders.value = emptyList()
            mutablePending.value = 0
            mutableInProgress.value = 0
            mutableReady.value = 0
            mutableLoaded.value = false
            mutableError.value = null
        }

        private fun takeBusy(id: Long): Boolean {
            while (true) {
                val now = mutableBusy.value
                if (id in now) return false
                if (mutableBusy.compareAndSet(now, now + id)) return true
            }
        }

        private fun releaseBusy(id: Long) {
            while (true) {
                val now = mutableBusy.value
                if (mutableBusy.compareAndSet(now, now - id)) return
            }
        }

        private companion object {
            const val STATUS_ACCEPTED = "accepted"
            const val STATUS_IN_PREPARATION = "in_preparation"
            const val STATUS_AWAITING_COLLECTION = "awaiting_collection"
            const val STATUS_IN_DELIVERY = "in_delivery"
            const val PRINTER_NOT_CONFIGURED = "printer_not_configured"

            const val FISCAL_PENDING = "pending"
            const val FISCAL_REFUND_PENDING = "refund_pending"
            const val FISCAL_CANCEL_PENDING = "cancel_pending"

            /** Preparation window in minutes: the restaurant's own webshop delivery gets its driver time. */
            const val OWN_DELIVERY_PREP_MINUTES = 40L
            const val DEFAULT_PREP_MINUTES = 30L

            /** Waits for a decision: not yet signed and new, or signed and cancelled by HubRise. */
            fun isPending(order: WebshopOrderSummary): Boolean {
                val fiscal = order.webshop?.fiscalState ?: FISCAL_PENDING
                if (fiscal == FISCAL_REFUND_PENDING || fiscal == FISCAL_CANCEL_PENDING) return true
                return fiscal == FISCAL_PENDING && (order.hubriseStatus == "new" || order.hubriseStatus == "received")
            }

            /** The own webshop delivers with its own driver, the aggregators handle their logistics. */
            fun defaultPrepMinutes(order: WebshopOrderSummary): Long {
                val ownWebshop = order.source.lowercase().contains("web")
                return if (ownWebshop && order.serviceType == "delivery") OWN_DELIVERY_PREP_MINUTES else DEFAULT_PREP_MINUTES
            }

            fun parseInstant(text: String): Instant? =
                try {
                    OffsetDateTime.parse(text).toInstant()
                } catch (e: DateTimeParseException) {
                    Timber.d(e, "Unreadable confirmed time")
                    null
                }
        }
    }
