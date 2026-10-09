package dev.nuagemagique.pilipos.feature.order

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
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
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.PosEvent

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
)

/** How the screen speaks and what the signed-in user may do. */
private data class Look(
    val language: AppLanguage,
    val labelMode: ProductLabelMode,
    val canPay: Boolean,
    val signedIn: Boolean,
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
 * It never navigates. A refusal of any cart action is shown as an error toast here (the whole screen's, so the
 * cart column of P5b-T12 does not report its own).
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
        combine(settings.language, settings.productLabel, session.canTakePayment, session.currentUser) { language, label, pay, user ->
            Look(language, label.toMode(), pay, user != null)
        }.stateIn(viewModelScope, SharingStarted.Eagerly, currentLook())

    /** What the screen draws. */
    val state: StateFlow<OrderUiState> =
        combine(data, order.current, order.lines, look) { d, current, lines, l -> build(d, current, lines, l) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, build(data.value, order.current.value, order.lines.value, currentLook()))

    init {
        viewModelScope.launch { active.collectLatest { on -> if (on) opened() else data.value = Data() } }
        viewModelScope.launch {
            combine(active, settings.language, reload) { on, language, _ -> if (on) language else null }
                .collectLatest { language -> if (language != null) loadMenu(language) }
        }
        viewModelScope.launch {
            entry.events().events.collect { event ->
                if (event is PosEvent.CatalogUpdated || event is PosEvent.Resync) reload.update { it + 1 }
            }
        }
        viewModelScope.launch { order.failures.collect { entry.errors().show(it, settings.language.value) } }
        viewModelScope.launch {
            order.lines.collectLatest { lines ->
                val amounts = amountsOf(lines)
                data.update { it.copy(amounts = amounts) }
            }
        }
    }

    private fun currentLook() =
        Look(
            settings.language.value,
            settings.productLabel.value.toMode(),
            session.canTakePayment.value,
            session.currentUser.value != null,
        )

    private fun build(
        d: Data,
        current: Order?,
        lines: List<CartLine>,
        l: Look,
    ): OrderUiState {
        val quantities = lines.groupingBy { it.menuItemId }.fold(0) { sum, line -> sum + line.quantity }
        return OrderUiState(
            signedIn = l.signedIn,
            menu = MenuState(d.categories, d.selectedCategoryId, d.items, quantities, l.language, l.labelMode, d.loaded),
            cart = OrderCartState(current, lines.map { CartRow(it, d.amounts[it.id]) }, l.language, l.labelMode, l.canPay),
            zWarning = d.zWarning,
            dialog = d.dialog,
        )
    }

    // ── Opening ───────────────────────────────────────────────────

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
        viewModelScope.launch { order.addItem(item, 1, priceCents) }
    }

    /** The picks of the options dialog: the engine prices the options and names the line, only the ids go up. */
    fun confirmModifiers(modifierIds: List<Long>) {
        val item = (data.value.dialog as? OrderDialog.Modifiers)?.item ?: return
        data.update { it.copy(dialog = OrderDialog.None) }
        viewModelScope.launch {
            order.addLineWith(item, 1) { orderId ->
                gateway.call { engine ->
                    engine.addOrderLineWithModifiers(AddWithModifiersInput(orderId, item.id, 1, modifierIds))
                }
            }
        }
    }

    /** Closes the open dialog without adding anything. */
    fun dismissDialog() {
        data.update { it.copy(dialog = OrderDialog.None) }
    }

    // ── Cart lines ────────────────────────────────────────────────

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
