@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.settings

import android.content.Context
import dev.nuagemagique.pilipos.engine.DisposableEngine
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.feature.checkout.CheckoutNatives
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.FakeEntry
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.nav.SettingsEntry
import dev.nuagemagique.pilipos.nav.registryOf
import dev.nuagemagique.pilipos.ui.badge.BadgeRig
import dev.nuagemagique.pilipos.ui.badge.BadgeStore
import dev.nuagemagique.pilipos.ui.badge.FakeNfcSupport
import dev.nuagemagique.pilipos.ui.badge.badgeRigOver
import dev.nuagemagique.pilipos.ui.badge.badgeView
import dev.nuagemagique.pilipos.ui.badge.noBadges
import dev.nuagemagique.pilipos.ui.components.QrMatrices
import dev.nuagemagique.pilipos.ui.fdm.FdmStore
import dev.nuagemagique.pilipos.ui.fdm.health
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.DeliveryStore
import dev.nuagemagique.pilipos.ui.state.EftStore
import dev.nuagemagique.pilipos.ui.state.EventFeed
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore
import io.mockk.coEvery
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import uniffi.pos_ffi.DeliveryView
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.TablePlan
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import java.io.File

internal const val ADMIN_INSZ = "85010100123"
internal const val CASHIER_INSZ = "90020200234"
internal const val WAITER_INSZ = "95030300345"
internal const val NEW_INSZ = "99040400456"

@Suppress("LongParameterList")
internal fun user(
    insz: String,
    name: String,
    role: UserRole,
    hasPin: Boolean = true,
    active: Boolean = true,
    presenceOnly: Boolean = false,
): User = User(insz = insz, name = name, role = role, active = active, hasPin = hasPin, presenceOnly = presenceOnly)

internal val admin = user(ADMIN_INSZ, "Alice", UserRole.ADMIN)
internal val cashier = user(CASHIER_INSZ, "Bob", UserRole.CASHIER)
internal val waiter = user(WAITER_INSZ, "Chen", UserRole.WAITER, hasPin = false)

/** Kitchen staff who never use the till: a waiter row with the presence-only flag and no PIN. */
internal const val COOK_INSZ = "88010112345"
internal val cook = user(COOK_INSZ, "Dao", UserRole.WAITER, hasPin = false, presenceOnly = true)

internal fun engineSettings(training: Boolean = false): PosSettings =
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

internal fun integrationsOf(
    enabled: Boolean,
    open: Long = 0L,
): Integrations = Integrations(hubriseEnabled = enabled, onlineOrdersOpen = open, hubriseLog = emptyList())

internal fun delivery(id: Long): DeliveryView =
    DeliveryView(
        id = id,
        fiscalEventId = 10L + id,
        medium = "email",
        recipientHint = "a***@x.be",
        status = "sent",
        attempts = 1L,
        lastError = null,
        fallbackPrinted = false,
        createdAt = "2026-10-09 09:05:00",
    )

internal fun mail(
    configured: Boolean = true,
    reachable: Boolean = true,
): MailStatus = MailStatus(configured = configured, reachable = reachable, allowDigitalOnly = false)

/** Stubs every read the settings screen and the HubRise panel make; a test overrides the one it cares about afterwards. */
internal fun stubReads(
    engine: DisposableEngine,
    users: List<User> = listOf(admin, cashier, waiter),
    hubrise: Boolean = false,
) {
    coEvery { engine.listUsers() } returns users
    coEvery { engine.listOpenShifts() } returns emptyList()
    coEvery { engine.fdmHealth() } returns health()
    coEvery { engine.checkMailStatus() } returns mail()
    coEvery { engine.listTicketDeliveries(50L) } returns listOf(delivery(1L), delivery(2L))
    coEvery { engine.getSettings() } returns engineSettings()
    coEvery { engine.getIntegrations() } returns integrationsOf(hubrise)
    coEvery { engine.hubriseListWebshopOrders() } returns emptyList()
    coEvery { engine.getTablePlan() } returns TablePlan(plan = "1-20", tableCount = 20L, log = emptyList())
    coEvery { engine.getBadgeSettings() } returns badgeView()
    coEvery { engine.listBadges() } returns noBadges()
}

/** Three settings rows: one open to every role, two for managers, in a mixed order to prove the sort. */
internal val rowOpen = SettingsEntry("a.open", 1, "settings-open", 10, managerOnly = false)
internal val rowAbout = SettingsEntry("platform.about", 2, "settings-about", 900, managerOnly = true)
internal val rowExport = SettingsEntry("platform.export", 3, "settings-export", 910, managerOnly = true)

