@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.ui.scaffold

import android.content.Context
import dev.nuagemagique.pilipos.engine.DisposableEngine
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.feature.checkout.CheckoutNatives
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.fdm.FdmStore
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.EftStore
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore
import io.mockk.coEvery
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.FdmHealth
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.HubriseStatus
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.LockOutcome
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentTerminalStatus
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import java.io.File

internal const val ADMIN_INSZ = "85010100123"
internal const val CASHIER_INSZ = "90020200234"

internal val admin = User(insz = ADMIN_INSZ, name = "Alice", role = UserRole.ADMIN, active = true, hasPin = true)
internal val cashier = User(insz = CASHIER_INSZ, name = "Bob", role = UserRole.CASHIER, active = true, hasPin = true)

internal fun posSettings(training: Boolean = false): PosSettings =
    PosSettings(
        trainingMode = training,
        printerAddress = null,
        fdmUrl = "http://fdm",
        posId = "POS1",
        terminalId = "T1",
        restaurantName = "Chez Pili",
        language = "fr",
        configProblems = emptyList(),
        posSwVersion = "pili-pos-android/1.2.3",
    )

internal fun integrations(
    enabled: Boolean = false,
    open: Long = 0L,
): Integrations = Integrations(hubriseEnabled = enabled, onlineOrdersOpen = open, hubriseLog = emptyList())

internal fun fdmHealth(severity: FdmSeverity): FdmHealth =
    FdmHealth(
        connected = severity != FdmSeverity.BLOCKING,
        answered = true,
        severity = severity,
        code = null,
        mandatory = false,
        checkedAt = "2026-10-09T10:00:00Z",
        nextCheckAt = null,
    )

internal fun order(
    id: Long,
    table: Int? = null,
    type: OrderType = if (table != null) OrderType.DINE_IN else OrderType.TAKEAWAY,
): Order =
    Order(
        id = id,
        tableNumber = table,
        orderType = type,
        status = OrderStatus.OPEN,
        userInsz = ADMIN_INSZ,
        createdAt = "2026-10-09T10:00:00",
        closedAt = null,
        subtotalCents = 0L,
        totalCents = 0L,
    )

/** Stubs every read the scaffold makes on [engine]; a test overrides the one it cares about afterwards. */
internal fun stubScaffold(
    engine: DisposableEngine,
    hubrise: Boolean = false,
    training: Boolean = false,
) {
    coEvery { engine.getSettings() } returns posSettings(training)
    coEvery { engine.getIntegrations() } returns integrations(enabled = hubrise)
    coEvery { engine.fdmHealth() } returns null
    coEvery { engine.fdmCheckNow() } returns fdmHealth(FdmSeverity.INFO)
    coEvery { engine.hubriseListWebshopOrders() } returns emptyList()
    coEvery { engine.checkPrinter() } returns true
    coEvery { engine.checkKitchenPrinter() } returns null
    coEvery { engine.checkPaymentTerminal() } returns noTerminal()
    coEvery { engine.listCartLines(any()) } returns emptyList<CartLine>()
    coEvery { engine.setActiveOrder(any()) } returns Unit
    coEvery { engine.leaveOrder(any(), any()) } returns SlipOutcome.NOTHING_TO_PRINT
    coEvery { engine.lockSession() } returns LockOutcome(orderError = null)
}

