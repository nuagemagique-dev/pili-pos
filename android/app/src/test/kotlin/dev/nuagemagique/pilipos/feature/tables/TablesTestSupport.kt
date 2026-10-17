@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.tables

import android.content.Context
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
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
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.TableInfo
import uniffi.pos_ffi.TakeawayInfo
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import java.io.File
import java.time.Clock
import java.time.Instant
import java.time.ZoneId
import java.time.ZoneOffset

internal const val ADMIN_INSZ = "85010100123"

internal val admin = User(insz = ADMIN_INSZ, name = "Alice", role = UserRole.ADMIN, active = true, hasPin = true)

/** A clock the test moves by hand. */
internal class HandClock(
    var now: Instant,
) : Clock() {
    override fun getZone(): ZoneId = ZoneOffset.UTC

    override fun withZone(zone: ZoneId?): Clock = this

    override fun instant(): Instant = now
}

@Suppress("LongParameterList")
internal fun table(
    number: Int,
    orderId: Long? = null,
    lines: Long = 0,
    totalCents: Long = 0,
    openedAt: String? = null,
    preBill: Boolean = false,
): TableInfo =
    TableInfo(
        tableNumber = number,
        orderId = orderId,
        lineCount = lines,
        totalCents = totalCents,
        status = if (orderId != null) "occupied" else "free",
        openedAt = openedAt,
        preBillPrinted = preBill,
    )

internal fun takeaway(
    orderId: Long,
    lines: Long = 1,
    totalCents: Long = 0,
    createdAt: String = "2026-10-09 10:05:00",
): TakeawayInfo = TakeawayInfo(orderId = orderId, lineCount = lines, totalCents = totalCents, createdAt = createdAt)

internal fun order(
    id: Long,
    table: Int? = null,
): Order =
    Order(
        id = id,
        tableNumber = table,
        orderType = if (table != null) OrderType.DINE_IN else OrderType.TAKEAWAY,
        status = OrderStatus.OPEN,
        userInsz = ADMIN_INSZ,
        createdAt = "2026-10-09T10:00:00",
        closedAt = null,
        subtotalCents = 0L,
        totalCents = 0L,
    )

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

/** The tables entry point over the given real stores. */
@Suppress("LongParameterList")
internal class TestTablesEntry(
    private val session: SessionStore,
    private val order: OrderStore,
    private val gateway: EngineGateway,
    private val events: EngineEvents,
    private val errors: ErrorPresenter,
    private val settings: PosSettingsStore,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
) : TablesEntryPoint {
    override fun session(): SessionStore = session

    override fun order(): OrderStore = order

    override fun gateway(): EngineGateway = gateway

    override fun events(): EngineEvents = events

    override fun errors(): ErrorPresenter = errors

    override fun settings(): PosSettingsStore = settings

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

/** A started tables view model with the real stores it drives, built on one engine fixture. */
@Suppress("LongParameterList")
internal class TablesRig(
    val f: EngineFixture,
    val model: TablesViewModel,
    val session: SessionStore,
    val order: OrderStore,
    val toasts: ToastStore,
    val catalog: MessageCatalog,
    val errorTexts: ErrorTexts,
    val clock: HandClock,
    val effects: List<TablesEffect>,
    private val collector: Job,
) {
    val state get() = model.state.value

    fun lastToast() = toasts.toasts.value.last()

    /** Logs the administrator in through the engine. */
    suspend fun login() {
        coEvery { f.engine.login(ADMIN_INSZ, any()) } returns admin
        session.login(ADMIN_INSZ, "1234")
    }

    fun stop() = collector.cancel()
}

/** An engine fixture under [root] whose reads are stubbed; the main dispatcher is the fixture's. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.tablesFixture(
    root: File,
    tables: List<TableInfo> = emptyList(),
    takeaways: List<TakeawayInfo> = emptyList(),
): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    coEvery { f.engine.getSettings() } returns posSettings()
    coEvery { f.engine.listTables() } returns tables
    coEvery { f.engine.listOpenTakeaways() } returns takeaways
    coEvery { f.engine.listUnsettledCardPayments() } returns emptyList()
    coEvery { f.engine.listCartLines(any()) } returns emptyList()
    coEvery { f.engine.setActiveOrder(any()) } returns Unit
    return f
}

/** Builds the view model over real stores on [f]; the engine is opened first unless [open] is false. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun TestScope.startTables(
    f: EngineFixture,
    context: Context,
    open: Boolean = true,
    clock: HandClock = HandClock(Instant.parse("2026-10-09T10:30:00Z")),
): TablesRig {
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errorTexts = ErrorTexts(catalog, FdmTexts(catalog))
    val errors = ErrorPresenter(toasts, errorTexts)
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val order = OrderStore(gateway, f.events, backgroundScope)
    if (open) {
        f.holder.open()
        runCurrent()
    }
    val entry = TestTablesEntry(session, order, gateway, f.events, errors, settings, toasts, catalog)
    val model = TablesViewModel(entry, clock)
    val seen = mutableListOf<TablesEffect>()
    val collector = CoroutineScope(UnconfinedTestDispatcher(testScheduler)).launch { model.effects.collect { seen += it } }
    runCurrent()
    return TablesRig(f, model, session, order, toasts, catalog, errorTexts, clock, seen, collector)
}

/** The French language, for the expected texts. */
internal val FR = AppLanguage.FR

/** An order created for the table or takeaway in [input]. */
internal fun CreateOrderInput.isTakeaway(): Boolean = orderType == OrderType.TAKEAWAY
