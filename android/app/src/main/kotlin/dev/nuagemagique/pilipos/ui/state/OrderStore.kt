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
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import timber.log.Timber
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.DiscountInput
import uniffi.pos_ffi.DiscountResult
import uniffi.pos_ffi.HeldCourse
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.NextCourse
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TableInfo
import uniffi.pos_ffi.TablePlan
import uniffi.pos_ffi.TableTransferResult
import uniffi.pos_ffi.TakeawayTag
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong
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
 * A payment part taken by hand and not signed yet (UX audit P4). It stays with the order when the operator goes back to
 * it from the checkout. A part the terminal approved is the engine's, so it is never kept here.
 */
data class HeldPart(
    val method: PaymentMethod,
    val cents: Long,
    /** The change handed back for a cash part. */
    val changeCents: Long,
    /** A gift voucher part: the voucher it redeems, and its masked code. */
    val giftVoucherId: Long? = null,
    val codeMasked: String? = null,
    /** The cash handed over for a cash part. */
    val givenCents: Long = 0L,
    /** The cart units this part pays when the sale is split by dishes (UX audit P3); empty otherwise. */
    val units: List<DishUnits> = emptyList(),
)

/** Units of one cart line given to a part of a sale split by dishes. Never sent to the engine. */
data class DishUnits(
    val lineId: Long,
    val count: Int,
)

/** The parts kept for one order. */
data class HeldParts(
    val orderId: Long,
    val parts: List<HeldPart>,
)

/**
 * An unpaid online order shown at the checkout (UX audit O2): [method] is the one the checkout starts on. It is not the
 * engine's active order, and leaving it registers nothing.
 */
data class OnlineCollect(
    val method: PaymentMethod,
)

