@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.webshop

import android.content.Context
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore
import io.mockk.coEvery
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import uniffi.pos_ffi.AcceptResult
import uniffi.pos_ffi.Attention
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.LockOutcome
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import uniffi.pos_ffi.WebshopOrderItem
import uniffi.pos_ffi.WebshopOrderSummary
import uniffi.pos_ffi.WebshopState
import java.io.File
import java.time.Clock
import java.time.Instant
import java.time.ZoneId
import java.time.ZoneOffset

internal const val ADMIN_INSZ = "85010100123"
internal const val CASHIER_INSZ = "90020200234"

internal val admin = User(insz = ADMIN_INSZ, name = "Alice", role = UserRole.ADMIN, active = true, hasPin = true)
internal val cashier = User(insz = CASHIER_INSZ, name = "Carl", role = UserRole.CASHIER, active = true, hasPin = true)

/** The French language, for the expected texts. */
internal val FR = AppLanguage.FR

/** 10:30 UTC is 12:30 in Brussels in October. */
internal val NOW: Instant = Instant.parse("2026-10-09T10:30:00Z")

/** A clock the test moves by hand. */
internal class HandClock(
    var now: Instant,
) : Clock() {
    override fun getZone(): ZoneId = ZoneOffset.UTC

    override fun withZone(zone: ZoneId?): Clock = this

    override fun instant(): Instant = now
}

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

/** The state the engine reports for a webshop order; [attention] makes it an order to check unless [resolution] says ok. */
internal fun webshopState(
    orderId: Long,
    fiscal: String,
    resolution: String = "ok",
    attention: List<Attention> = emptyList(),
    paymentMode: String? = null,
) = WebshopState(
    orderId = orderId,
    hubriseOrderId = "h$orderId",
    channel = "Web",
    channelKind = null,
    serviceType = null,
    paymentMode = paymentMode,
    paymentType = null,
    fiscalState = fiscal,
    resolution = resolution,
    attention = attention,
    warnings = emptyList(),
    lines = emptyList(),
)

internal fun item(
    name: String = "Sushi saumon",
    zh: String? = "三文鱼寿司",
    quantity: Long = 2,
    options: List<String> = emptyList(),
    notes: String? = null,
) = WebshopOrderItem(
    productName = name,
    quantity = quantity,
    unitPriceCents = 500L,
    lineTotalCents = 1_000L,
    options = options,
    customerNotes = notes,
    productNameZh = zh,
)

/** A webshop order as `hubrise_list_webshop_orders` lists it; [fiscal] null gives an order without a webshop record. */
@Suppress("LongParameterList")
internal fun webshopOrder(
    id: Long,
    hubrise: String? = "new",
    fiscal: String? = "pending",
    source: String = "deliveroo",
    service: String? = "collection",
    confirmed: String? = null,
    totalCents: Long = 2_000,
    createdAt: String = "2026-10-09T10:25:00Z",
    items: List<WebshopOrderItem> = listOf(item()),
    customer: String? = "Client",
    notes: String? = null,
    address: List<String?> = listOf(null, null, null, null),
    state: WebshopState? = fiscal?.let { webshopState(id, it) },
) = WebshopOrderSummary(
    id = id,
    hubriseOrderId = "h$id",
    status = "open",
    hubriseStatus = hubrise,
    source = source,
    serviceType = service,
    totalCents = totalCents,
    createdAt = createdAt,
    items = items,
    customerNotes = notes,
    customerName = customer,
    confirmedTime = confirmed,
    customerAddress1 = address[0],
    customerAddress2 = address[1],
    customerPostalCode = address[2],
    customerCity = address[3],
    webshop = state,
)

internal fun sale(
    success: Boolean = true,
    error: String? = null,
) = CheckoutResult(
    success = success,
    ticket = null,
    error = error,
    fdmConnected = true,
    fiscalEventId = null,
    delivery = null,
    kitchenSlip = null,
)

internal fun accepted(printed: Boolean = true) = AcceptResult("signed_n", 1L, null, printed)

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

internal fun integrations(enabled: Boolean = true): Integrations =
    Integrations(
        hubriseEnabled = enabled,
        onlineOrdersOpen = 0L,
        hubriseLog = emptyList(),
    )

/** The webshop entry point over the given real stores. */
@Suppress("LongParameterList")
internal class TestWebshopEntry(
    private val webshop: WebshopOrdersStore,
    private val session: SessionStore,
    private val settings: PosSettingsStore,
    private val integrations: IntegrationsStore,
    private val errors: ErrorPresenter,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
) : WebshopEntryPoint {
    override fun webshop(): WebshopOrdersStore = webshop

    override fun session(): SessionStore = session

    override fun settings(): PosSettingsStore = settings

    override fun integrations(): IntegrationsStore = integrations

    override fun errors(): ErrorPresenter = errors

    override fun toasts(): ToastStore = toasts

    override fun catalog(): MessageCatalog = catalog
}

/** A started webshop view model with the real stores it drives, built on one engine fixture. */
@Suppress("LongParameterList")
internal class WebshopRig(
    val f: EngineFixture,
    val model: WebshopViewModel,
    val store: WebshopOrdersStore,
    val session: SessionStore,
    val toasts: ToastStore,
    val catalog: MessageCatalog,
    val clock: HandClock,
) {
    val state get() = model.state.value

    fun lastToast() = toasts.toasts.value.last()

    /** Logs [user] in through the engine. */
    suspend fun login(user: User = admin) {
        coEvery { f.engine.login(user.insz, any()) } returns user
        session.login(user.insz, "1234")
    }

    /** The order with id [id] as the store lists it now. */
    fun order(id: Long): WebshopOrderSummary = store.orders.value.first { it.id == id }

    /** The French text of Vue [key]. */
    fun text(key: String): String = catalog.text(key, FR)
}

/** An engine fixture under [root] whose reads are stubbed; the main dispatcher is the fixture's. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.webshopFixture(
    root: File,
    orders: List<WebshopOrderSummary> = emptyList(),
    hubrise: Boolean = true,
): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    coEvery { f.engine.getSettings() } returns posSettings()
    coEvery { f.engine.getIntegrations() } returns integrations(hubrise)
    coEvery { f.engine.hubriseListWebshopOrders() } returns orders
    coEvery { f.engine.lockSession() } returns LockOutcome(orderError = null)
    return f
}

/** Builds the view model over real stores on [f]; the engine is opened first unless [open] is false. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun TestScope.startWebshop(
    f: EngineFixture,
    context: Context,
    open: Boolean = true,
    clock: HandClock = HandClock(NOW),
): WebshopRig {
    val gateway: EngineGateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errors = ErrorPresenter(toasts, ErrorTexts(catalog, FdmTexts(catalog)))
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val integrations = IntegrationsStore(gateway, f.events, backgroundScope)
    val store = WebshopOrdersStore(gateway, f.events, errors, toasts, integrations, settings, catalog, backgroundScope)
    store.clock = clock
    if (open) {
        f.holder.open()
        runCurrent()
    }
    val entry = TestWebshopEntry(store, session, settings, integrations, errors, toasts, catalog)
    val model = WebshopViewModel(entry, clock)
    runCurrent()
    return WebshopRig(f, model, store, session, toasts, catalog, clock)
}
