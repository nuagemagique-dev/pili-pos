package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.scaffold.DeviceLights
import dev.nuagemagique.pilipos.ui.scaffold.FakeProbe
import dev.nuagemagique.pilipos.ui.scaffold.HeaderActions
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.HeaderUiState
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffoldContent
import dev.nuagemagique.pilipos.ui.scaffold.StatusLights
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.CorrectionDone
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

/**
 * Goldens of the reports screen at 1920 x 1080 dp: the day tab, the cash tab, the tickets tab (for a user and for a
 * manager), the refund confirmation and the payment correction.
 */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class ReportsScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.REPORTS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)
    private val base = ReportsState(signedIn = true)

    private fun capture(
        state: ReportsState,
        name: String,
        delivery: ReportsDelivery = ReportsDelivery(),
        refund: RefundUi = RefundUi(),
    ) {
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    PosScaffoldContent(
                        header = header,
                        actions = HeaderActions(),
                        fdm = FdmUiState(connected = true),
                        onCheckFdm = {},
                        training = false,
                        clock = "12:30",
                        lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = false) },
                        overlay = { ReportsOverlay(state, delivery, refund, ReportsCallbacks()) },
                    ) { ReportsContent(state, ReportsCallbacks()) }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/reports/$name.png")
    }

    @Test
    fun day() {
        val last = ReportOutcome(ReportKind.X, reportResult())
        val day = DayState(zHistory = listOf(zSummary(5L), zSummary(4L)), zPending = true, lastReport = last)
        capture(base.copy(day = day), "reports_day")
    }

    @Test
    fun cash() {
        val cash = CashState(direction = CashDirection.OUT, reason = "retrait", amountCents = 1_250L)
        capture(base.copy(tab = ReportsTab.CASH, cash = cash), "reports_cash")
    }

    @Test
    fun tickets() {
        val copyables =
            listOf(
                copyable(5L, totalCents = -850L),
                copyable(4L, label = "T", totalCents = 2_400L),
                copyable(3L),
                copyable(2L, label = "P"),
            )
        val refundables = listOf(refundable(4L), refundable(3L), refundable(1L))
        val tickets = TicketsState(loaded = true, copyable = copyables, refundable = refundables, rows = mergeRows(copyables, refundables))
        capture(base.copy(tab = ReportsTab.TICKETS, tickets = tickets), "reports_tickets")
    }

    @Test
    fun refundConfirmation() {
        val lines = listOf(orderLine(1L, 3), orderLine(2L, 1))
        val flow = RefundFlow(RefundStep.DELIVERY, refundable(7L), lines, mapOf(1L to 2))
        val amounts = mapOf<Long, LineAmount>(1L to LineAmount.Total(1_500L), 2L to LineAmount.Total(500L))
        val preview =
            RefundPreview(
                lines = listOf(PreviewLine(RefundSelection(lines[0], 2), LineAmount.Total(1_000L))),
                vatRows = listOf(VatSplitRow(VatCode.B, 12L, 1_000L, 107L)),
                totalCents = 1_000L,
                cashCents = 1_000L,
            )
        val tickets = TicketsState(loaded = true, refundable = listOf(refundable(7L)))
        val state = base.copy(tab = ReportsTab.TICKETS, tickets = tickets, refundFlow = flow)
        capture(state, "reports_refund", ReportsDelivery(MailStatus(true, true, true), canPrintMissing = true), RefundUi(amounts, preview))
    }

    @Test
    fun ticketsForAManager() {
        val copyables = listOf(copyable(4L, label = "T", totalCents = 2_400L), copyable(3L), copyable(2L))
        val refundables = listOf(refundable(4L), refundable(3L))
        val done = CorrectionDone("CASH", "CARD_DEBIT", 1_000L, "2026-10-09T13:00:00")
        val tickets =
            TicketsState(
                loaded = true,
                copyable = copyables,
                refundable = refundables,
                rows = mergeRows(copyables, refundables),
                canCorrect = true,
                corrections = mapOf(3L to done),
            )
        capture(base.copy(tab = ReportsTab.TICKETS, tickets = tickets), "reports_tickets_manager")
    }

    @Test
    fun paymentCorrection() {
        val payments = salePayments(5L, listOf(paymentLine(0, "CASH", 700L), paymentLine(1, "CARD_DEBIT", 300L, brand = "Bancontact")))
        val flow = CorrectionFlow(5L, 5L, payments, lineIndex = 0, method = PaymentMethod.CARD)
        val tickets = TicketsState(loaded = true, canCorrect = true)
        capture(base.copy(tab = ReportsTab.TICKETS, tickets = tickets, correctionFlow = flow), "reports_payment_correction")
    }
}
