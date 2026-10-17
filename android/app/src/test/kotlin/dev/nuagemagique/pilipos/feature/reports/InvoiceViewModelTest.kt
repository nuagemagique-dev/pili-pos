package dev.nuagemagique.pilipos.feature.reports

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.B2bCustomerInput
import uniffi.pos_ffi.PosException

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class InvoiceViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    private val sales = listOf(invoiceableSale(12L), invoiceableSale(11L, 2_400L), invoiceableSale(10L, 900L, invoicedBy = "20260001"))

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(): EngineFixture = reportsFixture(tmp.root)

    private suspend fun TestScope.start(
        f: EngineFixture,
        open: Boolean = true,
    ): ReportsRig = startReports(f, context, open)

    /** A rig whose sales are loaded and 11 and 12 are ticked. */
    private suspend fun TestScope.ticked(f: EngineFixture): ReportsRig {
        coEvery { f.engine.listInvoiceableSales(any(), any()) } returns sales
        val r = start(f)
        r.invoiceModel.load()
        runCurrent()
        r.invoiceModel.toggleSale(11L)
        r.invoiceModel.toggleSale(12L)
        return r
    }

    /** A rig with the dialog open and the form filled in. */
    private suspend fun TestScope.filled(f: EngineFixture): ReportsRig {
        val r = ticked(f)
        r.invoiceModel.openFlow()
        r.invoiceModel.setVatNumber("be0308357159")
        r.invoiceModel.setName("acme sprl")
        r.invoiceModel.setStreet("rue du pont 1")
        r.invoiceModel.setPostalCode("4000")
        r.invoiceModel.setCity("liege")
        runCurrent()
        return r
    }

    // ── Lists and selection ───────────────────────────────────────

    @Test
    fun loadReadsTheInvoiceableSalesAndTheInvoices() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listInvoiceableSales(any(), any()) } returns sales
            coEvery { f.engine.listInvoices(any()) } returns listOf(invoiceSummary(2L), invoiceSummary(1L))
            val r = start(f)

            r.invoiceModel.load()
            runCurrent()

            assertEquals(listOf(12L, 11L, 10L), r.invoices.sales.map { it.fiscalEventId })
            assertEquals(listOf(2L, 1L), r.invoices.invoices.map { it.id })
            coVerify { f.engine.listInvoiceableSales(50L, null) }
            coVerify { f.engine.listInvoices(30L) }
        }

    @Test
    fun aFailedReadGivesEmptyListsAndNoCrash() =
        runTest {
            val f = fixture()
            val r = start(f, open = false)

            r.invoiceModel.load()
            runCurrent()

            assertTrue(r.invoices.sales.isEmpty())
            assertTrue(r.invoices.invoices.isEmpty())
        }

    @Test
    fun aSaleIsTickedAndUntickedAndAnInvoicedOneDropsOutOnReload() =
        runTest {
            val f = fixture()
            val r = ticked(f)
            assertEquals(listOf(11L, 12L), r.invoices.selected)

            r.invoiceModel.toggleSale(11L)
            assertEquals(listOf(12L), r.invoices.selected)

            // Sale 12 got invoiced elsewhere meanwhile: it cannot stay ticked.
            coEvery { f.engine.listInvoiceableSales(any(), any()) } returns listOf(invoiceableSale(12L, invoicedBy = "20260002"))
            r.invoiceModel.load()
            runCurrent()
            assertTrue(r.invoices.selected.isEmpty())
        }

    @Test
    fun theDialogDoesNotOpenWithNothingTicked() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.invoiceModel.openFlow()

            assertNull(r.invoices.flow)
        }

    @Test
    fun theDialogOpensOnTheCustomerStepWithTheTickedSales() =
        runTest {
            val f = fixture()
            val r = ticked(f)

            r.invoiceModel.openFlow()

            val flow = r.invoices.flow
            assertNotNull(flow)
            assertEquals(InvoiceStep.CUSTOMER, flow?.step)
            assertEquals(listOf(11L, 12L), flow?.saleEventIds)
            assertFalse(flow?.canContinue ?: true)
        }

    // ── Customer step ─────────────────────────────────────────────

    @Test
    fun theFieldsAreKeptInCapitalsAndCut() =
        runTest {
            val f = fixture()
            val r = ticked(f)
            r.invoiceModel.openFlow()

            r.invoiceModel.setVatNumber("be0308357159xxxxxxxxxxxx")
            r.invoiceModel.setName("café du pont")
            r.invoiceModel.setCountryCode("belgique")
            r.invoiceModel.setPostalCode("40000000000000")

            val form = r.invoices.flow?.form
            assertEquals("BE0308357159XXXX", form?.vatNumber)
            assertEquals("CAFÉ DU PONT", form?.name)
            assertEquals("BE", form?.countryCode)
            assertEquals("4000000000", form?.postalCode)
        }

    @Test
    fun theContinueButtonNeedsAllTheRequiredFields() =
        runTest {
            val f = fixture()
            val r = ticked(f)
            r.invoiceModel.openFlow()
            r.invoiceModel.setVatNumber("BE0308357159")
            r.invoiceModel.setName("ACME")
            r.invoiceModel.setStreet("RUE 1")
            r.invoiceModel.setPostalCode("4000")
            assertFalse(r.invoices.flow?.canContinue ?: true)

            r.invoiceModel.setCity("LIEGE")

            assertTrue(r.invoices.flow?.canContinue ?: false)
        }

    @Test
    fun theCheckAsksTheEngineAndShowsTheRegisteredChip() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup(PEPPOL_REGISTERED)
            val r = ticked(f)
            r.invoiceModel.openFlow()
            r.invoiceModel.setVatNumber("BE0308357159")

            r.invoiceModel.checkVat()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(PEPPOL_REGISTERED, flow?.peppolStatus)
            assertFalse(flow?.lookupBusy ?: true)
            assertNull(flow?.customerError)
            assertEquals("BE", flow?.form?.countryCode)
            coVerify { f.engine.lookupB2bCustomer("BE0308357159") }
        }

    @Test
    fun aKnownCustomerFillsTheFormAfterTheCheck() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup(known = customer())
            val r = ticked(f)
            r.invoiceModel.openFlow()
            r.invoiceModel.setVatNumber("BE0308357159")

            r.invoiceModel.checkVat()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(7L, flow?.form?.id)
            assertEquals("ACME SPRL", flow?.form?.name)
            assertEquals("LIEGE", flow?.form?.city)
            assertEquals(7L, flow?.customer?.id)
        }

    @Test
    fun anOfflineLookupIsUnknownAndTheInvoiceStaysPossible() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup(PEPPOL_UNKNOWN)
            val r = filled(f)

            r.invoiceModel.checkVat()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(PEPPOL_UNKNOWN, flow?.peppolStatus)
            assertTrue(flow?.canContinue ?: false)
        }

    @Test
    fun aWrongVatNumberShowsTheEngineRefusalAndKeepsTheStep() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } throws PosException.Coded("customer_vat_invalid", emptyMap())
            val r = filled(f)

            r.invoiceModel.checkVat()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(r.errorText("customer_vat_invalid"), flow?.customerError)
            assertNull(flow?.lookup)
            assertEquals(InvoiceStep.CUSTOMER, flow?.step)
        }

    @Test
    fun aChangedVatNumberForgetsTheLookupAndTheKnownCustomer() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup(known = customer())
            val r = ticked(f)
            r.invoiceModel.openFlow()
            r.invoiceModel.setVatNumber("BE0308357159")
            r.invoiceModel.checkVat()
            runCurrent()

            r.invoiceModel.setVatNumber("BE0403019261")

            val flow = r.invoices.flow
            assertNull(flow?.lookup)
            assertNull(flow?.customer)
            assertNull(flow?.form?.id)
            assertEquals("ACME SPRL", flow?.form?.name)
        }

    @Test
    fun theSearchFindsKnownCustomersAndAnEmptySearchClears() =
        runTest {
            val f = fixture()
            coEvery { f.engine.searchB2bCustomers(any()) } returns listOf(customer())
            val r = ticked(f)
            r.invoiceModel.openFlow()

            r.invoiceModel.setSearch("ac")
            runCurrent()
            assertEquals(listOf(7L), r.invoices.flow?.searchResults?.map { it.id })
            assertEquals("AC", r.invoices.flow?.search)
            coVerify { f.engine.searchB2bCustomers("AC") }

            r.invoiceModel.setSearch("")
            runCurrent()
            assertTrue(r.invoices.flow?.searchResults?.isEmpty() ?: false)
        }

    @Test
    fun pickingACustomerFillsTheFormAndClearsTheSearch() =
        runTest {
            val f = fixture()
            coEvery { f.engine.searchB2bCustomers(any()) } returns listOf(customer())
            val r = ticked(f)
            r.invoiceModel.openFlow()
            r.invoiceModel.setSearch("ac")
            runCurrent()

            r.invoiceModel.pickCustomer(customer())

            val flow = r.invoices.flow
            assertEquals("BE0308357159", flow?.form?.vatNumber)
            assertEquals(7L, flow?.form?.id)
            assertEquals("", flow?.search)
            assertTrue(flow?.searchResults?.isEmpty() ?: false)
            assertTrue(flow?.canContinue ?: false)
        }

    // ── Continue, preview ─────────────────────────────────────────

    @Test
    fun continueChecksTheNumberSavesTheCustomerAndShowsThePreview() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup()
            coEvery { f.engine.saveB2bCustomer(any()) } returns customer()
            coEvery { f.engine.invoicePreview(any()) } returns invoicePreview()
            val r = filled(f)

            r.invoiceModel.continueToPreview()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(InvoiceStep.PREVIEW, flow?.step)
            assertEquals(7L, flow?.customer?.id)
            assertEquals(2, flow?.preview?.lines?.size)
            assertTrue(flow?.problems?.isEmpty() ?: false)
            assertFalse(flow?.busy ?: true)
            coVerify {
                f.engine.saveB2bCustomer(
                    B2bCustomerInput(
                        id = null,
                        vatNumber = "BE0308357159",
                        name = "ACME SPRL",
                        street = "RUE DU PONT 1",
                        postalCode = "4000",
                        city = "LIEGE",
                        countryCode = "BE",
                    ),
                )
            }
            coVerify { f.engine.invoicePreview(listOf(11L, 12L)) }
        }

    @Test
    fun continueAfterAnExplicitCheckDoesNotAskPeppolAgain() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup()
            coEvery { f.engine.saveB2bCustomer(any()) } returns customer()
            coEvery { f.engine.invoicePreview(any()) } returns invoicePreview()
            val r = filled(f)
            r.invoiceModel.checkVat()
            runCurrent()

            r.invoiceModel.continueToPreview()
            runCurrent()

            coVerify(exactly = 1) { f.engine.lookupB2bCustomer(any()) }
            assertEquals(InvoiceStep.PREVIEW, r.invoices.flow?.step)
        }

    @Test
    fun aBadNumberAtContinueStopsBeforeSaving() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } throws PosException.Coded("customer_vat_invalid", emptyMap())
            val r = filled(f)

            r.invoiceModel.continueToPreview()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(InvoiceStep.CUSTOMER, flow?.step)
            assertEquals(r.errorText("customer_vat_invalid"), flow?.customerError)
            assertFalse(flow?.busy ?: true)
            coVerify(exactly = 0) { f.engine.saveB2bCustomer(any()) }
        }

    @Test
    fun aRefusedCustomerSaveShowsTheErrorOnTheCustomerStep() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup()
            coEvery { f.engine.saveB2bCustomer(any()) } throws PosException.Coded("customer_incomplete", mapOf("field" to "street"))
            val r = filled(f)

            r.invoiceModel.continueToPreview()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(InvoiceStep.CUSTOMER, flow?.step)
            assertNotNull(flow?.customerError)
            assertFalse(flow?.busy ?: true)
        }

    @Test
    fun theProblemsOfThePreviewBlockTheConfirmation() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup()
            coEvery { f.engine.saveB2bCustomer(any()) } returns customer()
            val raw = "invoice_sale_already_invoiced|event=11|invoice=20260001"
            coEvery { f.engine.invoicePreview(any()) } returns invoicePreview(problems = listOf(raw))
            val r = filled(f)
            r.invoiceModel.continueToPreview()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(listOf(r.errorText(raw)), flow?.problems)
            assertFalse(flow?.canConfirm ?: true)

            r.invoiceModel.confirm()
            runCurrent()
            coVerify(exactly = 0) { f.engine.signInvoiceFiscal(any(), any(), any()) }
        }

    @Test
    fun backReturnsToTheCustomerStep() =
        runTest {
            val f = fixture()
            coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup()
            coEvery { f.engine.saveB2bCustomer(any()) } returns customer()
            coEvery { f.engine.invoicePreview(any()) } returns invoicePreview()
            val r = filled(f)
            r.invoiceModel.continueToPreview()
            runCurrent()

            r.invoiceModel.backToCustomer()

            assertEquals(InvoiceStep.CUSTOMER, r.invoices.flow?.step)
        }

    // ── Signing ───────────────────────────────────────────────────

    private suspend fun TestScope.atPreview(f: EngineFixture): ReportsRig {
        coEvery { f.engine.lookupB2bCustomer(any()) } returns lookup()
        coEvery { f.engine.saveB2bCustomer(any()) } returns customer()
        coEvery { f.engine.invoicePreview(any()) } returns invoicePreview()
        val r = filled(f)
        r.invoiceModel.continueToPreview()
        runCurrent()
        return r
    }

    @Test
    fun confirmSignsTheInvoiceOfTheSalesForTheSavedCustomer() =
        runTest {
            val f = fixture()
            coEvery { f.engine.signInvoiceFiscal(any(), any(), any()) } returns invoiceResult()
            val r = atPreview(f)

            r.invoiceModel.confirm()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(InvoiceStep.RESULT, flow?.step)
            assertEquals("20260002", flow?.result?.invoiceNo)
            assertNull(flow?.resultError)
            coVerify { f.engine.signInvoiceFiscal(listOf(11L, 12L), 7L, OPERATOR_INSZ) }
        }

    @Test
    fun aRefusedInvoiceKeepsTheEngineErrorInTheResult() =
        runTest {
            val f = fixture()
            coEvery { f.engine.signInvoiceFiscal(any(), any(), any()) } returns
                invoiceResult(success = false, error = "invoice_training_mode")
            val r = atPreview(f)

            r.invoiceModel.confirm()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(InvoiceStep.RESULT, flow?.step)
            assertEquals(r.errorText("invoice_training_mode"), flow?.resultError)
            assertFalse(flow?.result?.success ?: true)
        }

    @Test
    fun aFailedSignatureCallShowsTheFailureInTheResult() =
        runTest {
            val f = fixture()
            coEvery { f.engine.signInvoiceFiscal(any(), any(), any()) } throws PosException.Coded("fdm_unreachable", emptyMap())
            val r = atPreview(f)

            r.invoiceModel.confirm()
            runCurrent()

            val flow = r.invoices.flow
            assertEquals(InvoiceStep.RESULT, flow?.step)
            assertNull(flow?.result)
            assertNotNull(flow?.resultError)
        }

    @Test
    fun confirmOutsideThePreviewDoesNothing() =
        runTest {
            val f = fixture()
            val r = filled(f)

            r.invoiceModel.confirm()
            runCurrent()

            assertEquals(InvoiceStep.CUSTOMER, r.invoices.flow?.step)
            coVerify(exactly = 0) { f.engine.signInvoiceFiscal(any(), any(), any()) }
        }

    @Test
    fun closingASignedInvoiceClearsTheTicksAndReloadsTheLists() =
        runTest {
            val f = fixture()
            coEvery { f.engine.signInvoiceFiscal(any(), any(), any()) } returns invoiceResult()
            val r = atPreview(f)
            r.invoiceModel.confirm()
            runCurrent()
            coEvery { f.engine.listInvoices(any()) } returns listOf(invoiceSummary(2L))

            r.invoiceModel.closeFlow()
            runCurrent()

            assertNull(r.invoices.flow)
            assertTrue(r.invoices.selected.isEmpty())
            assertEquals(listOf(2L), r.invoices.invoices.map { it.id })
        }

    @Test
    fun closingARefusedInvoiceKeepsTheTicks() =
        runTest {
            val f = fixture()
            coEvery { f.engine.signInvoiceFiscal(any(), any(), any()) } returns
                invoiceResult(success = false, error = "invoice_training_mode")
            val r = atPreview(f)
            r.invoiceModel.confirm()
            runCurrent()

            r.invoiceModel.closeFlow()
            runCurrent()

            assertNull(r.invoices.flow)
            assertEquals(listOf(11L, 12L), r.invoices.selected)
        }

    @Test
    fun cancellingTheCustomerStepKeepsTheTicks() =
        runTest {
            val f = fixture()
            val r = filled(f)

            r.invoiceModel.closeFlow()
            runCurrent()

            assertNull(r.invoices.flow)
            assertEquals(listOf(11L, 12L), r.invoices.selected)
        }

    // ── Copy of an invoice ────────────────────────────────────────

    @Test
    fun copyingAnInvoiceSignsACopyOfTheInvoiceEventAndToasts() =
        runTest {
            val f = fixture()
            coEvery { f.engine.copyTicket(any(), any(), any()) } returns copyResult()
            val r = start(f)

            r.invoiceModel.copyInvoice(43L)
            runCurrent()

            coVerify { f.engine.copyTicket(43L, OPERATOR_INSZ, null) }
            assertEquals(listOf(r.text("report_reprint_ok")), r.toastTexts())
            assertNull(r.invoices.copying)
        }

    @Test
    fun aRefusedCopyShowsTheEngineError() =
        runTest {
            val f = fixture()
            coEvery { f.engine.copyTicket(any(), any(), any()) } returns copyResult(success = false, error = "invoice_training_mode")
            val r = start(f)

            r.invoiceModel.copyInvoice(43L)
            runCurrent()

            assertEquals(listOf(r.errorText("invoice_training_mode")), r.toastTexts())
            assertNull(r.invoices.copying)
        }

    @Test
    fun aFailedCopyCallShowsAnErrorToast() =
        runTest {
            val f = fixture()
            coEvery { f.engine.copyTicket(any(), any(), any()) } throws PosException.Coded("fdm_unreachable", emptyMap())
            val r = start(f)

            r.invoiceModel.copyInvoice(43L)
            runCurrent()

            assertEquals(1, r.toastTexts().size)
            assertNull(r.invoices.copying)
        }

    @Test
    fun onlyAManagerCopies() =
        runTest {
            val f = fixture()
            val r = start(f)
            assertTrue(r.invoices.canCopy)

            val cashier = start(fixtureWithCashier())
            assertFalse(cashier.invoices.canCopy)
        }

    private fun TestScope.fixtureWithCashier(): EngineFixture {
        val f = reportsFixture(tmp.newFolder())
        coEvery { f.engine.login(any(), any()) } returns operatorUser.copy(role = uniffi.pos_ffi.UserRole.CASHIER)
        return f
    }

    // ── Pure helpers ──────────────────────────────────────────────

    @Test
    fun salesNotInTheTicketsListGetTheirOwnRowsNewestFirst() {
        val rows = mergeRows(listOf(copyable(5L)), emptyList())
        val merged = ticketRowsWith(rows, listOf(invoiceableSale(5L), invoiceableSale(9L), invoiceableSale(3L, isRefund = true)))

        assertEquals(listOf(9L, 5L, 3L), merged.map { it.fiscalEventId })
        assertEquals(TicketKind.SALE, merged[0].kind)
        assertEquals(TicketKind.REFUND, merged[2].kind)
        assertNull(merged[0].copyable)
        assertNotNull(merged[1].copyable)
    }

    @Test
    fun theStatusKeyFollowsTheDeliveryStatus() {
        assertEquals("invoice_status_queued", invoiceStatusKey("queued"))
        assertEquals("invoice_status_accepted", invoiceStatusKey("accepted"))
        assertEquals("invoice_status_failed", invoiceStatusKey("failed"))
        assertEquals("invoice_status_none", invoiceStatusKey(null))
        assertEquals("invoice_status_none", invoiceStatusKey("whatever"))
    }

    @Test
    fun theIssueDayIsWrittenDayMonthYear() {
        assertEquals("17/10/2026", issueDay("2026-10-17"))
        assertEquals("17/10/2026", issueDay("2026-10-17T10:00:00"))
        assertEquals("garbage", issueDay("garbage"))
    }

    @Test
    fun quantitiesAndRatesAreWrittenLikeTheVueDialog() {
        assertEquals("2", quantityText(2_000L))
        assertEquals("0,35", quantityText(350L))
        assertEquals("1,5", quantityText(1_500L))
        assertEquals("21 %", rateText(2_100L))
        assertEquals("5,5 %", rateText(550L))
        assertEquals("0 %", rateText(0L))
    }
}