internal fun testRegistry(): NavRegistry =
    registryOf(
        entries =
            setOf(
                FakeEntry("login", "login"),
                FakeEntry("settings", "settings"),
                FakeEntry("platform", "settings-open", "settings-about", "settings-export"),
            ),
        settings = setOf(rowExport, rowOpen, rowAbout),
    )

/** The entry point over the given real stores. */
@Suppress("LongParameterList")
internal class TestSettingsEntryPoint(
    private val gateway: EngineGateway,
    private val events: EngineEvents,
    private val helpers: EngineHelpers,
    private val session: SessionStore,
    private val settings: PosSettingsStore,
    private val integrations: IntegrationsStore,
    private val errors: ErrorPresenter,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
    private val registry: NavRegistry,
    private val delivery: DeliveryStore,
    private val webshop: WebshopOrdersStore,
    private val fdm: FdmStore,
    private val badges: BadgeStore,
) : SettingsFeatureEntryPoint {
    override fun gateway(): EngineGateway = gateway

    override fun events(): EngineEvents = events

    override fun helpers(): EngineHelpers = helpers

    override fun session(): SessionStore = session

    override fun settings(): PosSettingsStore = settings

    override fun integrations(): IntegrationsStore = integrations

    override fun errors(): ErrorPresenter = errors

    override fun toasts(): ToastStore = toasts

    override fun catalog(): MessageCatalog = catalog

    override fun registry(): NavRegistry = registry

    override fun delivery(): DeliveryStore = delivery

    override fun webshop(): WebshopOrdersStore = webshop

    override fun fdm(): FdmStore = fdm

    override fun badges(): BadgeStore = badges

    override fun qrMatrices(): QrMatrices = QrMatrices(helpers)
}

/** The real stores of one test over one engine fixture, and the entry point built on them. */
@Suppress("LongParameterList")
internal class SettingsEnv(
    val f: EngineFixture,
    val entry: TestSettingsEntryPoint,
    val feed: EventFeed,
    val toasts: ToastStore,
    val errors: ErrorPresenter,
    val catalog: MessageCatalog,
    val session: SessionStore,
    val settings: PosSettingsStore,
    val integrations: IntegrationsStore,
    val delivery: DeliveryStore,
    val webshop: WebshopOrdersStore,
    val fdm: FdmStore,
    val badges: BadgeRig,
) {
    /** The French text of Vue [key]. */
    fun text(
        key: String,
        vars: Map<String, Any> = emptyMap(),
    ): String = catalog.text(key, AppLanguage.FR, vars)

    fun lastToast() = toasts.toasts.value.last()

    /** The French screen text of [error]. */
    fun describe(error: Throwable): String = errors.text(error, AppLanguage.FR)

    /** Logs [who] in through the session store, so the role flags are set. */
    @OptIn(ExperimentalCoroutinesApi::class)
    suspend fun TestScope.loginAs(who: User) {
        coEvery { f.engine.login(who.insz, null) } returns who
        session.login(who.insz, null)
        runCurrent()
    }
}

/** An engine fixture under [root] whose reads are stubbed; the main dispatcher is the fixture's. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.settingsFixture(
    root: File,
    hubrise: Boolean = false,
): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    stubReads(f.engine, hubrise = hubrise)
    return f
}

/** The stores over [f], created before the engine opens (they subscribe to the events first). */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.settingsEnv(
    f: EngineFixture,
    context: Context,
    nfc: FakeNfcSupport = FakeNfcSupport(),
): SettingsEnv {
    val feed = EventFeed(f)
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errors = ErrorPresenter(toasts, ErrorTexts(catalog, FdmTexts(catalog)))
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val integrations = IntegrationsStore(gateway, f.events, backgroundScope)
    val helpers = EngineHelpers(f.natives, f.dispatcher)
    val delivery = DeliveryStore(gateway, errors, settings)
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
    val fdm = FdmStore(gateway, f.events, errors, toasts, FdmTexts(catalog), settings, backgroundScope)
    val badges = badgeRigOver(f, gateway, session, toasts, errors, settings, catalog, nfc)
    val entry =
        TestSettingsEntryPoint(
            gateway, f.events, helpers, session, settings, integrations, errors, toasts, catalog, testRegistry(), delivery, webshop, fdm,
            badges.store,
        )
    return SettingsEnv(f, entry, feed, toasts, errors, catalog, session, settings, integrations, delivery, webshop, fdm, badges)
}

/** Opens the engine and lets the stores load. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun TestScope.openEngine(env: SettingsEnv) {
    env.f.holder.open()
    runCurrent()
}
