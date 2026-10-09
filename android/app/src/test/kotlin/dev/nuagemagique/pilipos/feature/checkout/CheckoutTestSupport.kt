@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.engine.FakeNatives
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.format.CartAmounts
import dev.nuagemagique.pilipos.ui.state.DeliveryStore
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
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.SettledPaymentRow
import uniffi.pos_ffi.SettlementPreview
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow
import java.io.File

internal const val CASHIER_INSZ = "85010100123"
internal const val ORDER_ID = 7L

internal val cashier = User(insz = CASHIER_INSZ, name = "Alice", role = UserRole.CASHIER, active = true, hasPin = true)
internal val waiter = User(insz = "85010100456", name = "Bob", role = UserRole.WAITER, active = true, hasPin = true)

/** The closing rule of the engine (`round_to_5_cents`): the delta added to a cash rest. */
private fun roundDelta(cents: Long): Long {
    val remainder = cents % 5
    return when {
        remainder == 0L -> 0L
        remainder <= 2L -> -remainder
        else -> 5 - remainder
    }
}

private fun refusal(code: String) = PosException.Coded(code, emptyMap())

/**
 * `pos_fiscal::payment::settle` in Kotlin, for the fake natives only: non-cash parts first, then the cash rest and
 * its rounding. The app never does this arithmetic: the tests need an engine stand-in that keeps the rules.
 */
internal fun settleLikeEngine(
    total: Long,
    parts: List<PaymentPart>,
): SettlementPreview {
    if (total == 0L) return SettlementPreview(listOf(SettledPaymentRow(PaymentMethod.CASH, 0L)), 0L, 0L, false)
    if (total < 0L) throw refusal("payment_invalid")
    if (parts.isEmpty()) throw refusal("payment_incomplete")
    val rows = mutableListOf<SettledPaymentRow>()
    var cash = false
    var nonCash = 0L
    for (part in parts) {
        val amount = part.amountCents
        if (part.method == PaymentMethod.CASH && amount == null && !cash) {
            cash = true
        } else if (part.method == PaymentMethod.CASH || amount == null || amount <= 0L) {
            throw refusal("payment_invalid")
        } else {
            nonCash += amount
            if (nonCash > total) throw refusal("payment_exceeds_total")
            rows += SettledPaymentRow(part.method, amount)
        }
    }
    val rest = total - nonCash
    var rounding = 0L
    if (rest > 0L) {
        if (!cash) throw refusal("payment_incomplete")
        rounding = roundDelta(rest)
        rows += SettledPaymentRow(PaymentMethod.CASH, rest)
    }
    val cashDue = rows.filter { it.method == PaymentMethod.CASH }.sumOf { it.amountCents } + rounding
    return SettlementPreview(rows, rounding, cashDue, rows.any { it.method == PaymentMethod.CASH && it.amountCents != 0L })
}

/** Natives whose helpers answer like the engine: the settlement, and a VAT row per line. */
internal class CheckoutNatives(
    private val base: FakeNatives = FakeNatives(),
) : EngineNatives by base {
    /** Every `settlePreview` call, to prove what the view model asked of the engine. */
    val previews: MutableList<Pair<Long, List<PaymentPart>>> = mutableListOf()

    override suspend fun settlePreview(
        totalCents: Long,
        parts: List<PaymentPart>,
    ): SettlementPreview {
        previews += totalCents to parts
        return settleLikeEngine(totalCents, parts)
    }

    override suspend fun cashDueCents(
        totalCents: Long,
        parts: List<PaymentPart>,
    ): Long = settleLikeEngine(totalCents, parts).cashDueCents

    override suspend fun vatSplit(lines: List<VatLine>): List<VatSplitRow> =
        lines.filter { it.vatCode != VatCode.X }.map { VatSplitRow(it.vatCode, 6, it.unitPriceCents * it.quantity, 0L) }
}

internal fun checkoutOrder(
    totalCents: Long,
    id: Long = ORDER_ID,
    table: Int? = 5,
): Order =
    Order(
        id = id,
        tableNumber = table,
        orderType = if (table != null) OrderType.DINE_IN else OrderType.TAKEAWAY,
        status = OrderStatus.OPEN,
        userInsz = CASHIER_INSZ,
        createdAt = "2026-10-09T10:00:00",
        closedAt = null,
        subtotalCents = totalCents,
        totalCents = totalCents,
    )

