package dev.nuagemagique.pilipos.feature.reports

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.test.core.app.ApplicationProvider
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.feature.tables.RecordingNavigator
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
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/**
 * The route over a signed-in manager and a mocked engine: the invoice lists are read whenever the tickets list
 * loads or changes, the sales tab opens a report on every visit, and the callbacks reach the real view models.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class, qualifiers = "w1920dp-h1080dp-mdpi")
class ReportsRouteWiringTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()
    private val tmp = TemporaryFolder()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(tmp).around(compose)

    private val context = ApplicationProvider.getApplicationContext<android.content.Context>()
    private val navigator = RecordingNavigator()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    /** The two schedulers move together: the view models' (main) and the composition's. */
    private fun TestScope.settle() {
        repeat(SETTLE_ROUNDS) {
            runCurrent()
            compose.waitForIdle()
        }
    }

    private suspend fun TestScope.routeRig(): ReportsRig {
        val f = reportsFixture(tmp.root)
        coEvery { f.engine.listCopyableTickets(any()) } returns listOf(copyable(11L), copyable(12L))
        coEvery { f.engine.listInvoiceableSales(any(), any()) } returns listOf(invoiceableSale(11L), invoiceableSale(12L))
        coEvery { f.engine.listInvoices(any()) } returns listOf(invoiceSummary(3L))
        coEvery { f.engine.salesReport(any(), any()) } answers {
            // The engine agrees with the device on which day it is: one read per visit.
            salesReport(from = firstArg(), to = secondArg(), businessDate = firstArg())
        }
        val r = startReports(f, context)
        compose.setContent { ReportsRoute(navigator, r.entry) }
        settle()
        return r
    }

    @Test
    fun theInvoiceListsAreReadWhenTheTicketsListLoadsAndAgainWhenItsRowsChange() =
        runTest {
            val r = routeRig()
            // The day tab is shown: nothing of the invoices is read yet.
            coVerify(exactly = 0) { r.engine.listInvoiceableSales(any(), any()) }
            coVerify(exactly = 0) { r.engine.listInvoices(any()) }

            compose.onNodeWithTag("reports_tab_tickets").performClick()
            settle()
            coVerify(exactly = 1) { r.engine.listInvoiceableSales(any(), any()) }
            coVerify(exactly = 1) { r.engine.listInvoices(any()) }
            compose.onNodeWithTag("ticket_check_11").assertIsDisplayed()
            compose.onNodeWithTag("invoice_row_3").assertIsDisplayed()

            // A copy or a refund was signed: the tickets list is read again with another row.
            coEvery { r.engine.listCopyableTickets(any()) } returns listOf(copyable(11L), copyable(12L), copyable(13L))
            ViewModelProvider(compose.activity)[ReportsViewModel::class.java].loadTickets()
            settle()
            coVerify(exactly = 2) { r.engine.listInvoiceableSales(any(), any()) }
            coVerify(exactly = 2) { r.engine.listInvoices(any()) }
        }

    @Test
    fun theSalesTabReadsAReportOnEveryVisit() =
        runTest {
            val r = routeRig()
            coVerify(exactly = 0) { r.engine.salesReport(any(), any()) }

            compose.onNodeWithTag("reports_tab_sales").performClick()
            settle()
            coVerify(exactly = 1) { r.engine.salesReport(any(), any()) }

            compose.onNodeWithTag("reports_tab_day").performClick()
            settle()
            coVerify(exactly = 1) { r.engine.salesReport(any(), any()) }
            compose.onNodeWithTag("reports_tab_sales").performClick()
            settle()
            coVerify(exactly = 2) { r.engine.salesReport(any(), any()) }
        }

    @Test
    fun theFactureButtonOfTheRoutedScreenOpensTheDialogOnTheRealInvoiceModel() =
        runTest {
            routeRig()
            compose.onNodeWithTag("reports_tab_tickets").performClick()
            settle()

            compose.onNodeWithTag("ticket_check_11").performClick()
            settle()
            compose.onNodeWithTag("invoice_action").performClick()
            settle()

            compose.onNodeWithTag("invoice_dialog").assertIsDisplayed()
            val model = ViewModelProvider(compose.activity)[InvoiceViewModel::class.java]
            assertEquals(listOf(11L), model.state.value.selected)
            assertEquals(listOf(11L), model.state.value.flow?.saleEventIds)
        }

    private companion object {
        const val SETTLE_ROUNDS = 4
    }
}
