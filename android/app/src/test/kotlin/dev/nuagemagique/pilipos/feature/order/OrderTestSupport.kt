@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.format.CartAmounts
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import io.mockk.coEvery
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import uniffi.pos_ffi.CallbackEvent
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.CartPriceChange
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Modifier
import uniffi.pos_ffi.ModifierGroup
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import uniffi.pos_ffi.VatCode
import java.io.File

internal const val ADMIN_INSZ = "85010100123"

/** The order the route of the tests points at. */
internal const val ORDER_ID = 7L

internal val admin = User(insz = ADMIN_INSZ, name = "Alice", role = UserRole.ADMIN, active = true, hasPin = true)

internal fun category(
    id: Long,
    name: String,
    nameFr: String? = null,
    nameZh: String? = null,
    color: String? = null,
): Category = Category(id = id, name = name, nameFr = nameFr, nameZh = nameZh, sortOrder = id.toInt(), color = color, active = true)

@Suppress("LongParameterList")
internal fun menuItem(
    id: Long,
    categoryId: Long,
    name: String,
    priceCents: Long = 500,
    openPrice: Boolean = false,
    code: String? = null,
    nameFr: String? = null,
    nameZh: String? = null,
): MenuItem =
    MenuItem(
        id = id,
        categoryId = categoryId,
        name = name,
        nameFr = nameFr,
        nameZh = nameZh,
        code = code,
        priceCents = priceCents,
        vatCode = VatCode.B,
        vatCodeTakeaway = VatCode.C,
        kitchenPrinter = true,
        openPrice = openPrice,
        sortOrder = id.toInt(),
        active = true,
    )

@Suppress("LongParameterList")
internal fun cartLine(
    id: Long,
    menuItemId: Long,
    name: String,
    quantity: Int = 1,
    unitPriceCents: Long = 500,
    nameZh: String? = null,
    code: String? = null,
    modifiersJson: String? = null,
    unsent: Long = 0,
    discountCents: Long = 0,
    priceChanges: List<CartPriceChange> = emptyList(),
    kitchenNote: String? = null,
): CartLine =
    CartLine(
        id = id,
        orderId = ORDER_ID,
        menuItemId = menuItemId,
        quantity = quantity,
        unitPriceCents = unitPriceCents,
        vatCode = VatCode.B,
        name = name,
        nameFr = null,
        nameZh = nameZh,
        code = code,
        modifiersJson = modifiersJson,
        createdAt = "2026-10-09 10:00:00",
        departmentId = "d",
        departmentName = "Dept",
        unsentQuantity = unsent,
        discountCents = discountCents,
        priceChanges = priceChanges,
        kitchenNote = kitchenNote,
    )

internal fun orderLine(menuItemId: Long): OrderLine =
    OrderLine(
        id = 99,
        orderId = ORDER_ID,
        menuItemId = menuItemId,
        quantity = 1,
        unitPriceCents = 500,
        vatCode = VatCode.B,
        name = "item$menuItemId",
        modifiersJson = "[]",
        createdAt = "2026-10-09T10:00:00",
        departmentId = "d",
        departmentName = "Dept",
    )

internal fun order(
    id: Long = ORDER_ID,
    table: Int? = 4,
    totalCents: Long = 0,
): Order =
    Order(
        id = id,
        tableNumber = table,
        orderType = if (table != null) OrderType.DINE_IN else OrderType.TAKEAWAY,
        status = OrderStatus.OPEN,
        userInsz = ADMIN_INSZ,
        createdAt = "2026-10-09T10:00:00",
        closedAt = null,
        subtotalCents = totalCents,
        totalCents = totalCents,
    )

internal fun modifier(
    id: Long,
    name: String,
    priceCents: Long = 0,
): Modifier = Modifier(id = id, name = name, priceCents = priceCents)

internal fun group(
    id: Long,
    name: String,
    min: Long,
    max: Long?,
    vararg options: Modifier,
): ModifierGroup = ModifierGroup(id = id, name = name, min = min, max = max, options = options.toList())

internal fun posSettings(): PosSettings =
    PosSettings(
        trainingMode = false,
        printerAddress = null,
        fdmUrl = "http://fdm",
        posId = "POS1",
        terminalId = "T1",
        restaurantName = "Chez Pili",
        language = "fr",
        configProblems = emptyList(),
        posSwVersion = "pili-pos-android/1.2.3",
    )

internal fun catalogUpdated(): PosEvent =
    PosEvent.CatalogUpdated(
        CallbackEvent(
            id = "1",
            eventType = "catalog",
            resourceType = "catalog",
            previousState = "",
            newState = "",
            extra = "",
            createdAt = "2026-10-09T10:00:00",
        ),
    )

