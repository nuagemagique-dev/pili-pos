package dev.nuagemagique.pilipos.feature.order

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import timber.log.Timber
import uniffi.pos_ffi.AddWithModifiersInput
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.DiscountInput
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.VatSplitRow

/** How long the error toast of a refused send or bill stays, in milliseconds (6 s in `CartSidebar.vue`). */
private const val ERROR_TTL_MS: Long = 6_000L

/** How long the warning of a discount that could not be registered stays, in milliseconds (8 s in `order.ts`). */
private const val WARNING_TTL_MS: Long = 8_000L

/** The `status` the engine gives a table without an open order. */
private const val STATUS_FREE = "free"

/** The scope of an order discount's price change (`EVENT`); a free line's is `LINE`. */
private const val SCOPE_EVENT = "EVENT"
private const val SCOPE_LINE = "LINE"

/** A cart row with what a manager may do with the line: give it free (no change, no order discount) or remove its change. */
private fun cartRow(
    line: CartLine,
    amount: LineAmount?,
    manager: Boolean,
    orderDiscounted: Boolean,
    canNote: Boolean,
): CartRow =
    CartRow(
        line = line,
        amount = amount,
        canOffer = manager && !orderDiscounted && line.priceChanges.isEmpty(),
        canRemoveDiscount = manager && line.priceChanges.any { it.scope == SCOPE_LINE },
        canNote = canNote,
    )

/** How often the Z-report reminder is checked again, in milliseconds (5 minutes in `OrderView.vue`). */
internal const val Z_CHECK_MS: Long = 300_000L

/** What the model keeps of the engine reads: everything that is not already a store of its own. */
private data class Data(
    val categories: List<Category> = emptyList(),
    val selectedCategoryId: Long? = null,
    val items: List<MenuItem> = emptyList(),
    val loaded: Boolean = false,
    val zWarning: Boolean = false,
    val dialog: OrderDialog = OrderDialog.None,
    val amounts: Map<Long, LineAmount> = emptyMap(),
    val vatRows: List<VatSplitRow> = emptyList(),
    val sending: Boolean = false,
    val preBilling: Boolean = false,
    val transferring: Boolean = false,
)

/** How the screen speaks and what the signed-in user may do. */
private data class Look(
    val language: AppLanguage,
    val labelMode: ProductLabelMode,
    val canPay: Boolean,
    val signedIn: Boolean,
    val isManager: Boolean,
)

/**
 * State holder of the order screen, a port of the script of `ui/src/views/OrderView.vue` and of
 * `ui/src/stores/menu.ts`.
 *
 * The engine owns the catalog and the order: this reads the categories and the products of the selected category
 * (`get_categories`, `get_menu_items`) when the engine is ready and somebody is logged in, again when the language
 * changes, and on [PosEvent.CatalogUpdated] and [PosEvent.Resync] (UI-PD303). It makes the route's order the
 * current one once ready when another one (or none) is current, checks the Z-report reminder (`needs_z_report`) on
 * open and every [Z_CHECK_MS] while [runClock] runs, and turns the taps on products into cart changes through
 * [OrderStore][dev.nuagemagique.pilipos.ui.state.OrderStore]: a plain product is added, a free-price one asks for
 * its price first, and with HubRise on a product with options asks for the picks (`get_item_modifiers`,
 * `add_order_line_with_modifiers`).
 *
 * It also runs the cart column (`CartSidebar.vue`): quantity steps, removal, send to kitchen, the provisional
 * bill and "customer left without paying" with the manager PIN, all through the order store. It never navigates.
 * A refusal of any cart action is shown as an error toast here (the whole screen's, so the cart column does not
 * report its own).
 */