/** The scaffold entry point over the given real stores. */
@Suppress("LongParameterList")
internal class TestScaffoldEntry(
    private val session: SessionStore,
    private val settings: PosSettingsStore,
    private val integrations: IntegrationsStore,
    private val order: OrderStore,
    private val fdm: FdmStore,
    private val webshop: WebshopOrdersStore,
    private val eft: EftStore,
    private val gateway: EngineGateway,
    private val errors: ErrorPresenter,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
) : ScaffoldEntryPoint {
    override fun session(): SessionStore = session

    override fun settings(): PosSettingsStore = settings

    override fun integrations(): IntegrationsStore = integrations

    override fun order(): OrderStore = order

    override fun fdm(): FdmStore = fdm

    override fun webshop(): WebshopOrdersStore = webshop

    override fun eft(): EftStore = eft

    override fun gateway(): EngineGateway = gateway

    override fun errors(): ErrorPresenter = errors

    override fun toasts(): ToastStore = toasts

    override fun catalog(): MessageCatalog = catalog
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

/** A started header view model with the real stores it drives, built on one engine fixture. */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LongParameterList")
internal class ScaffoldRig(
    val f: EngineFixture,
    val model: HeaderViewModel,
    val entry: TestScaffoldEntry,
    val session: SessionStore,
    val order: OrderStore,
    val toasts: ToastStore,
    val catalog: MessageCatalog,
    val errorTexts: ErrorTexts,
    val effects: List<HeaderEffect>,
    private val collector: Job,
) {
    val state get() = model.state.value

    /** The French text of Vue [key]. */
    fun text(
        key: String,
        vars: Map<String, Any> = emptyMap(),
    ): String = catalog.text(key, AppLanguage.FR, vars)

    fun lastToast() = toasts.toasts.value.last()

    /** Logs [user] in through the engine. */
    suspend fun login(user: User) {
        coEvery { f.engine.login(user.insz, any()) } returns user
        session.login(user.insz, "1234")
    }

    /** Starts [order] as the current order of the store (a table, or a takeaway). */
    suspend fun openOrder(order: Order) {
        coEvery { f.engine.createOrder(CreateOrderInput(order.orderType, order.tableNumber, order.userInsz)) } returns order
        coEvery { f.engine.getOrder(order.id) } returns order
        this.order.loadOrder(order.id)
    }

    fun stop() = collector.cancel()
}

/** An engine fixture under [root] whose reads are stubbed; the main dispatcher is the fixture's. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.scaffoldFixture(
    root: File,
    hubrise: Boolean = false,
): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    stubScaffold(f.engine, hubrise)
    return f
}

/** Builds the view model over real stores on [f]; the engine is opened first unless [open] is false. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun TestScope.startScaffold(
    f: EngineFixture,
    context: Context,
    area: HeaderArea,
    open: Boolean = true,
): ScaffoldRig {
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errorTexts = ErrorTexts(catalog, FdmTexts(catalog))
    val errors = ErrorPresenter(toasts, errorTexts)
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val integrations = IntegrationsStore(gateway, f.events, backgroundScope)
    val order = OrderStore(gateway, f.events, backgroundScope)
    val fdm = FdmStore(gateway, f.events, errors, toasts, FdmTexts(catalog), settings, backgroundScope)
    val eft = EftStore(gateway, errors, settings, backgroundScope)
    val webshop =
        WebshopOrdersStore(
            gateway,
            f.events,
            errors,
            toasts,
            integrations,
            settings,
            catalog,
            eft,
            EngineHelpers(CheckoutNatives(), UnconfinedTestDispatcher(testScheduler)),
            backgroundScope,
        )
    if (open) {
        f.holder.open()
        runCurrent()
    }
    val entry = TestScaffoldEntry(session, settings, integrations, order, fdm, webshop, eft, gateway, errors, toasts, catalog)
    val model = HeaderViewModel(entry, area)
    val seen = mutableListOf<HeaderEffect>()
    val collector = CoroutineScope(UnconfinedTestDispatcher(testScheduler)).launch { model.effects.collect { seen += it } }
    runCurrent()
    return ScaffoldRig(f, model, entry, session, order, toasts, catalog, errorTexts, seen, collector)
}

/** What a till without a linked card terminal answers. */
internal fun noTerminal(): PaymentTerminalStatus =
    PaymentTerminalStatus(configured = false, kind = null, connected = false, busy = false, detail = null)

/** A hubrise status with everything quiet; a test copies it to change one field. */
internal fun quietHubrise(): HubriseStatus =
    HubriseStatus(
        health =
            uniffi.pos_ffi.ConnectionHealth(
                connected = true,
                lastPollAt = null,
                authErrorAt = null,
                lastError = null,
                callbackRegisteredAt = null,
                lastCatalogSyncAt = null,
            ),
        outbox = uniffi.pos_ffi.Backlog(pending = 0L, parked = 0L),
        outboxError = null,
        parkedEvents = 0L,
        staleOrders = 0L,
        catalog = null,
        catalogAwaitingAdoption = false,
    )