/** An [AppNavigator] that records what it was asked. */
internal class RecordingNavigator : AppNavigator {
    val calls: MutableList<String> = mutableListOf()

    override fun navigate(route: String) {
        calls += "navigate:$route"
    }

    override fun back() {
        calls += "back"
    }

    override fun toLogin() {
        calls += "toLogin"
    }

    override fun replaceAll(route: String) {
        calls += "replaceAll:$route"
    }
}

/** The order entry point over the given real stores. */
@Suppress("LongParameterList")
internal class TestOrderEntry(
    private val session: SessionStore,
    private val order: OrderStore,
    private val gateway: EngineGateway,
    private val events: EngineEvents,
    private val errors: ErrorPresenter,
    private val settings: PosSettingsStore,
    private val integrations: IntegrationsStore,
    private val amounts: CartAmounts,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
) : OrderEntryPoint {
    override fun session(): SessionStore = session

    override fun order(): OrderStore = order

    override fun gateway(): EngineGateway = gateway

    override fun events(): EngineEvents = events

    override fun errors(): ErrorPresenter = errors

    override fun settings(): PosSettingsStore = settings

    override fun integrations(): IntegrationsStore = integrations

    override fun amounts(): CartAmounts = amounts

    override fun toasts(): ToastStore = toasts

    override fun catalog(): MessageCatalog = catalog
}

/** What the stubbed engine holds: the catalog, the cart and the switches a test turns. */
internal class OrderWorld(
    var categories: List<Category> = listOf(category(1, "Sushi"), category(2, "Boissons")),
    var items: Map<Long, List<MenuItem>> =
        mapOf(
            1L to listOf(menuItem(10, 1, "Maki saumon", 650), menuItem(11, 1, "Nigiri thon", 480)),
            2L to listOf(menuItem(20, 2, "Cola", 300)),
        ),
    var cart: List<CartLine> = emptyList(),
    var needsZ: Boolean = false,
    var hubrise: Boolean = false,
)

/** A started order view model with the real stores it drives, built on one engine fixture. */
@Suppress("LongParameterList")
internal class OrderRig(
    val f: EngineFixture,
    val world: OrderWorld,
    val model: MenuViewModel,
    val session: SessionStore,
    val order: OrderStore,
    val settings: PosSettingsStore,
    val integrations: IntegrationsStore,
    val toasts: ToastStore,
    val entry: OrderEntryPoint,
) {
    val state get() = model.state.value

    fun lastToast() = toasts.toasts.value.last()

    /** Logs the administrator in through the engine. */
    suspend fun login() {
        coEvery { f.engine.login(ADMIN_INSZ, any()) } returns admin
        session.login(ADMIN_INSZ, "1234")
    }
}

/** An engine fixture under [root] whose reads answer from [world]; the main dispatcher is the fixture's. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.orderFixture(
    root: File,
    world: OrderWorld,
): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    coEvery { f.engine.getSettings() } returns posSettings()
    coEvery { f.engine.getCategories(any()) } answers { world.categories }
    coEvery { f.engine.getMenuItems(any(), any()) } answers { world.items[firstArg<Long?>()] ?: emptyList() }
    coEvery { f.engine.needsZReport() } answers { world.needsZ }
    coEvery { f.engine.getOrder(any()) } answers { order(firstArg()) }
    coEvery { f.engine.listCartLines(any()) } answers { world.cart }
    coEvery { f.engine.setActiveOrder(any()) } returns Unit
    coEvery { f.engine.getIntegrations() } answers { Integrations(world.hubrise, 0L, emptyList()) }
    return f
}

/** Builds the view model over real stores on [f]; the engine is opened first unless [open] is false. */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LongParameterList")
internal suspend fun TestScope.startOrder(
    f: EngineFixture,
    world: OrderWorld,
    context: Context,
    open: Boolean = true,
    routeOrderId: Long = ORDER_ID,
    shown: Boolean = true,
): OrderRig {
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errors = ErrorPresenter(toasts, ErrorTexts(catalog, FdmTexts(catalog)))
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val order = OrderStore(gateway, f.events, backgroundScope)
    val integrations = IntegrationsStore(gateway, f.events, backgroundScope)
    val amounts = CartAmounts(EngineHelpers(f.natives, f.dispatcher))
    if (open) {
        f.holder.open()
        runCurrent()
    }
    val entry = TestOrderEntry(session, order, gateway, f.events, errors, settings, integrations, amounts, toasts, catalog)
    val model = MenuViewModel(entry, routeOrderId)
    // The screen runs its model while it is composed; a model of a screen left on the back stack is not run.
    if (shown) backgroundScope.launch { model.run() }
    runCurrent()
    return OrderRig(f, world, model, session, order, settings, integrations, toasts, entry)
}
