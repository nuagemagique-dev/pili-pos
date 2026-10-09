package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.SlipOutcome
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The engine refused to close the order unpaid. [raw] is the error text as the engine sent it (a code such as
 * `pin_locked|seconds=30`, possibly empty), for `ErrorTexts.describe`.
 */
class OrderRefusedException(
    val raw: String,
) : Exception(raw)

/**
 * The order being taken, shared by every screen (`ui/src/stores/order.ts`).
 *
 * The engine is the truth (PD207): after each change the order and its lines are read again instead of being
 * patched locally, so the total is [Order.totalCents] and the unsent quantities are the engine's. No amount is
 * computed here.
 *
 * Operations that change the order run one at a time. Cart actions ([addItem], [addLineWith], [removeLine],
 * [updateQuantity]) report a refusal on [failures] and answer false; the other operations throw it to the caller.
 */
@Suppress("TooManyFunctions")
@Singleton
class OrderStore
    @Inject
    constructor(
        private val gateway: EngineGateway,
        events: EngineEvents,
        @ApplicationScope scope: CoroutineScope,
    ) {
        private val mutableCurrent = MutableStateFlow<Order?>(null)
        private val mutableLines = MutableStateFlow<List<CartLine>>(emptyList())
        private val mutableOpenOrders = MutableStateFlow<List<Order>>(emptyList())
        private val mutableUnsentCount = MutableStateFlow(0)
        private val mutableFailures =
            MutableSharedFlow<Throwable>(extraBufferCapacity = BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private val mutableSlipFailures =
            MutableSharedFlow<Unit>(extraBufferCapacity = BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private val categories = ConcurrentHashMap<Long, Long>()
        private val mutex = Mutex()

        // Bumped by resetLocal(). An operation reads it before it talks to the engine and publishes only while it is
        // unchanged, so a read that was in flight during a reset (lock, idle lock) cannot bring the old order back.
        private val generation = AtomicInteger()
        private val publishLock = Any()

        /** The order being taken; null when there is none. */
        val current: StateFlow<Order?> = mutableCurrent.asStateFlow()

        /** The lines of [current], as the engine lists them. */
        val lines: StateFlow<List<CartLine>> = mutableLines.asStateFlow()

        /** The open orders (the tables screen). */
        val openOrders: StateFlow<List<Order>> = mutableOpenOrders.asStateFlow()

        /**
         * Units ordered but not yet sent to the bar or kitchen (dine-in: they go out with the next registration;
         * takeaway: with the sale). A count of units, not an amount.
         */
        val unsentCount: StateFlow<Int> = mutableUnsentCount.asStateFlow()

        /** Cart refusals (order paid meanwhile, product no longer available, session closed). */
        val failures: SharedFlow<Throwable> = mutableFailures.asSharedFlow()

        /** A bar or kitchen slip did not print: the order is signed, the slip must be told by hand. */
        val slipFailures: SharedFlow<Unit> = mutableSlipFailures.asSharedFlow()

        init {
            // Undispatched: the collector is subscribed when the constructor returns (events have no replay).
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event ->
                    if (event is PosEvent.SessionLocked) resetLocal()
                }
            }
        }

        /**
         * Starts an order: dine-in when [tableNumber] is given, else takeaway. A busy table answers its open
         * order, so its lines are listed. The order becomes the engine's active order.
         */
        suspend fun startOrder(
            tableNumber: Int?,
            userInsz: String,
        ) = mutex.withLock {
            val seen = generation.get()
            val type = if (tableNumber != null) OrderType.DINE_IN else OrderType.TAKEAWAY
            val order = gateway.call { engine -> engine.createOrder(CreateOrderInput(type, tableNumber, userInsz)) }
            val cart =
                if (tableNumber != null) {
                    gateway.call { engine -> engine.listCartLines(order.id) }
                } else {
                    emptyList()
                }
            if (publish(order, cart, seen)) gateway.call { engine -> engine.setActiveOrder(order.id) }
        }

        /** Loads an existing order and its lines (re-opening a table) and makes it the active order. */
        suspend fun loadOrder(orderId: Long) =
            mutex.withLock {
                val seen = generation.get()
                val (order, cart) =
                    gateway.call { engine -> engine.getOrder(orderId) to engine.listCartLines(orderId) }
                if (publish(order, cart, seen)) gateway.call { engine -> engine.setActiveOrder(orderId) }
            }

        /**
         * Adds [quantity] of [item]. A fixed-price item that already has a line without modifiers is merged into
         * it; an open-price item always gets its own line, priced [unitPriceCents] (passed through unchanged).
         *
         * @return false when the engine refused (the refusal is on [failures])
         */
        suspend fun addItem(
            item: MenuItem,
            quantity: Int = 1,
            unitPriceCents: Long? = null,
        ): Boolean = reported { addItemLocked(item, quantity, unitPriceCents) }

        /**
         * Shows a line that another command added ([create] runs against the current order id, inside the store's
         * lock, so it must not call the store; used for a product with options). The item's category is
         * remembered. [quantity] is what was ordered; the engine's lines are the truth, so it is only documentation
         * for the call site.
         *
         * @return false when the engine refused (the refusal is on [failures])
         */
        @Suppress("UNUSED_PARAMETER")
        suspend fun addLineWith(
            item: MenuItem,
            quantity: Int,
            create: suspend (orderId: Long) -> OrderLine,
        ): Boolean =
            reported {
                val order = requireCurrent()
                create(order.id)
                categories[item.id] = item.categoryId
                refresh(order.id)
            }

        /** Removes a line. @return false when the engine refused (the refusal is on [failures]) */
        suspend fun removeLine(lineId: Long): Boolean =
            reported {
                gateway.call { engine -> engine.removeOrderLine(lineId) }
                mutableCurrent.value?.let { refresh(it.id) }
            }

        /**
         * Sets the quantity of a line. Quantities never go below 1 here (UX guide section 4.3): removing a line is
         * [removeLine], so a quantity under 1 does nothing.
         *
         * @return false when ignored or when the engine refused (the refusal is on [failures])
         */
        suspend fun updateQuantity(
            lineId: Long,
            quantity: Int,
        ): Boolean {
            if (quantity < 1) return false
            return reported {
                gateway.call { engine -> engine.updateOrderLineQuantity(lineId, quantity) }
                mutableCurrent.value?.let { refresh(it.id) }
            }
        }

        /** Reads the open orders again. */
        suspend fun loadOpenOrders() {
            mutableOpenOrders.value = gateway.call { engine -> engine.listOpenOrders() }
        }

        /**
         * The operator leaves the order (another screen, another order, or "send to kitchen"). The engine
         * registers what was ordered for an open dine-in order and does nothing for a takeaway. A slip that did
         * not print is emitted on [slipFailures]; a refusal is thrown.
         */
        suspend fun leave(reason: LeaveReason) = mutex.withLock { leaveUnlocked(reason) }

        /**
         * [leave] when there is an order.
         *
         * @return true when there was nothing to leave or the leave succeeded; a refusal is thrown
         */
        suspend fun leaveIfActive(reason: LeaveReason): Boolean =
            mutex.withLock {
                if (mutableCurrent.value == null) return@withLock true
                leaveUnlocked(reason)
                true
            }

        /** Prints the provisional bill (PRO FORMA) of the current order; a refusal is thrown. */
        suspend fun preBill() =
            mutex.withLock {
                val order = requireCurrent()
                val outcome = gateway.call { engine -> engine.signPreBill(order.id) }
                warnIfSlipFailed(outcome)
                refresh(order.id)
            }

        /**
         * Closes the current order unpaid, confirmed by the manager's [pin]. When the engine does not report
         * success, [OrderRefusedException] carries its error text. On success the local order is cleared.
         */
        suspend fun closeUnpaid(
            userInsz: String,
            pin: String,
        ) = mutex.withLock {
            val order = requireCurrent()
            val result = gateway.call { engine -> engine.closeUnpaid(order.id, userInsz, pin) }
            if (!result.success) throw OrderRefusedException(result.error ?: "")
            warnIfSlipFailed(result.kitchenSlip)
            clearUnlocked()
        }

        /** Forgets the order here and tells the engine, best effort. */
        suspend fun clearOrder() = mutex.withLock { clearUnlocked() }

        /**
         * Forgets the local order state only: after a lock the engine has already registered and cleared its
         * active order.
         */
        fun resetLocal() {
            synchronized(publishLock) {
                // Every read started before this point is now stale: its publish is refused.
                generation.incrementAndGet()
                mutableCurrent.value = null
                mutableLines.value = emptyList()
                mutableUnsentCount.value = 0
                categories.clear()
            }
        }

        /** The category of a menu item added in this session, or null. */
        fun categoryOf(menuItemId: Long): Long? = categories[menuItemId]

        private suspend fun addItemLocked(
            item: MenuItem,
            quantity: Int,
            unitPriceCents: Long?,
        ) {
            val order = requireCurrent()
            // A line with options is never merged into (its options differ), nor is an open-price line.
            val existing =
                if (item.openPrice) {
                    null
                } else {
                    mutableLines.value.firstOrNull { it.menuItemId == item.id && it.modifiersJson.isNullOrEmpty() }
                }
            if (existing != null) {
                val merged = existing.quantity + quantity
                gateway.call { engine -> engine.updateOrderLineQuantity(existing.id, merged) }
            } else {
                val input =
                    AddOrderLineInput(
                        orderId = order.id,
                        menuItemId = item.id,
                        quantity = quantity,
                        modifiersJson = null,
                        unitPriceCents = if (item.openPrice) unitPriceCents else null,
                    )
                gateway.call { engine -> engine.addOrderLine(input) }
            }
            categories[item.id] = item.categoryId
            refresh(order.id)
        }

        private suspend fun leaveUnlocked(reason: LeaveReason) {
            val order = mutableCurrent.value ?: return
            val seen = generation.get()
            val outcome = gateway.call { engine -> engine.leaveOrder(order.id, reason) }
            warnIfSlipFailed(outcome)
            // The lines carry the new unsent quantities.
            publishLines(gateway.call { engine -> engine.listCartLines(order.id) }, seen)
        }

        @Suppress("TooGenericExceptionCaught", "SwallowedException")
        private suspend fun clearUnlocked() {
            resetLocal()
            // Best effort: the engine drops the active order on the next set_active_order anyway.
            try {
                gateway.call { engine -> engine.setActiveOrder(null) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // Ignored on purpose.
            }
        }

        /** Re-reads the order and its lines from the engine. */
        private suspend fun refresh(orderId: Long) {
            val seen = generation.get()
            val (order, cart) =
                gateway.call { engine -> engine.getOrder(orderId) to engine.listCartLines(orderId) }
            publish(order, cart, seen)
        }

        /** Shows [order] and [cart] unless [resetLocal] ran since [seen] was read; answers whether it did. */
        private fun publish(
            order: Order,
            cart: List<CartLine>,
            seen: Int,
        ): Boolean =
            synchronized(publishLock) {
                if (generation.get() != seen) return@synchronized false
                mutableCurrent.value = order
                setLines(cart)
                true
            }

        private fun publishLines(
            cart: List<CartLine>,
            seen: Int,
        ) {
            synchronized(publishLock) {
                if (generation.get() == seen) setLines(cart)
            }
        }

        private fun setLines(cart: List<CartLine>) {
            mutableLines.value = cart
            val units = cart.sumOf { it.unsentQuantity.coerceAtLeast(0L) }
            mutableUnsentCount.value = units.coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
        }

        private fun requireCurrent(): Order = mutableCurrent.value ?: error("No active order")

        private fun warnIfSlipFailed(outcome: SlipOutcome?) {
            if (outcome == SlipOutcome.FAILED) mutableSlipFailures.tryEmit(Unit)
        }

        /** Runs a cart action under the lock; a refusal is emitted on [failures] instead of being lost. */
        @Suppress("TooGenericExceptionCaught")
        private suspend fun reported(action: suspend () -> Unit): Boolean =
            try {
                mutex.withLock { action() }
                true
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                mutableFailures.tryEmit(e)
                false
            }

        private companion object {
            /** Reports held for a slow collector before the oldest is dropped. */
            const val BUFFER = 8
        }
    }