/** An add to the cart: the line it went to and the units it gave. [seq] grows with each one. */
data class LineAdd(
    val seq: Long,
    val lineId: Long,
    val itemId: Long,
    val units: Int,
)

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
        private val mutableCourseSettings = MutableStateFlow(CourseSettings(enabled = false, categoryCourses = emptyList()))
        private val mutableHeldCourses = MutableStateFlow<List<HeldCourse>>(emptyList())
        private val mutableFailures =
            MutableSharedFlow<Throwable>(extraBufferCapacity = BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private val mutableSlipFailures =
            MutableSharedFlow<Unit>(extraBufferCapacity = BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private val mutableDiscountWarnings =
            MutableSharedFlow<Unit>(extraBufferCapacity = BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private val categories = ConcurrentHashMap<Long, Long>()
        private val mutex = Mutex()
        private val mutableLastAdd = MutableStateFlow<LineAdd?>(null)
        private val mutableTakeawayTag = MutableStateFlow<TakeawayTag?>(null)
        private val mutableTableMoves = MutableStateFlow(0)
        private val mutableCovers = MutableStateFlow<Int?>(null)
        private val mutableTableZones = MutableStateFlow<Map<Int, String>>(emptyMap())
        private val mutableOnlineCollect = MutableStateFlow<OnlineCollect?>(null)
        private val mutableHeldParts = MutableStateFlow<HeldParts?>(null)
        private val addSeq = AtomicLong()

        // The open order on screen when the last lock came (NO_ORDER: none); taken once at the next login.
        private val lockedOrder = AtomicLong(NO_ORDER)

        // Bumped by resetLocal(). An operation reads it before it talks to the engine and publishes only while it is
        // unchanged, so a read that was in flight during a reset (lock, idle lock) cannot bring the old order back.
        private val generation = AtomicInteger()
        private val publishLock = Any()

        /** The order being taken; null when there is none. */
        val current: StateFlow<Order?> = mutableCurrent.asStateFlow()

        /**
         * The last add (grid, code pad, digit after an add, option and price dialogs): the beep and the 200 ms
         * flash of its cart line follow it. Null after a reset.
         */
        val lastAdd: StateFlow<LineAdd?> = mutableLastAdd.asStateFlow()

        /**
         * The number of the day and the customer of the current takeaway (`takeawayTag` of `order.ts`, UX audit T2);
         * null for a table, an online order or when the till gives none.
         */
        val takeawayTag: StateFlow<TakeawayTag?> = mutableTakeawayTag.asStateFlow()

        /**
         * The covers (guests) of the current table order (`covers` of `order.ts`, UX audit T5); null when none were set
         * and for an order that is not a table. Local data: never signed.
         */
        val covers: StateFlow<Int?> = mutableCovers.asStateFlow()

        /** Table number to zone letters (the plan, UX audit T1); empty on a plan without letters. */
        val tableZones: StateFlow<Map<Int, String>> = mutableTableZones.asStateFlow()

        /**
         * How many table moves were signed (a move and its undo each count): the tables screen reads the plan again
         * when it changes (`tableMoves` of `ui/src/stores/order.ts`).
         */
        val tableMoves: StateFlow<Int> = mutableTableMoves.asStateFlow()

        /** The online order being collected at the checkout (UX audit O2), or null for an ordinary order. */
        val onlineCollect: StateFlow<OnlineCollect?> = mutableOnlineCollect.asStateFlow()

        /** The payment parts taken by hand for an order, not signed yet (UX audit P4); null when there are none. */
        val heldParts: StateFlow<HeldParts?> = mutableHeldParts.asStateFlow()

        /** The lines of [current], as the engine lists them. */
        val lines: StateFlow<List<CartLine>> = mutableLines.asStateFlow()

        /** The open orders (the tables screen). */
        val openOrders: StateFlow<List<Order>> = mutableOpenOrders.asStateFlow()

        /**
         * Units ordered but not yet sent to the bar or kitchen (dine-in: they go out with the next registration;
         * takeaway: with the sale). A count of units, not an amount.
         */
        val unsentCount: StateFlow<Int> = mutableUnsentCount.asStateFlow()

        /** The courses setting (`ui/src/stores/order.ts`): off until a manager switches it on. */
        val courseSettings: StateFlow<CourseSettings> = mutableCourseSettings.asStateFlow()

        /** The courses of [current] that wait for "Envoyer la suite", lowest first; empty with courses off. */
        val heldCourses: StateFlow<List<HeldCourse>> = mutableHeldCourses.asStateFlow()

        /** Cart refusals (order paid meanwhile, product no longer available, session closed). */
        val failures: SharedFlow<Throwable> = mutableFailures.asSharedFlow()

        /** A bar or kitchen slip did not print: the order is signed, the slip must be told by hand. */
        val slipFailures: SharedFlow<Unit> = mutableSlipFailures.asSharedFlow()

        /**
         * A discount was recorded but the P that registers it could not be signed (FDM down, say): it goes with the
         * next registration or the sale, the screen shows a warning.
         */
        val discountWarnings: SharedFlow<Unit> = mutableDiscountWarnings.asSharedFlow()

        init {
            // Undispatched: the collector is subscribed when the constructor returns (events have no replay).
            scope.launch { gateway.ready.collect { ready -> if (ready) loadTablePlan() } }
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event ->
                    if (event is PosEvent.Resync) scope.launch { loadTablePlan() }
                    if (event is PosEvent.SessionLocked) {
                        // Before the reset: the order the same user may come back to after an idle lock (UX audit T6).
                        // An online order at its checkout (O2) was never the engine's active order: none to come back to.
                        val reopenable = mutableCurrent.value.takeIf { mutableOnlineCollect.value == null }
                        lockedOrder.set(reopenable?.takeIf { it.status == OrderStatus.OPEN }?.id ?: NO_ORDER)
                        resetLocal()
                    }
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
            if (publish(order, cart, seen)) {
                gateway.call { engine -> engine.setActiveOrder(order.id) }
                readCourses(order.id, seen)
                readTakeawayTag(order, seen)
                readCovers(order, seen)
            }
        }

        /**
         * Shows an unpaid online order at the checkout (UX audit O2): its order and lines are read, but it does not
         * become the engine's active order, so a lock or leaving the screen registers nothing.
         */
        suspend fun beginOnlineCollect(
            orderId: Long,
            method: PaymentMethod,
        ) = mutex.withLock {
            val seen = generation.get()
            val (order, cart) =
                gateway.call { engine -> engine.getOrder(orderId) to engine.listCartLines(orderId) }
            if (publish(order, cart, seen)) mutableOnlineCollect.value = OnlineCollect(method)
        }

        /** The parts kept for order [orderId]; none for another order. */
        fun heldPartsFor(orderId: Long): List<HeldPart> = mutableHeldParts.value?.takeIf { it.orderId == orderId }?.parts.orEmpty()

        /** Keeps [parts] (taken by hand, unsigned) for order [orderId]; none forgets them. */
        fun holdParts(
            orderId: Long,
            parts: List<HeldPart>,
        ) {
            mutableHeldParts.value = if (parts.isEmpty()) null else HeldParts(orderId, parts)
        }

        /** Loads an existing order and its lines (re-opening a table) and makes it the active order. */
        suspend fun loadOrder(orderId: Long) =
            mutex.withLock {
                val seen = generation.get()
                val (order, cart) =
                    gateway.call { engine -> engine.getOrder(orderId) to engine.listCartLines(orderId) }
                if (publish(order, cart, seen)) {
                    gateway.call { engine -> engine.setActiveOrder(orderId) }
                    readCourses(orderId, seen)
                    readTakeawayTag(order, seen)
                    readCovers(order, seen)
                }
            }

        /**
         * Puts (or, with null or empty values, clears) the name and the phone of the customer of the current takeaway;
         * both are replaced. The tag becomes the engine's answer.
         *
         * @return false when the engine refused (the refusal is on [failures])
         */
        suspend fun setTakeawayCustomer(
            name: String?,
            phone: String?,
        ): Boolean =
            reported {
                val order = requireCurrent()
                val tag = gateway.call { engine -> engine.setTakeawayCustomer(order.id, name, phone) }
                mutableTakeawayTag.value = tag
            }

        /**
         * Sets (1 to 30) or clears (null) the covers of the current table order; the engine's answer becomes the
         * state (`setCovers` of `order.ts`, UX audit T5).
         *
         * @return false when the engine refused (the refusal is on [failures])
         */
        suspend fun setCovers(count: Int?): Boolean =
            reported {
                val order = requireCurrent()
                mutableCovers.value = gateway.call { engine -> engine.setOrderCovers(order.id, count) }
            }

        /** How table [number] is written on screen: "A-19" in a zone, "19" without one; empty for no table. */
        fun tableLabel(number: Int?): String = if (number == null) "" else tableLabel(mutableTableZones.value[number], number)

        /** The compact form of the ticket lists: "A-19" in a zone, "T19" without one. */
        fun tableShortLabel(number: Int): String = mutableTableZones.value[number]?.let { "$it-$number" } ?: "T$number"

        /** Reads the plan from the engine; a till that does not answer keeps plain numbers (`load` of `tablePlan.ts`). */
        @Suppress("TooGenericExceptionCaught")
        suspend fun loadTablePlan() {
            try {
                applyTablePlan(gateway.call { engine -> engine.getTablePlan() })
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.d(e, "Table plan not loaded")
            }
        }

        /** Learns the zones from a plan the engine just returned (`apply` of `tablePlan.ts`). */
        fun applyTablePlan(plan: TablePlan) {
            mutableTableZones.value = plan.zones.flatMap { zone -> zone.numbers.map { it to zone.letter } }.toMap()
        }

        /** Learns the zones from a list of tables, each of which says its own (`learn` of `tablePlan.ts`). */
        fun learnTables(tables: List<TableInfo>) {
            val next = mutableTableZones.value.toMutableMap()
            for (table in tables) {
                val zone = table.zone
                if (inZone(zone) && zone != null) next[table.tableNumber] = zone else next.remove(table.tableNumber)
            }
            mutableTableZones.value = next
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
        ): Boolean = addProduct(item, quantity, unitPriceCents) != null

        /** Like [addItem], answering the add (its line and units), or null when the engine refused. */
        suspend fun addProduct(
            item: MenuItem,
            quantity: Int = 1,
            unitPriceCents: Long? = null,
        ): LineAdd? {
            var add: LineAdd? = null
            val ok = reported { add = addItemLocked(item, quantity, unitPriceCents) }
            return if (ok) add else null
        }

        /**
         * The digit after an add: [delta] more units on the line the add went to, as extra units of the same line
         * through the quantity command. The units already sent to the kitchen never change: the quantity stays
         * above them. A line removed meanwhile does nothing (false).
         *
         * @return false when the line is gone or the engine refused (the refusal is on [failures])
         */
        suspend fun addUnits(
            lineId: Long,
            delta: Int,
        ): Boolean {
            var done = false
            val ok =
                reported {
                    val line = mutableLines.value.firstOrNull { it.id == lineId } ?: return@reported
                    val sent = line.quantity - line.unsentQuantity.coerceAtLeast(0L).toInt()
                    val quantity = maxOf(line.quantity + delta, sent + 1)
                    if (quantity != line.quantity) {
                        gateway.call { engine -> engine.updateOrderLineQuantity(lineId, quantity) }
                        mutableCurrent.value?.let { refresh(it.id) }
                    }
                    noteAdd(lineId, line.menuItemId, quantity - line.quantity)
                    done = true
                }
            return ok && done
        }

        /**
         * Shows a line that another command added ([create] runs against the current order id, inside the store's
         * lock, so it must not call the store; used for a product with options). The item's category is
         * remembered. [quantity] is what was ordered (the units of the add on [lastAdd]); the engine's lines are the
         * truth.
         *
         * @return false when the engine refused (the refusal is on [failures])
         */
        suspend fun addLineWith(
            item: MenuItem,
            quantity: Int,
            create: suspend (orderId: Long) -> OrderLine,
        ): Boolean =
            reported {
                val order = requireCurrent()
                val line = create(order.id)
                categories[item.id] = item.categoryId
                refresh(order.id)
                noteAdd(line.id, item.id, quantity)
            }

        /**
         * Sells a gift voucher of [amountCents]: a fixed-quantity line of the system product. The engine
         * checks the setting, the amount and the order.
         *
         * @return false when the engine refused (the refusal is on [failures])
         */
        suspend fun addGiftVoucher(amountCents: Long): Boolean =
            reported {
                val order = requireCurrent()
                gateway.call { engine -> engine.addGiftVoucherLine(order.id, amountCents) }
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

        /**
         * Sets (or, with an empty text, clears) the kitchen note of a line. A note slip that did not print is
         * emitted on [slipFailures]; the lines are read again, so the engine's trimmed text is what the cart shows.
         *
         * @return false when the engine refused (the refusal is on [failures])
         */
        suspend fun setLineNote(
            lineId: Long,
            note: String,
        ): Boolean =
            reported {
                val outcome = gateway.call { engine -> engine.setLineNote(lineId, note.trim().ifEmpty { null }) }
                warnIfSlipFailed(outcome)
                mutableCurrent.value?.let { refresh(it.id) }
            }

        /**
         * The quick notes of this installation, the chips of the note dialog (`loadQuickNotes` of `order.ts`). A till
         * that does not answer shows none: typing a note still works.
         */
        @Suppress("TooGenericExceptionCaught", "SwallowedException")
        suspend fun quickNotes(): List<String> =
            try {
                gateway.call { engine -> engine.getQuickNotes() }.notes
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                emptyList()
            }

        /**
         * Changes the course of a line (refused once a slip carried it). @return false when the engine refused
         * (the refusal is on [failures])
         */
        suspend fun setLineCourse(
            lineId: Long,
            course: Int,
        ): Boolean =
            reported {
                gateway.call { engine -> engine.setLineCourse(lineId, course) }
                mutableCurrent.value?.let { refresh(it.id) }
            }

        /**
         * "Envoyer la suite": the engine signs what is unsent, releases the next held course and prints it. A
         * refusal (FDM down, nothing held) is thrown to the caller; a slip that did not print is on [slipFailures].
         */
        suspend fun sendNextCourse(): NextCourse =
            mutex.withLock {
                val order = requireCurrent()
                val next = gateway.call { engine -> engine.sendNextCourse(order.id) }
                warnIfSlipFailed(next.signed)
                warnIfSlipFailed(next.slip)
                refresh(order.id)
                next
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
        suspend fun preBill() = mutex.withLock { preBillUnlocked(requireCurrent().id) }

        /**
         * Prints the provisional bill of any open table order (UX audit T7: from the action sheet of the Tables
         * screen). The engine registers the unsent entries first; the order on screen, if it is that one, is read
         * again. A refusal is thrown.
         */
        suspend fun preBillOrder(orderId: Long) = mutex.withLock { preBillUnlocked(orderId) }

        private suspend fun preBillUnlocked(orderId: Long) {
            val outcome = gateway.call { engine -> engine.signPreBill(orderId) }
            warnIfSlipFailed(outcome)
            if (mutableCurrent.value?.id == orderId) refresh(orderId)
        }

        /**
         * Moves the open dine-in order [orderId] to the free table [toTable] (event P, cost center change). The
         * engine registers the unsent entries first; a slip that did not print is emitted on [slipFailures]; a
         * refusal is thrown. When the order is the current one it is read again (new table, nothing unsent).
         */
        suspend fun transferOrder(
            orderId: Long,
            toTable: Int,
        ): TableTransferResult =
            mutex.withLock {
                val result = gateway.call { engine -> engine.transferTable(orderId, toTable) }
                warnIfSlipFailed(result.kitchenSlip)
                warnIfSlipFailed(result.transferSlip)
                if (mutableCurrent.value?.id == orderId) refresh(orderId)
                mutableTableMoves.update { it + 1 }
                result
            }

        /** Moves the order on screen to the free table [toTable]; a refusal is thrown. */
        suspend fun transferTable(toTable: Int): TableTransferResult = transferOrder(requireCurrent().id, toTable)

        /**
         * Gives a discount on the current order, confirmed by the manager's [pin] (a percent, an amount or "Offert"
         * for the order, or one line for free), or by the PIN kept for this order when [pin] is null (UX audit P8:
         * the engine refuses with `pin_required` when it kept none). The order and its lines are read again (net
         * amounts, price changes, what the P registered); a refusal is thrown to the PIN dialog. The answer says
         * whether the kept PIN was used and for how long it still stands.
         */
        suspend fun applyDiscount(
            input: DiscountInput,
            userInsz: String,
            pin: String?,
        ): DiscountResult =
            mutex.withLock {
                val order = requireCurrent()
                val result = gateway.call { engine -> engine.applyDiscount(order.id, input, userInsz, pin) }
                discountDone(order.id, result)
                result
            }

        /**
         * Removes the order discount ([lineId] null) or the free line [lineId], confirmed by the manager's [pin] or,
         * when [pin] is null, by the PIN kept for this order. A removal never opens the grant.
         */
        suspend fun removeDiscount(
            lineId: Long?,
            userInsz: String,
            pin: String?,
        ): DiscountResult =
            mutex.withLock {
                val order = requireCurrent()
                val result = gateway.call { engine -> engine.removeDiscount(order.id, lineId, userInsz, pin) }
                discountDone(order.id, result)
                result
            }

        /** The wall clock the kept PIN hint counts on; a test replaces it. */
        @Volatile
        internal var clockMs: () -> Long = System::currentTimeMillis

        @Volatile
        private var pinGrantHint: PinGrantHint? = null

        /** Whether to try the next discount without the PIN: the engine said it keeps one for the order on screen. */
        fun keepsManagerPin(): Boolean = KeptPin.triesWithoutPin(pinGrantHint, mutableCurrent.value?.id, clockMs())

        /** The engine answered `pin_required`: the hint was stale. */
        fun forgetManagerPin() {
            pinGrantHint = null
        }

        private suspend fun discountDone(
            orderId: Long,
            result: DiscountResult,
        ) {
            pinGrantHint = KeptPin.hintAfter(orderId, result.pinGrantMs, clockMs())
            warnIfSlipFailed(result.kitchenSlip)
            if (result.registrationError != null) mutableDiscountWarnings.tryEmit(Unit)
            refresh(orderId)
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
         * The id of the order that was open on screen when the till last locked, once (T6, `takeLockedOrder` of
         * `auth.ts`): the login that finds the same user back reopens it. Null when there was none, or already taken.
         */
        fun takeLockedOrderId(): Long? = lockedOrder.getAndSet(NO_ORDER).takeIf { it != NO_ORDER }

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
                mutableLastAdd.value = null
                mutableTakeawayTag.value = null
                mutableCovers.value = null
                mutableOnlineCollect.value = null
                mutableHeldParts.value = null
                mutableUnsentCount.value = 0
                mutableHeldCourses.value = emptyList()
                categories.clear()
            }
        }

        /**
         * Reads the tag of a takeaway (its number of the day and customer); a table has none. A till that does not
         * answer simply shows no number.
         */
        @Suppress("TooGenericExceptionCaught", "SwallowedException")
        private suspend fun readTakeawayTag(
            order: Order,
            seen: Int,
        ) {
            val tag =
                if (order.orderType != OrderType.TAKEAWAY) {
                    null
                } else {
                    try {
                        gateway.call { engine -> engine.getTakeawayTag(order.id) }
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        null
                    }
                }
            synchronized(publishLock) {
                if (generation.get() == seen) mutableTakeawayTag.value = tag
            }
        }

        /** Reads the covers of a table order; a till that does not answer shows none (`loadCovers` of `order.ts`). */
        @Suppress("TooGenericExceptionCaught", "SwallowedException")
        private suspend fun readCovers(
            order: Order,
            seen: Int,
        ) {
            val stored =
                if (order.orderType != OrderType.DINE_IN) {
                    null
                } else {
                    try {
                        gateway.call { engine -> engine.getOrderCovers(order.id) }
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        null
                    }
                }
            synchronized(publishLock) {
                if (generation.get() == seen) mutableCovers.value = stored
            }
        }

        /** The category of a menu item added in this session, or null. */
        fun categoryOf(menuItemId: Long): Long? = categories[menuItemId]

        private fun noteAdd(
            lineId: Long,
            itemId: Long,
            units: Int,
        ): LineAdd = LineAdd(addSeq.incrementAndGet(), lineId, itemId, units).also { mutableLastAdd.value = it }

        private suspend fun addItemLocked(
            item: MenuItem,
            quantity: Int,
            unitPriceCents: Long?,
        ): LineAdd {
            val order = requireCurrent()
            // A line with options is never merged into (its options differ), nor is an open-price line, nor a line with
            // a price change (a till discount): the engine refuses a quantity change on it, so the product gets a new line.
            // Nor a line with a kitchen note: "1 with, 1 without" are two lines.
            val existing =
                if (item.openPrice) {
                    null
                } else {
                    mutableLines.value.firstOrNull {
                        it.menuItemId == item.id && it.modifiersJson.isNullOrEmpty() &&
                            it.priceChanges.isEmpty() && it.kitchenNote.isNullOrEmpty()
                    }
                }
            val lineId: Long
            if (existing != null) {
                val merged = existing.quantity + quantity
                gateway.call { engine -> engine.updateOrderLineQuantity(existing.id, merged) }
                lineId = existing.id
            } else {
                val input =
                    AddOrderLineInput(
                        orderId = order.id,
                        menuItemId = item.id,
                        quantity = quantity,
                        modifiersJson = null,
                        unitPriceCents = if (item.openPrice) unitPriceCents else null,
                    )
                lineId = gateway.call { engine -> engine.addOrderLine(input) }.id
            }
            categories[item.id] = item.categoryId
            refresh(order.id)
            return noteAdd(lineId, item.id, quantity)
        }

        private suspend fun leaveUnlocked(reason: LeaveReason) {
            val order = mutableCurrent.value ?: return
            if (mutableOnlineCollect.value != null) {
                // An online order at the checkout was never the engine's active order: nothing to register.
                clearUnlocked()
                return
            }
            val seen = generation.get()
            val outcome = gateway.call { engine -> engine.leaveOrder(order.id, reason) }
            warnIfSlipFailed(outcome)
            // The lines carry the new unsent quantities.
            publishLines(gateway.call { engine -> engine.listCartLines(order.id) }, seen)
            readCourses(order.id, seen)
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
            if (publish(order, cart, seen)) readCourses(orderId, seen)
        }

        /**
         * Reads the courses setting and, with courses on, the held courses of [orderId]. A till that does not
         * answer simply has courses off: the cart stays what the engine listed.
         */
        @Suppress("TooGenericExceptionCaught", "SwallowedException")
        private suspend fun readCourses(
            orderId: Long?,
            seen: Int,
        ) {
            val read =
                try {
                    val settings = gateway.call { engine -> engine.getCourseSettings() }
                    val held =
                        if (settings.enabled && orderId != null) {
                            gateway.call { engine -> engine.listHeldCourses(orderId) }
                        } else {
                            emptyList()
                        }
                    settings to held
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    CourseSettings(enabled = false, categoryCourses = emptyList()) to emptyList()
                }
            synchronized(publishLock) {
                if (generation.get() == seen) {
                    mutableCourseSettings.value = read.first
                    mutableHeldCourses.value = read.second
                }
            }
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

            /** No order was open at the last lock. */
            const val NO_ORDER = -1L
        }
    }
