package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
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

/** Goldens of the invoice flow at 1920 x 1080 dp: the tickets tab, the customer step, the preview and the result. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class InvoiceScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.REPORTS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)
    private val base = ReportsState(signedIn = true, tab = ReportsTab.TICKETS)
    private val sales = listOf(invoiceableSale(12L), invoiceableSale(11L, 2_400L), invoiceableSale(10L, 900L, invoicedBy = "20260001"))
    private val copyables = listOf(copyable(12L, totalCents = 1_500L), copyable(11L, totalCents = 2_400L), copyable(10L, totalCents = 900L))
    private val tickets = TicketsState(loaded = true, copyable = copyables, rows = mergeRows(copyables, emptyList()))
    private val acme = CustomerForm(null, "BE0308357159", "ACME SPRL", "RUE DU PONT 1", "4000", "LIEGE", "BE")

    private fun capture(
        invoices: InvoicesState,
        name: String,
    ) {
        val state = base.copy(tickets = tickets)
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
                        overlay = { ReportsOverlay(state, ReportsDelivery(), RefundUi(), ReportsCallbacks(), invoices) },
                    ) { ReportsContent(state, ReportsCallbacks(), invoices = invoices) }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/reports/$name.png")
    }

    private fun flowState(
        step: InvoiceStep,
        change: InvoiceFlow.() -> InvoiceFlow = { this },
    ) = InvoicesState(
        sales = sales,
        selected = listOf(11L, 12L),
        flow = InvoiceFlow(listOf(11L, 12L), step = step).change(),
        canCopy = true,
    )

    @Test
    fun ticketsWithInvoices() {
        val invoices =
            listOf(
                invoiceSummary(3L, "queued"),
                invoiceSummary(2L, "accepted", name = "BETA SA"),
                invoiceSummary(1L, "failed", name = "GAMMA GMBH"),
            )
        capture(InvoicesState(sales = sales, selected = listOf(11L, 12L), invoices = invoices, canCopy = true), "reports_invoice_tickets")
    }

    @Test
    fun customerStep() {
        capture(
            flowState(InvoiceStep.CUSTOMER) {
                copy(
                    form = acme,
                    lookup = lookup(PEPPOL_UNKNOWN),
                    search = "AC",
                    searchResults = listOf(customer(), customer(8L, "BE0403019261", "ACIERIES SA")),
                )
            },
            "reports_invoice_customer",
        )
    }

    @Test
    fun customerStepNotOnPeppol() {
        capture(
            flowState(InvoiceStep.CUSTOMER) {
                copy(form = acme, lookup = lookup(PEPPOL_NOT_REGISTERED))
            },
            "reports_invoice_customer_not_registered",
        )
    }

    @Test
    fun preview() {
        capture(
            flowState(InvoiceStep.PREVIEW) { copy(form = acme, customer = customer(), lookup = lookup(), preview = invoicePreview()) },
            "reports_invoice_preview",
        )
    }

    @Test
    fun result() {
        capture(
            flowState(InvoiceStep.RESULT) { copy(form = acme, customer = customer(), result = invoiceResult()) },
            "reports_invoice_result",
        )
    }
}
