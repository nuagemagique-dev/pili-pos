@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.login

import android.content.Context
import dev.nuagemagique.pilipos.engine.DisposableEngine
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import io.mockk.coEvery
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import uniffi.pos_ffi.FdmHealth
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.HubriseConnectionInfo
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.OpenShift
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import java.io.File

internal const val ADMIN_INSZ = "85010100123"
internal const val CASHIER_INSZ = "90020200234"
internal const val WAITER_INSZ = "95030300345"

internal fun user(
    insz: String,
    name: String,
    role: UserRole,
    hasPin: Boolean = true,
    active: Boolean = true,
): User = User(insz = insz, name = name, role = role, active = active, hasPin = hasPin)

internal val admin = user(ADMIN_INSZ, "Alice", UserRole.ADMIN)
internal val cashier = user(CASHIER_INSZ, "Bob", UserRole.CASHIER)
internal val waiterNoPin = user(WAITER_INSZ, "Chen", UserRole.WAITER, hasPin = false)

internal fun settings(
    training: Boolean = false,
    restaurant: String = "Chez Pili",
    terminal: String = "T1",
    version: String = "pili-pos-android/1.2.3",
): PosSettings =
    PosSettings(
        trainingMode = training,
        printerAddress = null,
        fdmUrl = "http://fdm",
        posId = "POS1",
        terminalId = terminal,
        restaurantName = restaurant,
        language = "fr",
        configProblems = emptyList(),
        posSwVersion = version,
    )

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

internal fun hubriseInfo(connected: Boolean): HubriseConnectionInfo =
    HubriseConnectionInfo(connected, null, null, null, null, null, null, null, null)

internal fun hubriseSwitch(enabled: Boolean): Integrations =
    Integrations(
        hubriseEnabled = enabled,
        onlineOrdersOpen = 0L,
        hubriseLog = emptyList(),
    )

internal fun mailStatus(
    configured: Boolean,
    reachable: Boolean,
): MailStatus = MailStatus(configured = configured, reachable = reachable, allowDigitalOnly = false)

internal fun shift(
    insz: String,
    startedAt: String = "2026-10-09 09:05:00",
): OpenShift = OpenShift(userInsz = insz, startedAt = startedAt)

/** Stubs every read the login screen makes on [engine]; a test overrides the one it cares about afterwards. */
internal fun stubReads(
    engine: DisposableEngine,
    users: List<User> = listOf(admin, cashier, waiterNoPin),
    shifts: List<OpenShift> = emptyList(),
    hubrise: Boolean = false,
) {
    coEvery { engine.listUsers() } returns users
    coEvery { engine.listOpenShifts() } returns shifts
    coEvery { engine.checkPrinter() } returns true
    coEvery { engine.checkKitchenPrinter() } returns null
    coEvery { engine.checkMailStatus() } returns mailStatus(configured = false, reachable = false)
    coEvery { engine.hubriseGetConnectionStatus() } returns hubriseInfo(true)
    coEvery { engine.fdmCheckNow() } returns fdmHealth(FdmSeverity.INFO)
    coEvery { engine.getSettings() } returns settings()
    coEvery { engine.getIntegrations() } returns hubriseSwitch(hubrise)
}

/** The login entry point over the given real stores. */
@Suppress("LongParameterList")
internal class TestEntryPoint(
    private val session: SessionStore,
    private val settings: PosSettingsStore,
    private val integrations: IntegrationsStore,
    private val gateway: EngineGateway,
    private val errors: ErrorPresenter,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
    private val errorTexts: ErrorTexts,
) : LoginEntryPoint {
    override fun session(): SessionStore = session

    override fun settings(): PosSettingsStore = settings

    override fun integrations(): IntegrationsStore = integrations

    override fun gateway(): EngineGateway = gateway

    override fun errors(): ErrorPresenter = errors

    override fun toasts(): ToastStore = toasts

    override fun catalog(): MessageCatalog = catalog

    override fun errorTexts(): ErrorTexts = errorTexts
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

/** A started login view model with the real stores it drives, built on one engine fixture. */
internal class LoginRig(
    val f: EngineFixture,
    val model: LoginViewModel,
    val toasts: ToastStore,
    val settings: PosSettingsStore,
    val catalog: MessageCatalog,
) {
    val state get() = model.state.value

    /** The French text of Vue [key]. */
    fun text(
        key: String,
        vars: Map<String, Any> = emptyMap(),
    ): String = catalog.text(key, AppLanguage.FR, vars)

    fun lastToast() = toasts.toasts.value.last()

    /** Presses the digits on the PIN pad. */
    fun type(digits: String) {
        digits.forEach { model.onPinKey(NumPadKey.Digit(it)) }
    }

    /** Presses the digits on the INSZ numpad. */
    fun typeInsz(digits: String) {
        digits.forEach { model.onInszKey(NumPadKey.Digit(it)) }
    }
}

/** An engine fixture under [root] whose reads are stubbed; the main dispatcher is the fixture's. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.loginFixture(root: File): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    stubReads(f.engine)
    return f
}

/** Builds the view model over real stores on [f]; the engine is opened first unless [open] is false. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun TestScope.startLogin(
    f: EngineFixture,
    context: Context,
    open: Boolean = true,
): LoginRig {
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errorTexts = ErrorTexts(catalog, FdmTexts(catalog))
    val errors = ErrorPresenter(toasts, errorTexts)
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val integrations = IntegrationsStore(gateway, f.events, backgroundScope)
    if (open) {
        f.holder.open()
        runCurrent()
    }
    val entry = TestEntryPoint(session, settings, integrations, gateway, errors, toasts, catalog, errorTexts)
    val model = LoginViewModel(entry)
    runCurrent()
    return LoginRig(f, model, toasts, settings, catalog)
}