@Suppress("LongParameterList")
internal fun cartLine(
    id: Long,
    name: String = "Sushi",
    quantity: Int = 1,
    unitPriceCents: Long = 500L,
    vat: VatCode = VatCode.C,
    modifiersJson: String? = null,
): CartLine =
    CartLine(
        id = id,
        orderId = ORDER_ID,
        menuItemId = id,
        quantity = quantity,
        unitPriceCents = unitPriceCents,
        vatCode = vat,
        name = name,
        modifiersJson = modifiersJson,
        createdAt = "2026-10-09 10:00:00",
        departmentId = "d",
        departmentName = "Dept",
        nameFr = null,
        nameZh = null,
        code = null,
        unsentQuantity = 0,
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

internal fun sale(
    fiscalEventId: Long? = 99L,
    fdmConnected: Boolean = true,
    delivery: uniffi.pos_ffi.DeliveryOutcome? = null,
    slip: uniffi.pos_ffi.SlipOutcome? = null,
    success: Boolean = true,
    error: String? = null,
): CheckoutResult =
    CheckoutResult(
        success = success,
        ticket = null,
        error = error,
        fdmConnected = fdmConnected,
        fiscalEventId = fiscalEventId,
        delivery = delivery,
        kitchenSlip = slip,
    )

internal val mailOff = MailStatus(configured = false, reachable = false, allowDigitalOnly = false)

/** The checkout entry point over the given real stores. */
@Suppress("LongParameterList")
internal class TestCheckoutEntry(
    private val gateway: EngineGateway,
    private val helpers: EngineHelpers,
    private val order: OrderStore,
    private val session: SessionStore,
    private val delivery: DeliveryStore,
    private val settings: PosSettingsStore,
    private val errors: ErrorPresenter,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
    private val amounts: CartAmounts,
) : CheckoutEntryPoint {
    override fun gateway() = gateway

    override fun helpers() = helpers

    override fun order() = order

    override fun session() = session

    override fun delivery() = delivery

    override fun settings() = settings

    override fun errors() = errors

    override fun toasts() = toasts

    override fun catalog() = catalog

    override fun amounts() = amounts
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

/** A started checkout view model with the real stores it drives, built on one engine fixture. */
@Suppress("LongParameterList")
internal class CheckoutRig(
    val f: EngineFixture,
    val model: CheckoutViewModel,
    val session: SessionStore,
    val order: OrderStore,
    val delivery: DeliveryStore,
    val toasts: ToastStore,
    val catalog: MessageCatalog,
    val errorTexts: ErrorTexts,
    val natives: CheckoutNatives,
    val effects: List<CheckoutEffect>,
    private val collector: Job,
) {
    val state get() = model.state.value

    /** Logs [user] in through the engine. */
    suspend fun login(user: User = cashier) {
        coEvery { f.engine.login(user.insz, any()) } returns user
        session.login(user.insz, "1234")
    }

    fun stop() = collector.cancel()
}

/** An engine fixture under [root] whose reads answer for an order of [totalCents] with [lines]. */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun TestScope.checkoutFixture(
    root: File,
    totalCents: Long,
    lines: List<CartLine> = listOf(cartLine(1, quantity = 2)),
    mail: MailStatus = mailOff,
): EngineFixture {
    val f = EngineFixture(this, root)
    Dispatchers.setMain(f.dispatcher)
    coEvery { f.engine.getSettings() } returns posSettings()
    coEvery { f.engine.getOrder(ORDER_ID) } returns checkoutOrder(totalCents)
    coEvery { f.engine.listCartLines(ORDER_ID) } returns lines
    coEvery { f.engine.setActiveOrder(any()) } returns Unit
    coEvery { f.engine.checkMailStatus() } returns mail
    return f
}

/** Builds the view model over real stores on [f]; the engine is opened and [user] logged in first unless told not to. */
@OptIn(ExperimentalCoroutinesApi::class)
internal suspend fun TestScope.startCheckout(
    f: EngineFixture,
    context: Context,
    open: Boolean = true,
    user: User? = cashier,
    orderId: Long = ORDER_ID,
): CheckoutRig {
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errorTexts = ErrorTexts(catalog, FdmTexts(catalog))
    val errors = ErrorPresenter(toasts, errorTexts)
    val session = SessionStore(gateway, f.events, backgroundScope)
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val order = OrderStore(gateway, f.events, backgroundScope)
    val delivery = DeliveryStore(gateway, errors, settings)
    val natives = CheckoutNatives()
    val helpers = EngineHelpers(natives, f.dispatcher)
    if (open) {
        f.holder.open()
        runCurrent()
    }
    if (user != null) {
        coEvery { f.engine.login(user.insz, any()) } returns user
        session.login(user.insz, "1234")
    }
    val entry =
        TestCheckoutEntry(gateway, helpers, order, session, delivery, settings, errors, toasts, catalog, CartAmounts(helpers))
    val model = CheckoutViewModel(entry, orderId)
    val seen = mutableListOf<CheckoutEffect>()
    val collector = CoroutineScope(UnconfinedTestDispatcher(testScheduler)).launch { model.effects.collect { seen += it } }
    runCurrent()
    return CheckoutRig(f, model, session, order, delivery, toasts, catalog, errorTexts, natives, seen, collector)
}

/** A row of the VAT summary, for the content tests. */
internal fun vatRow(
    vat: VatCode,
    rate: Long,
    vatCents: Long,
): VatSplitRow = VatSplitRow(vat, rate, 0L, vatCents)
