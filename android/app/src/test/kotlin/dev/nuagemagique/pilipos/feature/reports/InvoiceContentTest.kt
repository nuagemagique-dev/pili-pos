package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@Suppress("TooManyFunctions", "LargeClass")
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class InvoiceContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val invoiceCallbacks =
        InvoiceCallbacks(
            onToggleSale = { calls += "toggle:$it" },
            onOpen = { calls += "open" },
            onClose = { calls += "close" },
            onSearch = { calls += "search:$it" },
            onPickCustomer = { calls += "pick:${it.id}" },
            onVatNumber = { calls += "vat:$it" },
            onCheckVat = { calls += "check" },
            onName = { calls += "name:$it" },
            onStreet = { calls += "street:$it" },
            onPostalCode = { calls += "postal:$it" },
            onCity = { calls += "city:$it" },
            onCountryCode = { calls += "country:$it" },
            onContinue = { calls += "continue" },
            onBack = { calls += "back" },
            onConfirm = { calls += "confirm" },
            onCopy = { calls += "copy:$it" },
        )

    private val callbacks = ReportsCallbacks(invoice = invoiceCallbacks)

    private val base = ReportsState(signedIn = true, tab = ReportsTab.TICKETS)
    private val sales = listOf(invoiceableSale(12L), invoiceableSale(11L, 2_400L), invoiceableSale(10L, 900L, invoicedBy = "20260001"))
    private val tickets =
        TicketsState(
            loaded = true,
            copyable = listOf(copyable(12L), copyable(11L), copyable(10L)),
            rows = mergeRows(listOf(copyable(12L), copyable(11L), copyable(10L)), emptyList()),
        )

    private fun show(
        invoices: InvoicesState,
        state: ReportsState = base.copy(tickets = tickets),
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) {
                        ReportsContent(state, callbacks, invoices = invoices)
                        ReportsOverlay(state, ReportsDelivery(), RefundUi(), callbacks, invoices)
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    private fun flow(
        step: InvoiceStep = InvoiceStep.CUSTOMER,
        change: InvoiceFlow.() -> InvoiceFlow = { this },
    ) = InvoicesState(sales = sales, selected = listOf(11L, 12L), flow = InvoiceFlow(listOf(11L, 12L), step = step).change())

    private val filledForm = CustomerForm(null, "BE0308357159", "ACME SPRL", "RUE DU PONT 1", "4000", "LIEGE", "BE")

    // ── Tickets tab ───────────────────────────────────────────────

    @Test
    fun theFactureActionNeedsATickedSale() {
        show(InvoicesState(sales = sales))

        compose.onNodeWithTag("invoice_action").assertTextEquals("Facture").assertIsNotEnabled()
    }

    @Test
    fun theFactureActionCountsTheTickedSalesAndOpensTheDialog() {
        show(InvoicesState(sales = sales, selected = listOf(11L, 12L)))

        compose.onNodeWithTag("invoice_action").assertTextEquals("Facture (2)").assertIsEnabled().performClick()

        assertEquals(listOf("open"), calls)
    }

    @Test
    fun aBoxTicksASaleAndAnInvoicedSaleShowsItsInvoiceInstead() {
        show(InvoicesState(sales = sales, selected = listOf(11L)))

        compose.onNodeWithTag("ticket_check_12").assertIsDisplayed().performClick()
        compose.onNodeWithTag("ticket_check_11").assertIsDisplayed()
        compose.onNodeWithTag("ticket_check_10").assertDoesNotExist()
        compose.onNodeWithTag("ticket_invoiced_10").assertTextEquals("Facture 20260001")
        assertEquals(listOf("toggle:12"), calls)
    }

    @Test
    fun aTicketThatCannotBeInvoicedHasNoBox() {
        show(InvoicesState(sales = listOf(invoiceableSale(11L))))

        compose.onNodeWithTag("ticket_check_11").assertIsDisplayed()
        compose.onNodeWithTag("ticket_check_12").assertDoesNotExist()
    }

    @Test
    fun theInvoiceListShowsEveryInvoiceWithItsPeppolState() {
        val invoices =
            listOf(invoiceSummary(3L, "queued"), invoiceSummary(2L, "accepted"), invoiceSummary(1L, "failed"), invoiceSummary(0L, null))
        show(InvoicesState(sales = sales, invoices = invoices, canCopy = true))

        compose.onNodeWithTag("invoice_list").assertIsDisplayed()
        compose.onNodeWithTag("invoice_no_3").assertTextEquals("20260003")
        compose.onNodeWithTag("invoice_date_3").assertTextEquals("17/10/2026")
        compose.onNodeWithTag("invoice_amount_3").assertTextContains("39,00", substring = true)
        compose.onNodeWithTag("invoice_status_3").assertTextEquals("Peppol : en attente")
        compose.onNodeWithTag("invoice_status_2").assertTextEquals("Peppol : remise")
        compose.onNodeWithTag("invoice_status_1").assertTextEquals("Peppol : bloquée")
        compose.onNodeWithTag("invoice_status_0").assertTextEquals("Papier seulement")
    }

    @Test
    fun noInvoiceListWhileThereIsNoInvoice() {
        show(InvoicesState(sales = sales))

        compose.onNodeWithTag("invoice_list").assertDoesNotExist()
    }

    @Test
    fun copieCopiesTheInvoiceEventAndIsAbsentForACashier() {
        show(InvoicesState(invoices = listOf(invoiceSummary(3L)), canCopy = true))
        compose.onNodeWithTag("invoice_copy_3").assertIsEnabled().performClick()
        assertEquals(listOf("copy:43"), calls)
    }

    @Test
    fun aCashierGetsNoCopieButton() {
        show(InvoicesState(invoices = listOf(invoiceSummary(3L)), canCopy = false))

        compose.onNodeWithTag("invoice_row_3").assertIsDisplayed()
        compose.onNodeWithTag("invoice_copy_3").assertDoesNotExist()
    }

    @Test
    fun copieIsDisabledWhileACopyIsBeingSigned() {
        show(InvoicesState(invoices = listOf(invoiceSummary(3L)), canCopy = true, copying = 43L))

        compose.onNodeWithTag("invoice_copy_3").assertIsNotEnabled()
    }

    // ── Customer step ─────────────────────────────────────────────

    @Test
    fun theCustomerStepShowsTheStepsTheSalesAndDisablesContinue() {
        show(flow())

        compose.onNodeWithTag("invoice_dialog").assertIsDisplayed()
        compose.onNodeWithTag("invoice_sales").assertTextEquals("2 vente(s) · 39,00 €")
        compose.onNodeWithTag("invoice_continue").assertIsNotEnabled()
        compose.onNodeWithTag("invoice_vat_check").assertIsNotEnabled()
        compose.onNodeWithTag("invoice_chip").assertDoesNotExist()
    }

    @Test
    fun theFieldsCallBackWithWhatIsTyped() {
        show(flow())

        compose.onNodeWithTag("invoice_vat_field").performTextInput("BE1")
        compose.onNodeWithTag("invoice_search_field").performTextInput("AC")
        compose.onNodeWithTag("invoice_field_name_input").performTextInput("ACME")
        compose.onNodeWithTag("invoice_field_street_input").performTextInput("RUE")
        compose.onNodeWithTag("invoice_field_postalCode_input").performTextInput("4000")
        compose.onNodeWithTag("invoice_field_city_input").performTextInput("LIEGE")
        compose.onNodeWithTag("invoice_field_countryCode_input").performTextInput("BE")

        assertEquals(
            listOf("vat:BE1", "search:AC", "name:ACME", "street:RUE", "postal:4000", "city:LIEGE", "country:BE"),
            // The controlled fields echo an empty value once after each input.
            calls.filterNot { it.endsWith(":") },
        )
    }

    @Test
    fun theCheckButtonAsksTheEngineAndTheChipShowsTheThreeStates() {
        show(flow { copy(form = filledForm, lookup = lookup(PEPPOL_REGISTERED)) })
        compose.onNodeWithTag("invoice_vat_check").assertIsEnabled().performClick()
        compose.onNodeWithTag("invoice_chip").assertTextEquals("Inscrit sur Peppol")
        assertEquals(listOf("check"), calls)
    }

    @Test
    fun anUnregisteredCustomerIsToldBeforeSigning() {
        show(flow { copy(form = filledForm, lookup = lookup(PEPPOL_NOT_REGISTERED)) })

        compose.onNodeWithTag("invoice_chip").assertTextEquals("Non inscrit sur Peppol")
        compose.onNodeWithTag("invoice_chip_note").assertTextContains("ne peut pas recevoir la facture par Peppol", substring = true)
    }

    @Test
    fun anOfflineLookupIsUnknownAndNeverBlocksContinue() {
        show(flow { copy(form = filledForm, lookup = lookup(PEPPOL_UNKNOWN)) })

        compose.onNodeWithTag("invoice_chip").assertTextEquals("Peppol inconnu (hors ligne)")
        compose.onNodeWithTag("invoice_chip_note").assertTextContains("l'envoi sera retenté", substring = true)
        compose.onNodeWithTag("invoice_continue").assertIsEnabled().performClick()
        assertEquals(listOf("continue"), calls)
    }

    @Test
    fun aForeignCustomerIsPrintedOnly() {
        show(flow { copy(form = filledForm, lookup = lookup(PEPPOL_UNKNOWN, vat = "FR12345678901", country = "FR")) })

        compose.onNodeWithTag("invoice_chip_note").assertTextContains("Client hors Belgique", substring = true)
    }

    @Test
    fun theBusyCheckShowsItsProgressCaption() {
        show(flow { copy(form = filledForm, lookupBusy = true) })

        compose.onNodeWithTag("invoice_vat_check").assertTextEquals("Vérification…").assertIsNotEnabled()
        compose.onNodeWithTag("invoice_continue").assertIsNotEnabled()
    }

    @Test
    fun theSearchResultsAreListedAndPickable() {
        show(flow { copy(search = "AC", searchResults = listOf(customer(), customer(8L, "BE0403019261", "ACIERIES SA"))) })

        compose.onNodeWithTag("invoice_search_results").assertIsDisplayed()
        compose.onNodeWithTag("invoice_search_result_8").performScrollTo().performClick()
        assertEquals(listOf("pick:8"), calls)
    }

    @Test
    fun noKnownCustomerSaysSo() {
        show(flow { copy(search = "ZZ") })

        compose.onNodeWithTag("invoice_no_match").assertTextEquals("Aucun client connu.")
    }

    @Test
    fun theCustomerErrorIsShown() {
        show(flow { copy(form = filledForm, customerError = "Numéro de TVA invalide. Vérifiez les chiffres.") })

        compose.onNodeWithTag("invoice_error").performScrollTo().assertTextContains("Numéro de TVA invalide", substring = true)
    }

    @Test
    fun backClosesTheCustomerStep() {
        show(flow())

        compose.onNodeWithTag("invoice_cancel").performClick()

        assertEquals(listOf("close"), calls)
    }

    // ── Preview step ──────────────────────────────────────────────

    private fun previewFlow(problems: List<String> = emptyList()) =
        flow(InvoiceStep.PREVIEW) {
            copy(form = filledForm, customer = customer(), lookup = lookup(), preview = invoicePreview(problems), problems = problems)
        }

    @Test
    fun thePreviewShowsTheLinesTheVatTheTotalAndTheAlreadyPaidMention() {
        show(previewFlow())

        compose.onNodeWithTag("invoice_preview").assertIsDisplayed()
        compose.onNodeWithTag("invoice_line_0").assertTextContains("Maki saumon x8", substring = true)
        compose.onNodeWithTag("invoice_line_0").assertTextContains("2x", substring = true)
        compose.onNodeWithTag("invoice_line_1").assertTextContains("Coca-Cola", substring = true)
        compose.onNodeWithTag("invoice_vat_A").assertTextContains("21 %", substring = true)
        compose.onNodeWithTag("invoice_vat_B").assertTextContains("12 %", substring = true)
        compose.onNodeWithTag("invoice_total").assertTextContains("39,00", substring = true)
        compose.onNodeWithTag("invoice_already_paid").assertTextContains("Déjà payé : 39,00 €", substring = true)
        compose.onNodeWithTag("invoice_customer").assertTextContains("ACME SPRL", substring = true)
        compose.onNodeWithTag("invoice_sale_11").assertTextContains("FDM0000001", substring = true)
        compose.onNodeWithTag("invoice_chip").assertTextEquals("Inscrit sur Peppol")
    }

    @Test
    fun theConfirmAndBackButtonsCallBack() {
        show(previewFlow())

        compose.onNodeWithTag("invoice_confirm").assertIsEnabled().performClick()
        compose.onNodeWithTag("invoice_back").performClick()

        assertEquals(listOf("confirm", "back"), calls)
    }

    @Test
    fun aProblemBlocksTheConfirmation() {
        show(previewFlow(problems = listOf("La vente 11 figure déjà sur la facture 20260001.")))

        compose.onNodeWithTag("invoice_problem_0").assertTextContains("figure déjà", substring = true)
        compose.onNodeWithTag("invoice_confirm").assertIsNotEnabled()
    }

    // ── Signing and result ────────────────────────────────────────

    @Test
    fun theSignatureInProgressShowsTheFdmText() {
        show(flow(InvoiceStep.PROCESSING))

        compose.onNodeWithTag("invoice_processing").assertIsDisplayed()
    }

    @Test
    fun aSignedInvoiceShowsItsNumberThePrintAndThePeppolQueue() {
        show(flow(InvoiceStep.RESULT) { copy(result = invoiceResult()) })

        compose.onNodeWithTag("invoice_result_title").assertTextEquals("Facture 20260002 établie")
        compose.onNodeWithTag("invoice_result_printed").assertTextEquals("Facture imprimée.")
        compose.onNodeWithTag("invoice_result_delivery").assertTextEquals("Envoi Peppol en file d'attente.")
        compose.onNodeWithTag("invoice_finish").performClick()
        assertEquals(listOf("close"), calls)
    }

    @Test
    fun anInvoiceThatWasNotPrintedOrDeliveredSaysSo() {
        show(flow(InvoiceStep.RESULT) { copy(result = invoiceResult(printed = false, delivery = null)) })

        compose.onNodeWithTag("invoice_result_printed").assertTextContains("pas imprimée", substring = true)
        compose.onNodeWithTag("invoice_result_delivery").assertTextEquals("Pas d'envoi Peppol pour cette facture.")
    }

    @Test
    fun aRefusedInvoiceShowsTheEngineError() {
        show(
            flow(InvoiceStep.RESULT) {
                copy(
                    result = invoiceResult(success = false, error = "invoice_training_mode"),
                    resultError = "Pas de facture en mode formation.",
                )
            },
        )

        compose.onNodeWithTag("invoice_result_title").assertTextEquals("La facture n'a pas été établie.")
        compose.onNodeWithTag("invoice_error").assertTextEquals("Pas de facture en mode formation.")
    }
}
