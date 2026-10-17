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
import uniffi.pos_ffi.AllergenEntry
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosEvent
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
    val allergenMode: Boolean = false,
    val allergenLegend: List<AllergenEntry> = emptyList(),
    val sendingNext: Boolean = false,
    val giftVouchersEnabled: Boolean = false,
)

/** What the cart shows of the courses: the switch and the next held course. */
private data class Courses(
    val enabled: Boolean = false,
    val nextHeld: Int? = null,
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

    private val courses: StateFlow<Courses> =
        combine(order.courseSettings, order.heldCourses) { settings, held ->
            Courses(settings.enabled, if (settings.enabled) held.minOfOrNull { it.course } else null)
        }.stateIn(viewModelScope, SharingStarted.Eagerly, currentCourses())

    // The unsent count and the courses travel together, so that the state combines five flows.
    private val unsentAndCourses: StateFlow<Pair<Int, Courses>> =
        combine(order.unsentCount, courses) { unsent, c -> unsent to c }
            .stateIn(viewModelScope, SharingStarted.Eagerly, order.unsentCount.value to courses.value)

    /** What the screen draws. */
    val state: StateFlow<OrderUiState> =
        combine(data, order.current, order.lines, look, unsentAndCourses) { d, current, lines, l, extra ->
            build(d, current, lines, l, extra.first, extra.second)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            build(
                data.value,
                order.current.value,
                order.lines.value,
                currentLook(),
                unsentAndCourses.value.first,
                unsentAndCourses.value.second,
            ),
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
        viewModelScope.launch { active.collect { on -> if (on) loadGiftVoucherSetting() } }
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

    private fun currentCourses(): Courses {
        val settings = order.courseSettings.value
        return Courses(settings.enabled, if (settings.enabled) order.heldCourses.value.minOfOrNull { it.course } else null)
    }

    @Suppress("LongParameterList")
    private fun build(
        d: Data,
        shared: Order?,
        sharedLines: List<CartLine>,
        l: Look,
        sharedUnsent: Int,
        c: Courses,
    ): OrderUiState {
        // The order store is shared by every screen: another order than the one of the route is not this screen's (PD205).
        val current = shared?.takeIf { it.id == routeOrderId }
        val lines = if (current != null) sharedLines else emptyList()
        val quantities = lines.groupingBy { it.menuItemId }.fold(0) { sum, line -> sum + line.quantity }
        return OrderUiState(
            signedIn = l.signedIn,
            menu =
                MenuState(
                    d.categories, d.selectedCategoryId, d.items, quantities, l.language, l.labelMode, d.loaded,
                    d.allergenMode, d.allergenLegend, d.giftVouchersEnabled,
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
                    coursesEnabled = current != null && c.enabled,
                    nextHeldCourse = if (current != null) c.nextHeld else null,
                    sendingNext = d.sendingNext,
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
        when {
            // Allergen mode: a tap shows the product's allergen sheet instead of adding it.
            data.value.allergenMode -> openAllergenSheet(item)
            // Only the order of the route takes products: while another one is current the menu does nothing.
            !isRouteOrderCurrent() -> Unit
            item.openPrice -> data.update { it.copy(dialog = OrderDialog.Price(item)) }
            // Every tap counts (OrderView.vue): OrderStore runs the cart actions one after the other, so taps queue up.
            else ->
                viewModelScope.launch {
                    if (entry.integrations().hubriseEnabled.value) askModifiers(item) else order.addItem(item)
                }
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

    // ── Gift vouchers ─────────────────────────────────────────────

    /** Reads the gift voucher switch (`load` of `giftVouchers.ts`); a till that does not answer has vouchers off. */
    private suspend fun loadGiftVoucherSetting() {
        val enabled = attempt { gateway.call { engine -> engine.getGiftVoucherSettings() } }.map { it.enabled }.getOrDefault(false)
        data.update { it.copy(giftVouchersEnabled = enabled) }
    }

    /** The "Bon cadeau" button: asks for the amount of the voucher. */
    fun askGiftVoucher() {
        if (!isRouteOrderCurrent() || !data.value.giftVouchersEnabled) return
        data.update { it.copy(dialog = OrderDialog.SellVoucher) }
    }

    /** The amount typed for the voucher (`handleVoucherAmount`): a refusal is on OrderStore.failures, shown by run(). */
    fun confirmGiftVoucher(amountCents: Long) {
        if (data.value.dialog != OrderDialog.SellVoucher) return
        data.update { it.copy(dialog = OrderDialog.None) }
        if (!isRouteOrderCurrent()) return
        viewModelScope.launch { order.addGiftVoucher(amountCents) }
    }

    // ── Allergens ─────────────────────────────────────────────────

    /** The "Allergènes" button: turns the mode on or off; the legend is read the first time (`toggleMode`). */
    fun toggleAllergenMode() {
        val on = !data.value.allergenMode
        data.update { it.copy(allergenMode = on) }
        if (on && data.value.allergenLegend.isEmpty()) {
            viewModelScope.launch {
                attempt { gateway.call { engine -> engine.listAllergens() } }.fold(
                    onSuccess = { legend -> data.update { it.copy(allergenLegend = legend) } },
                    onFailure = { Timber.d(it, "Allergen legend not loaded") },
                )
            }
        }
    }

    /** Opens the allergen sheet of [item]: its sheet in the UI language, and in French when that differs (`openSheet`). */
    private fun openAllergenSheet(item: MenuItem) {
        data.update { it.copy(dialog = OrderDialog.AllergenSheet(item)) }
        viewModelScope.launch {
            val language = settings.language.value
            attempt {
                val own = gateway.call { engine -> engine.getAllergenSheet(item.id, language.code) }
                val fr =
                    if (language == AppLanguage.FR) null else gateway.call { engine -> engine.getAllergenSheet(item.id, "fr") }
                own.products.firstOrNull() to fr?.products?.firstOrNull()
            }.fold(
                onSuccess = { (own, fr) -> updateSheet(item) { it.copy(sheet = own, sheetFr = fr, error = null) } },
                onFailure = { error -> updateSheet(item) { it.copy(error = entry.errors().text(error, language)) } },
            )
        }
    }

    /** Prints the open product's sheet, or the whole menu's ([all]); the printer speaks fr, nl, en and de (zh prints in French). */
    fun printAllergenSheet(all: Boolean) {
        val open = data.value.dialog as? OrderDialog.AllergenSheet ?: return
        if (open.busy) return
        updateSheet(open.item) { it.copy(busy = true, error = null, printResult = null) }
        viewModelScope.launch {
            val language = settings.language.value
            val lang = if (language == AppLanguage.ZH) "fr" else language.code
            attempt { gateway.call { engine -> engine.printAllergenSheet(if (all) null else open.item.id, lang) } }.fold(
                onSuccess = { result -> updateSheet(open.item) { it.copy(busy = false, printResult = result) } },
                onFailure = { error -> updateSheet(open.item) { it.copy(busy = false, error = entry.errors().text(error, language)) } },
            )
        }
    }

    /** Changes the open allergen sheet of [item] (a sheet closed or replaced meanwhile is left alone). */
    private fun updateSheet(
        item: MenuItem,
        change: (OrderDialog.AllergenSheet) -> OrderDialog.AllergenSheet,
    ) {
        data.update {
            val open = it.dialog as? OrderDialog.AllergenSheet
            if (open != null && open.item.id == item.id) it.copy(dialog = change(open)) else it
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

    // ── Courses ───────────────────────────────────────────────────

    /** The course chip of a line: opens the picker, unless a slip already carried the line (its chip is locked). */
    fun openCoursePicker(lineId: Long) {
        val line = order.lines.value.firstOrNull { it.id == lineId } ?: return
        if (!isRouteOrderCurrent() || line.kitchenState == "printed") return
        data.update { it.copy(dialog = OrderDialog.CoursePicker(lineId, line.course)) }
    }

    /** A course picked in the picker (`pickCourse`): the dialog closes and a different course is stored. */
    fun pickCourse(course: Int) {
        val open = data.value.dialog as? OrderDialog.CoursePicker ?: return
        data.update { it.copy(dialog = OrderDialog.None) }
        if (!isRouteOrderCurrent() || course == open.current) return
        // A refusal (course_already_sent) is on OrderStore.failures, shown by run().
        viewModelScope.launch { order.setLineCourse(open.lineId, course) }
    }

    /**
     * "Envoyer la suite" (`sendNextCourse`): the engine signs what is unsent, releases the next held course and
     * prints it. Ignored while it runs and when no course is held.
     */
    fun sendNextCourse() {
        if (!isRouteOrderCurrent() || data.value.sendingNext || courses.value.nextHeld == null) return
        data.update { it.copy(sendingNext = true) }
        viewModelScope.launch {
            val language = settings.language.value
            val catalog = entry.catalog()
            attempt { order.sendNextCourse() }.fold(
                onSuccess = { next ->
                    val course = next.course
                    val text =
                        if (course == null) {
                            catalog.text("course_resent", language)
                        } else {
                            catalog.text("course_sent", language, mapOf("course" to catalog.text("course_$course", language)))
                        }
                    entry.toasts().show(text, ToastVariant.Success)
                },
                onFailure = { entry.errors().show(it, language, null, ERROR_TTL_MS) },
            )
            data.update { it.copy(sendingNext = false) }
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
