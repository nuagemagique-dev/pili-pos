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
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.SoldOutItem
import uniffi.pos_ffi.VatSplitRow

/** How long the error toast of a refused send or bill stays, in milliseconds (6 s in `CartSidebar.vue`). */
private const val ERROR_TTL_MS: Long = 6_000L

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
    val soldOutMode: Boolean = false,
    val soldOut: List<SoldOutItem> = emptyList(),
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
            // The sold-out list is read with the menu, for a user who may change it only.
            combine(active, settings.language, reload, session.canTakePayment) { on, language, _, pay ->
                if (on && pay) language else null
            }.distinctUntilChanged().collectLatest { language -> if (language != null) loadSoldOut(language) }
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
        val quantities = lines.groupingBy { it.menuItemId }.fold(0) { sum, line -> sum + line.quantity }
        return OrderUiState(
            signedIn = l.signedIn,
            menu =
                MenuState(
                    categories = d.categories,
                    selectedCategoryId = d.selectedCategoryId,
                    items = d.items,
                    cartQuantities = quantities,
                    language = l.language,
                    labelMode = l.labelMode,
                    loaded = d.loaded,
                    canSoldOut = l.canPay,
                    soldOutMode = d.soldOutMode && l.canPay,
                    soldOut = if (l.canPay) d.soldOut else emptyList(),
                ),
            cart =
                OrderCartState(
                    order = current,
                    rows = lines.map { CartRow(it, d.amounts[it.id]) },
                    language = l.language,
                    labelMode = l.labelMode,
                    canPay = l.canPay,
                    vatRows = if (current != null) d.vatRows else emptyList(),
                    unsentCount = if (current != null) sharedUnsent else 0,
                    sending = d.sending,
                    preBilling = d.preBilling,
                    canCloseUnpaid = l.isManager && current?.status == OrderStatus.OPEN && lines.isNotEmpty(),
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

    /** `loadSoldOut`: the products sold out now, for the cashier and the manager (the engine refuses the others). */
    private suspend fun loadSoldOut(language: AppLanguage) {
        attempt { gateway.call { engine -> engine.listSoldOut(language.code) } }.fold(
            onSuccess = { list -> data.update { it.copy(soldOut = list) } },
            onFailure = { entry.errors().show(it, language) },
        )
    }

    /** A tap on a category tab: it becomes the selected one and its products are read (`selectCategory`). */
    fun selectCategory(categoryId: Long) {
        data.update { it.copy(selectedCategoryId = categoryId) }
        refreshItems(categoryId)
    }

    // ── Products ──────────────────────────────────────────────────

    /** A tap on a product button (`handleAddItem`). */
    fun addItem(item: MenuItem) {
        // Only the order of the route takes products: while another one is current the menu does nothing.
        if (soldOutTap(item) || !isRouteOrderCurrent()) return
        if (item.openPrice) {
            data.update { it.copy(dialog = OrderDialog.Price(item)) }
            return
        }
        // Every tap counts (OrderView.vue): OrderStore runs the cart actions one after the other, so taps queue up.
        viewModelScope.launch {
            if (entry.integrations().hubriseEnabled.value) askModifiers(item) else order.addItem(item)
        }
    }

    /**
     * The sold-out side of a tap (true when it was handled). In the "Épuisé" mode (cashier and manager) a tap opens the
     * availability dialog instead of adding the product. Out of the mode a sold-out product shows why it cannot be
     * added; the grid is read again in case the end of the business day passed since it was.
     */
    private fun soldOutTap(item: MenuItem): Boolean =
        when {
            data.value.soldOutMode && look.value.canPay -> {
                data.update { it.copy(dialog = OrderDialog.SoldOut(item)) }
                true
            }
            item.soldOut -> {
                entry.toasts().show(entry.catalog().text("err_product_sold_out", settings.language.value), ToastVariant.Error)
                selectedCategoryId()?.let { refreshItems(it) }
                true
            }
            else -> false
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

    private fun selectedCategoryId(): Long? = data.value.selectedCategoryId

    /** Reads the products of [categoryId] again (the sold-out flags changed). */
    private fun refreshItems(categoryId: Long) {
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

    // ── Sold out (Épuisé) ─────────────────────────────────────────

    /** The "Épuisé" button: the mode is turned on or off (cashier and manager only). */
    fun toggleSoldOutMode() {
        if (!look.value.canPay) return
        data.update { it.copy(soldOutMode = !it.soldOutMode) }
    }

    /** "Produits épuisés (n)": the list opens over the screen, read again. */
    fun openSoldOutList() {
        if (!look.value.canPay) return
        data.update { it.copy(dialog = OrderDialog.SoldOutList) }
        viewModelScope.launch { loadSoldOut(settings.language.value) }
    }

    /**
     * The choice of the availability dialog, or the re-enable of the list: the engine marks the product (or clears it),
     * the list and the grid are read again. The dialog of a product closes on success and stays on a refusal, which is
     * shown as a toast; the list stays open.
     */
    fun setAvailability(
        menuItemId: Long,
        choice: SoldOutChoice,
    ) {
        viewModelScope.launch {
            val language = settings.language.value
            attempt {
                gateway.call { engine ->
                    when (choice) {
                        SoldOutChoice.DAY -> engine.setSoldOut(menuItemId, true, language.code)
                        SoldOutChoice.UNTIL -> engine.setSoldOut(menuItemId, false, language.code)
                        SoldOutChoice.AVAILABLE -> engine.clearSoldOut(menuItemId, language.code)
                    }
                }
            }.fold(
                onSuccess = { list ->
                    data.update {
                        it.copy(soldOut = list, dialog = if (it.dialog is OrderDialog.SoldOut) OrderDialog.None else it.dialog)
                    }
                    selectedCategoryId()?.let { refreshItems(it) }
                },
                onFailure = { entry.errors().show(it, language) },
            )
        }
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
