@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.reports

import android.content.Context
import dev.nuagemagique.pilipos.engine.DisposableEngine
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.EventFeed
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import io.mockk.coEvery
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.CopyResult
import uniffi.pos_ffi.CopyableTicket
import uniffi.pos_ffi.FinancialResult
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.RefundableEvent
import uniffi.pos_ffi.ReportResult
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.ZReportSummary
import java.io.File

internal const val OPERATOR_INSZ = "85010100123"

internal val operatorUser = User(insz = OPERATOR_INSZ, name = "Alice", role = UserRole.ADMIN, active = true, hasPin = true)

internal fun zSummary(
    id: Long,
    fiscalEventId: Long? = id,
): ZReportSummary =
    ZReportSummary(
        id = id,
        reportDate = "2026-10-09",
        sequenceNumber = id,
        createdAt = "2026-10-09T22:00:00",
        fiscalEventId = fiscalEventId,
    )

internal fun reportResult(
    success: Boolean = true,
    error: String? = null,
    reportNo: Long? = null,
    type: String = "X",
): ReportResult = ReportResult(success = success, error = error, reportNo = reportNo, reportType = type)

internal fun financial(
    success: Boolean = true,
    error: String? = null,
): FinancialResult = FinancialResult(success = success, error = error)

internal fun copyable(
    id: Long,
    label: String = "N",
    totalCents: Long = 1000L,
): CopyableTicket =
    CopyableTicket(
        fiscalEventId = id,
        eventLabel = label,
        ticketNumber = id,
        tableNumber = 4,
        totalCents = totalCents,
        createdAt = "2026-10-09T12:00:00",
    )

internal fun refundable(
    id: Long,
    orderId: Long = id * 10L,
): RefundableEvent =
    RefundableEvent(
        fiscalEventId = id,
        orderId = orderId,
        ticketNumber = id,
        tableNumber = 4,
        totalCents = 1000L,
        createdAt = "2026-10-09T12:00:00",
    )

internal fun orderLine(
    id: Long,
    quantity: Int,
    menuItemId: Long = id * 100L,
): OrderLine =
    OrderLine(
        id = id,
        orderId = 1L,
        menuItemId = menuItemId,
        quantity = quantity,
        unitPriceCents = 500L,
        vatCode = VatCode.B,
        name = "item$id",
        modifiersJson = "[]",
        createdAt = "2026-10-09T10:00:00",
        departmentId = "d",
        departmentName = "Dept",
    )

internal fun copyResult(
    success: Boolean = true,
    error: String? = null,
): CopyResult = CopyResult(success = success, error = error, ticket = null, fiscalEventId = 7L, delivery = null)

internal fun checkoutResult(
    success: Boolean = true,
    error: String? = null,
): CheckoutResult =
    CheckoutResult(
        success = success,
        ticket = null,
        error = error,
        fdmConnected = true,
        fiscalEventId = 8L,
        delivery = null,
        kitchenSlip = null,
    )

internal fun delivery(
    medium: RequestedMedium,
    email: String? = null,
): TicketDeliveryRequest = TicketDeliveryRequest(medium = medium, email = email)

/** The reports entry point over real stores and a mocked engine. */
@Suppress("LongParameterList")
internal class TestReportsEntryPoint(
    private val gateway: EngineGateway,
    private val helpers: EngineHelpers,
    private val events: EngineEvents,
    private val session: SessionStore,
    private val settings: PosSettingsStore,
    private val errors: ErrorPresenter,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
) : ReportsEntryPoint {
    override fun gateway(): EngineGateway = gateway

    override fun helpers(): EngineHelpers = helpers

    override fun events(): EngineEvents = events

    override fun session(): SessionStore = session

    override fun settings(): PosSettingsStore = settings

    override fun errors(): ErrorPresenter = errors

    override fun toasts(): ToastStore = toasts

    override fun catalog(): MessageCatalog = catalog
}

/** A started reports view model with the real stores it drives, built on one engine fixture. */
internal class ReportsRig(
    val f: EngineFixture,
    val model: ReportsViewModel,
    val toasts: ToastStore,
    val catalog: MessageCatalog,
    val feed: EventFeed,
    private val errorTexts: ErrorTexts,
) {
    val state get() = model.state.value
    val engine: DisposableEngine get() = f.engine

    /** The French text of Vue [key]. */
    fun text(key: String): String = catalog.text(key, AppLanguage.FR)

    /** The French screen text of the raw engine error [raw]. */
    fun errorText(raw: String): String = errorTexts.describe(raw, AppLanguage.FR)

    /** The messages of the toasts shown so far. */
    fun toastTexts(): List<String> = toasts.toasts.value.map { it.message }
}

/** Stubs the reads the screen makes at start; a test overrides the one it cares about afterwards. */
internal fun stubReportReads(engine: DisposableEngine) {
    coEvery { engine.listZReports() } returns emptyList()
    coEvery { engine.needsZReport() } returns false
    coEvery { engine.listCopyableTickets(any()) } returns emptyList()
    coEvery { engine.listRefundableEvents(any()) } returns emptyList()
    coEvery { engine.login(any(), any()) } returns operatorUser
}

/** An engine fixture whose reads are stubbed; the main dispatcher is the fixture's. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.reportsFixture(root: File): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    stubReportReads(f.engine)
    return f
}

/** Builds the view model over real stores on [f]; the engine is opened first and the operator logged in unless told otherwise. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun TestScope.startReports(
    f: EngineFixture,
    context: Context,
    open: Boolean = true,
    loggedIn: Boolean = true,
): ReportsRig {
    val feed = EventFeed(f)
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errorTexts = ErrorTexts(catalog, FdmTexts(catalog))
    val errors = ErrorPresenter(toasts, errorTexts)
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val helpers = EngineHelpers(f.natives, f.dispatcher)
    if (open) {
        f.holder.open()
        runCurrent()
    }
    if (loggedIn && open) session.login(OPERATOR_INSZ, "1234")
    val entry = TestReportsEntryPoint(gateway, helpers, f.events, session, settings, errors, toasts, catalog)
    val model = ReportsViewModel(entry)
    runCurrent()
    return ReportsRig(f, model, toasts, catalog, feed, errorTexts)
}