@Suppress("TooManyFunctions")
internal class MenuViewModel(
    private val entry: OrderEntryPoint,
    private val routeOrderId: Long,
) : ViewModel() {
    private val order = entry.order()
    private val session = entry.session()
    private val gateway = entry.gateway()
    private val settings = entry.settings()
    private val data = MutableStateFlow(Data())
    private val reload = MutableStateFlow(0)
    private val menuLock = Mutex()
    private var itemsJob: Job? = null

    // The engine is ready and somebody is logged in: the only state in which anything is read.
    private val active: StateFlow<Boolean> =
        combine(gateway.ready, session.currentUser) { ready, user -> ready && user != null }
            .distinctUntilChanged()
            .stateIn(viewModelScope, SharingStarted.Eagerly, gateway.ready.value && session.currentUser.value != null)

    private val look: StateFlow<Look> =
        combine(
            settings.language,
            settings.productLabel,
            session.canTakePayment,
            session.currentUser,
            session.isManager,
        ) { language, label, pay, user, manager -> Look(language, label.toMode(), pay, user != null, manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, currentLook())

    /** What the screen draws. */
    val state: StateFlow<OrderUiState> =
        combine(data, order.current, order.lines, look, order.unsentCount) { d, current, lines, l, unsent ->
            build(d, current, lines, l, unsent)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            build(data.value, order.current.value, order.lines.value, currentLook(), order.unsentCount.value),
        )

    init {
        viewModelScope.launch { active.collect { on -> if (!on) data.value = Data() } }
        viewModelScope.launch {
            combine(active, settings.language, reload) { on, language, _ -> if (on) language else null }
                .collectLatest { language -> if (language != null) loadMenu(language) }
        }
        viewModelScope.launch {
            entry.events().events.collect { event ->
                if (event is PosEvent.CatalogUpdated || event is PosEvent.Resync) reload.update { it + 1 }
            }
        }
        viewModelScope.launch {
            order.lines.collectLatest { lines ->
                val amounts = amountsOf(lines)
                val vatRows = vatRowsOf(lines)
                data.update { it.copy(amounts = amounts, vatRows = vatRows) }
            }
        }
    }

    private fun currentLook() =
        Look(
            settings.language.value,
            settings.productLabel.value.toMode(),
            session.canTakePayment.value,
            session.currentUser.value != null,
            session.isManager.value,
        )

    private fun build(
        d: Data,
        shared: Order?,
        sharedLines: List<CartLine>,
        l: Look,
        sharedUnsent: Int,
    ): OrderUiState {
        // The order store is shared by every screen: another order than the one of the route is not this screen's (PD205).
        val current = shared?.takeIf { it.id == routeOrderId }
        val lines = if (current != null) sharedLines else emptyList()
        val orderDiscounted = lines.any { line -> line.priceChanges.any { it.scope == SCOPE_EVENT } }
        val quantities = lines.groupingBy { it.menuItemId }.fold(0) { sum, line -> sum + line.quantity }
        return OrderUiState(
            signedIn = l.signedIn,
            menu = MenuState(d.categories, d.selectedCategoryId, d.items, quantities, l.language, l.labelMode, d.loaded),
            cart =
                OrderCartState(
                    order = current,
                    rows =
                        lines.map {
                            cartRow(
                                it,
                                d.amounts[it.id],
                                l.isManager && current?.status == OrderStatus.OPEN,
                                orderDiscounted,
                                current?.status == OrderStatus.OPEN,
                            )
                        },
                    language = l.language,
                    labelMode = l.labelMode,
                    canPay = l.canPay,
                    vatRows = if (current != null) d.vatRows else emptyList(),
                    unsentCount = if (current != null) sharedUnsent else 0,
                    sending = d.sending,
                    preBilling = d.preBilling,
                    canCloseUnpaid = l.isManager && current?.status == OrderStatus.OPEN && lines.isNotEmpty(),
                    canTransfer =
                        current?.orderType == OrderType.DINE_IN &&
                            current.status == OrderStatus.OPEN &&
                            current.tableNumber != null &&
                            lines.isNotEmpty() &&
                            !d.transferring,
                    canDiscount = l.isManager && current?.status == OrderStatus.OPEN && lines.isNotEmpty(),
                    orderDiscounted = orderDiscounted,
                ),
            zWarning = d.zWarning,
            dialog = d.dialog,
        )
    }

    // ── Opening ───────────────────────────────────────────────────

    /**
     * What the screen does while it is composed and only then: it makes the route's order the current one (once
     * ready, and again each time the engine becomes ready again), and it shows every refusal of a cart action as an
     * error toast. A screen left on the back stack does neither, so a refusal is shown once and a hidden screen
     * never takes the current order back.
     */
    suspend fun run() =
        coroutineScope {
            launch { order.failures.collect { entry.errors().show(it, settings.language.value) } }
            launch {
                order.discountWarnings.collect {
                    val language = settings.language.value
                    entry.toasts().show(
                        entry.catalog().text("discount_registration_failed", language),
                        ToastVariant.Warning,
                        WARNING_TTL_MS,
                    )
                }
            }
            active.collectLatest { on -> if (on) opened() }
        }

    /**
     * Checks the Z-report reminder every [Z_CHECK_MS] until cancelled. The screen runs it while it is composed
     * (`onMounted` and `onUnmounted` of `OrderView.vue`); the first check is made when the screen opens.
     */
    suspend fun runClock() {
        while (true) {
            delay(Z_CHECK_MS)
            if (active.value) checkZ()
        }
    }

    private suspend fun opened() {
        openRouteOrder()
        checkZ()
    }

    /** Makes the order of the route the current one: a different (or no) current order is replaced by loading it. */
    private suspend fun openRouteOrder() {
        if (order.current.value?.id == routeOrderId) return
        attempt { order.loadOrder(routeOrderId) }.onFailure { entry.errors().show(it, settings.language.value) }
    }

    /** `checkZReport`: the reminder is non-critical, a failure leaves it as it was. */
    private suspend fun checkZ() {
        attempt { gateway.call { engine -> engine.needsZReport() } }
            .onSuccess { needed -> data.update { it.copy(zWarning = needed) } }
            .onFailure { Timber.d(it, "Z-report reminder not checked") }
    }

    // ── Menu ──────────────────────────────────────────────────────

    /** Reads the categories and the products of the selected one (the first when none is selected or it is gone). */
    private suspend fun loadMenu(language: AppLanguage) =
        menuLock.withLock {
            // The selection the read starts from: a tab tapped after this point must not be undone by the result.
            val keep = data.value.selectedCategoryId
            attempt {
                val categories = gateway.call { engine -> engine.getCategories(language.code) }
                val selected = categories.firstOrNull { it.id == keep }?.id ?: categories.firstOrNull()?.id
                val items =
                    if (selected != null) {
                        gateway.call { engine -> engine.getMenuItems(selected, language.code) }
                    } else {
                        emptyList()
                    }
                Triple(categories, selected, items)
            }.fold(
                onSuccess = { (categories, selected, items) ->
                    data.update {
                        // A tab tapped while the read ran wins: its own job reads its products, so only the tabs are published.
                        if (it.selectedCategoryId == keep) {
                            it.copy(categories = categories, selectedCategoryId = selected, items = items, loaded = true)
                        } else {
                            it.copy(categories = categories, loaded = true)
                        }
                    }
                },
                // What was shown stays: the next language change, catalog event or resync reads it again.
                onFailure = { Timber.d(it, "Menu not loaded") },
            )
        }

    /** A tap on a category tab: it becomes the selected one and its products are read (`selectCategory`). */
    fun selectCategory(categoryId: Long) {
        data.update { it.copy(selectedCategoryId = categoryId) }
        itemsJob?.cancel()
        itemsJob =
            viewModelScope.launch {
                menuLock.withLock {
                    val language = settings.language.value
                    attempt { gateway.call { engine -> engine.getMenuItems(categoryId, language.code) } }.fold(
                        onSuccess = { items ->
                            data.update { if (it.selectedCategoryId == categoryId) it.copy(items = items) else it }
                        },
                        onFailure = { Timber.d(it, "Products not loaded") },
                    )
                }
            }
    }

    // ── Products ──────────────────────────────────────────────────

    /** A tap on a product button (`handleAddItem`). */
    fun addItem(item: MenuItem) {
        // Only the order of the route takes products: while another one is current the menu does nothing.
        if (!isRouteOrderCurrent()) return
        if (item.openPrice) {
            data.update { it.copy(dialog = OrderDialog.Price(item)) }
            return
        }
        // Every tap counts (OrderView.vue): OrderStore runs the cart actions one after the other, so taps queue up.
        viewModelScope.launch {
            if (entry.integrations().hubriseEnabled.value) askModifiers(item) else order.addItem(item)
        }
    }

    /** With HubRise on, a product with options asks for the picks; one without options is added plainly. */
    private suspend fun askModifiers(item: MenuItem) {
        val language = settings.language.value
        attempt { gateway.call { engine -> engine.getItemModifiers(item.id, language.code) } }.fold(
            onSuccess = { groups ->
                if (groups.isEmpty()) {
                    order.addItem(item)
                } else {
                    data.update { it.copy(dialog = OrderDialog.Modifiers(item, groups)) }
                }
            },
            onFailure = { entry.errors().show(it, language) },
        )
    }

    /** The price typed for the free-price product of the open price dialog (`handlePriceConfirmed`). */
    fun confirmPrice(priceCents: Long) {
        val item = (data.value.dialog as? OrderDialog.Price)?.item ?: return
        data.update { it.copy(dialog = OrderDialog.None) }
        if (!isRouteOrderCurrent()) return
        viewModelScope.launch { order.addItem(item, 1, priceCents) }
    }

    /** The picks of the options dialog: the engine prices the options and names the line, only the ids go up. */
    fun confirmModifiers(modifierIds: List<Long>) {
        val item = (data.value.dialog as? OrderDialog.Modifiers)?.item ?: return
        data.update { it.copy(dialog = OrderDialog.None) }
        if (!isRouteOrderCurrent()) return
        viewModelScope.launch {
            order.addLineWith(item, 1) { orderId ->
                gateway.call { engine ->
                    engine.addOrderLineWithModifiers(AddWithModifiersInput(orderId, item.id, 1, modifierIds))
                }
            }
        }
    }

    private fun isRouteOrderCurrent() = order.current.value?.id == routeOrderId

    /** Closes the open dialog without adding anything. */
    fun dismissDialog() {
        data.update { it.copy(dialog = OrderDialog.None) }
    }

    // ── Cart lines ────────────────────────────────────────────────

    /** The plus of a line (`increment`): one more. Ignored when the line is gone or another order is current. */
    fun incrementLine(lineId: Long) = changeQuantity(lineId, 1)

    /** The minus of a line (`decrement`): one less, never below 1 (removing a line is [removeLine]). */
    fun decrementLine(lineId: Long) = changeQuantity(lineId, -1)

    private fun changeQuantity(
        lineId: Long,
        step: Int,
    ) {
        val line = order.lines.value.firstOrNull { it.id == lineId }
        val quantity = (line?.quantity ?: 0) + step
        if (isRouteOrderCurrent() && line != null && quantity >= 1) {
            // A refusal is on OrderStore.failures, shown by run().
            viewModelScope.launch { order.updateQuantity(lineId, quantity) }
        }
    }

    /** The delete control of a line. */
    fun removeLine(lineId: Long) {
        if (!isRouteOrderCurrent()) return
        viewModelScope.launch { order.removeLine(lineId) }
    }

    /**
     * Sends what was ordered to the bar and kitchen without leaving the table (`sendToKitchen`, event P as when
     * leaving the order). Ignored while a send runs and when nothing is unsent.
     */
    fun sendToKitchen() {
        val count = order.unsentCount.value
        if (!isRouteOrderCurrent() || data.value.sending || count == 0) return
        data.update { it.copy(sending = true) }
        viewModelScope.launch {
            val language = settings.language.value
            attempt { order.leave(LeaveReason.SEND_TO_KITCHEN) }.fold(
                onSuccess = {
                    entry.toasts().show(entry.catalog().text("sent_to_kitchen", language, mapOf("n" to count)), ToastVariant.Success)
                },
                onFailure = { entry.errors().show(it, language, entry.catalog().text("order_register_failed", language), ERROR_TTL_MS) },
            )
            data.update { it.copy(sending = false) }
        }
    }

    /** Prints the provisional bill of a dine-in order with lines (`printPreBill`). */
    fun printPreBill() {
        val current = order.current.value
        if (!isRouteOrderCurrent() || current?.orderType != OrderType.DINE_IN || order.lines.value.isEmpty()) return
        if (data.value.preBilling) return
        data.update { it.copy(preBilling = true) }
        viewModelScope.launch {
            val language = settings.language.value
            attempt { order.preBill() }.fold(
                onSuccess = { entry.toasts().show(entry.catalog().text("pre_bill_printed", language), ToastVariant.Success) },
                onFailure = { entry.errors().show(it, language, entry.catalog().text("pre_bill_failed", language), ERROR_TTL_MS) },
            )
            data.update { it.copy(preBilling = false) }
        }
    }

    /**
     * Opens the table picker of "change table" (`openTablePicker`): the free tables of the plan, the current one
     * left out. A refusal to list them is shown as an error toast.
     */
    fun askTransfer() {
        val current = order.current.value
        val from = current?.tableNumber
        if (!isRouteOrderCurrent() || from == null || !state.value.cart.canTransfer) return
        viewModelScope.launch {
            val language = settings.language.value
            attempt { gateway.call { engine -> engine.listTables() } }.fold(
                onSuccess = { tables ->
                    val free = tables.filter { it.status == STATUS_FREE && it.tableNumber != from }.map { it.tableNumber }
                    data.update { it.copy(dialog = OrderDialog.TablePicker(from, free)) }
                },
                onFailure = { entry.errors().show(it, language) },
            )
        }
    }

    /** The table picked in the picker (`transferTo`): the order moves there, the toast says so or the refusal. */
    fun confirmTransfer(toTable: Int) {
        if (!isRouteOrderCurrent() || data.value.transferring) return
        data.update { it.copy(dialog = OrderDialog.None, transferring = true) }
        viewModelScope.launch {
            val language = settings.language.value
            attempt { order.transferTable(toTable) }.fold(
                onSuccess = {
                    entry.toasts().show(entry.catalog().text("transfer_done", language, mapOf("to" to toTable)), ToastVariant.Success)
                },
                onFailure = { entry.errors().show(it, language, null, ERROR_TTL_MS) },
            )
            data.update { it.copy(transferring = false) }
        }
    }

    /** Opens the manager PIN dialog of "customer left without paying", when the order may be closed that way. */
    fun askCloseUnpaid() {
        if (!state.value.cart.canCloseUnpaid) return
        val name = session.currentUser.value?.name.orEmpty()
        data.update { it.copy(dialog = OrderDialog.CloseUnpaid(name)) }
    }

    /**
     * The PIN typed in the dialog (`closeUnpaid`): the engine closes the order unpaid for the logged-in user, or
     * refuses and the error is thrown to the dialog, which shows it on the pad.
     */
    suspend fun closeUnpaid(pin: String) {
        val insz = session.currentUser.value?.insz ?: error("No user")
        order.closeUnpaid(insz, pin)
    }

    /** The text of a refusal of [closeUnpaid], for the line of the PIN pad. */
    fun describe(error: Throwable): String = entry.errors().text(error, settings.language.value)

    /** The sale was closed unpaid (`onUnpaidClosed`): the dialog closes and the toast says so; the screen leaves. */
    fun unpaidClosed() {
        data.update { it.copy(dialog = OrderDialog.None) }
        entry.toasts().show(entry.catalog().text("unpaid_closed", settings.language.value), ToastVariant.Success)
    }

    // ── Kitchen note ──────────────────────────────────────────────

    /** The "Note" button of a line (`noteLineId` of `CartSidebar.vue`): opens the note dialog with the line's note. */
    fun askNote(lineId: Long) {
        val line = order.lines.value.firstOrNull { it.id == lineId }
        if (!isRouteOrderCurrent() || line == null || order.current.value?.status != OrderStatus.OPEN) return
        // Sent: part of the line already went to the kitchen, so a changed note prints a note slip now.
        val sent = line.quantity - line.unsentQuantity > 0
        data.update { it.copy(dialog = OrderDialog.Note(lineId, line.kitchenNote.orEmpty(), sent)) }
    }

    /** The note typed in the dialog (`saveNote`); an empty text clears it. A refusal is shown by [run]. */
    fun saveNote(text: String) {
        val dialog = data.value.dialog as? OrderDialog.Note ?: return
        data.update { it.copy(dialog = OrderDialog.None) }
        if (!isRouteOrderCurrent()) return
        viewModelScope.launch { order.setLineNote(dialog.lineId, text) }
    }

    // ── Discount at the till ──────────────────────────────────────

    /**
     * Opens the discount dialog (`discountStep` "ask" of `CartSidebar.vue`): a discount on the order's total, or
     * "Offrir" on [lineId] (the line's amount). Ignored unless a manager may discount this order.
     */
    fun askDiscount(lineId: Long? = null) {
        if (!isRouteOrderCurrent() || !state.value.cart.canDiscount) return
        val base =
            if (lineId == null) {
                order.current.value?.totalCents ?: 0L
            } else {
                (data.value.amounts[lineId] as? LineAmount.Total)?.cents ?: 0L
            }
        data.update { it.copy(dialog = OrderDialog.Discount(lineId, base)) }
    }

    /** The input of the dialog: the manager PIN confirms it next. */
    fun chooseDiscount(input: DiscountInput) {
        if (data.value.dialog !is OrderDialog.Discount) return
        val name = session.currentUser.value?.name.orEmpty()
        data.update { it.copy(dialog = OrderDialog.DiscountPin(name, DiscountAction.Apply(input))) }
    }

    /** "Retirer la remise" on the order ([lineId] null) or on an offered line: the manager PIN confirms it. */
    fun askRemoveDiscount(lineId: Long? = null) {
        if (!isRouteOrderCurrent() || !state.value.cart.canDiscount) return
        val name = session.currentUser.value?.name.orEmpty()
        data.update { it.copy(dialog = OrderDialog.DiscountPin(name, DiscountAction.Remove(lineId))) }
    }

    /**
     * The PIN typed in the discount dialog: the engine applies or removes the discount for the logged-in manager,
     * or refuses and the error is thrown to the dialog, which shows it on the pad.
     */
    suspend fun confirmDiscountPin(pin: String) {
        val insz = session.currentUser.value?.insz ?: error("No user")
        when (val action = (data.value.dialog as? OrderDialog.DiscountPin)?.action ?: error("No discount to confirm")) {
            is DiscountAction.Apply -> order.applyDiscount(action.input, insz, pin)
            is DiscountAction.Remove -> order.removeDiscount(action.lineId, insz, pin)
        }
    }

    /** The discount was applied or removed (`onDiscountDone`): the dialog closes and the toast says so. */
    fun discountDone() {
        val removed = (data.value.dialog as? OrderDialog.DiscountPin)?.action is DiscountAction.Remove
        data.update { it.copy(dialog = OrderDialog.None) }
        entry.toasts().show(
            entry.catalog().text(if (removed) "discount_removed" else "discount_applied", settings.language.value),
            ToastVariant.Success,
        )
    }

    /** The VAT summary rows of the cart from the engine VAT helper; none when it cannot answer. */
    private suspend fun vatRowsOf(lines: List<CartLine>): List<VatSplitRow> =
        if (lines.isEmpty()) emptyList() else attempt { entry.amounts().vatRows(lines) }.getOrDefault(emptyList())

    /** The amount of each line from the engine VAT helper; a line it cannot answer for has no amount. */
    private suspend fun amountsOf(lines: List<CartLine>): Map<Long, LineAmount> {
        val amounts = entry.amounts()
        val out = LinkedHashMap<Long, LineAmount>()
        for (line in lines) {
            attempt { amounts.lineAmount(line) }.onSuccess { out[line.id] = it }
        }
        return out
    }
}

private fun ProductLabel.toMode(): ProductLabelMode = if (this == ProductLabel.CODE) ProductLabelMode.CODE else ProductLabelMode.NAME

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
