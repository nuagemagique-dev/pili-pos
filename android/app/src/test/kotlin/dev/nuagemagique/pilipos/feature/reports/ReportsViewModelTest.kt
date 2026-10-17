package dev.nuagemagique.pilipos.feature.reports

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
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
import uniffi.pos_ffi.CorrectionDone
import uniffi.pos_ffi.MoneyDirection
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RefundItem
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.SalePayments

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class ReportsViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    private val refused = PosException.Coded("order_no_lines", emptyMap())

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(): EngineFixture = reportsFixture(tmp.root)

    private suspend fun TestScope.start(
        f: EngineFixture,
        open: Boolean = true,
        loggedIn: Boolean = true,
    ): ReportsRig = startReports(f, context, open, loggedIn)

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun loadReadsTheZHistoryAndTheZReminder() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listZReports() } returns listOf(zSummary(5L), zSummary(4L))
            coEvery { f.engine.needsZReport() } returns true
            val r = start(f)

            r.model.load()
            runCurrent()

            assertEquals(listOf(5L, 4L), r.state.day.zHistory.map { it.id })
            assertEquals(5L, r.state.day.lastZ?.id)
            assertTrue(r.state.day.zPending)
            assertTrue(r.state.signedIn)
            assertEquals(ReportsTab.DAY, r.state.tab)
        }

    @Test
    fun loadReadsTheCardPaymentsNoSaleRecordsYet() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.load()
            runCurrent()
            assertTrue(r.state.day.unsettled.isEmpty())

            coEvery { f.engine.listUnsettledCardPayments() } returns
                listOf(unsettled(1L, table = 4), unsettled(2L, amountCents = 950L, brand = null))
            r.model.load()
            runCurrent()

            assertEquals(listOf(1L, 2L), r.state.day.unsettled.map { it.paymentId })
        }

    @Test
    fun theZIsNeverBlockedByUnsettledCardsAndTheListIsReadAgainWhenArmedAndAfterwards() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listUnsettledCardPayments() } returnsMany
                listOf(emptyList(), listOf(unsettled(1L, table = 4)), emptyList())
            coEvery { f.engine.generateZReport(OPERATOR_INSZ) } returns reportResult(reportNo = 3L, type = "Z")
            val r = start(f)
            r.model.load()
            runCurrent()
            assertTrue(r.state.day.unsettled.isEmpty())

            // The first tap arms the button and reads the list again: the warning is fresh.
            r.model.generateZ()
            runCurrent()
            assertTrue(r.state.day.zArmed)
            assertEquals(listOf(1L), r.state.day.unsettled.map { it.paymentId })

            // The second tap closes the day all the same.
            r.model.generateZ()
            runCurrent()
            coVerify(exactly = 1) { f.engine.generateZReport(OPERATOR_INSZ) }
            assertEquals(ReportKind.Z, r.state.day.lastReport?.kind)
            assertTrue(r.state.day.unsettled.isEmpty())
        }

    @Test
    fun anUnreadableListMeansNoWarning() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listUnsettledCardPayments() } throws refused
            val r = start(f)

            r.model.load()
            runCurrent()

            assertTrue(r.state.day.unsettled.isEmpty())
            assertTrue(r.toastTexts().isEmpty())
        }

    @Test
    fun failingReadsDegradeSilently() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listZReports() } throws refused
            coEvery { f.engine.needsZReport() } throws refused
            val r = start(f)

            r.model.load()
            runCurrent()

            assertTrue(r.state.day.zHistory.isEmpty())
            assertFalse(r.state.day.zPending)
            assertTrue(r.toastTexts().isEmpty())
        }

    @Test
    fun aResyncLoadsTheDayTabAgain() =
        runTest {
            val f = fixture()
            coEvery { f.engine.needsZReport() } returnsMany listOf(false, true)
            val r = start(f)
            r.model.load()
            runCurrent()
            assertFalse(r.state.day.zPending)

            r.feed.send(PosEvent.Resync(missed = 2L))
            runCurrent()

            assertTrue(r.state.day.zPending)
            coVerify(exactly = 2) { f.engine.listZReports() }
        }

    // ── X, user X ─────────────────────────────────────────────────

    @Test
    fun theXReportIsSignedByTheOperatorAndKeptForThePanel() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateXReport(OPERATOR_INSZ) } returns reportResult(type = "X")
            val r = start(f)

            r.model.generateX()
            runCurrent()

            coVerify(exactly = 1) { f.engine.generateXReport(OPERATOR_INSZ) }
            val outcome = r.state.day.lastReport!!
            assertEquals(ReportKind.X, outcome.kind)
            assertTrue(outcome.generated)
            assertEquals("X", outcome.result?.reportType)
            assertNull(r.state.day.loadingKind)
            assertEquals(listOf("${r.text("report_generated")} (X)"), r.toastTexts())
        }

    @Test
    fun theUserXReportIsSignedByTheOperator() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateUserXReport(OPERATOR_INSZ) } returns reportResult(type = "USER_X")
            val r = start(f)

            r.model.generateUserX()
            runCurrent()

            coVerify(exactly = 1) { f.engine.generateUserXReport(OPERATOR_INSZ) }
            assertEquals(ReportKind.USER_X, r.state.day.lastReport?.kind)
            assertEquals(listOf("${r.text("report_generated")} (X util.)"), r.toastTexts())
        }

    @Test
    fun aRefusedReportShowsTheErrorAndKeepsItInThePanel() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateXReport(any()) } returns reportResult(success = false, error = "order_no_lines")
            val r = start(f)

            r.model.generateX()
            runCurrent()

            val outcome = r.state.day.lastReport!!
            assertFalse(outcome.generated)
            assertEquals(r.errorText("order_no_lines"), outcome.failure)
            assertEquals(listOf(r.errorText("order_no_lines")), r.toastTexts())
            assertNull(r.state.day.loadingKind)
        }

    @Test
    fun aFailingReportCallShowsTheErrorAndFreesTheButtons() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateUserXReport(any()) } throws refused
            val r = start(f)

            r.model.generateUserX()
            runCurrent()

            val outcome = r.state.day.lastReport!!
            assertNull(outcome.result)
            assertEquals(r.model.describeError(refused), outcome.failure)
            assertEquals(1, r.toastTexts().size)
            assertFalse(r.state.day.busy)
        }

    @Test
    fun aReportInProgressBlocksTheOtherButtons() =
        runTest {
            val f = fixture()
            val gate = CompletableDeferred<Unit>()
            coEvery { f.engine.generateXReport(any()) } coAnswers
                {
                    gate.await()
                    reportResult()
                }
            val r = start(f)

            r.model.generateX()
            runCurrent()
            assertEquals(ReportKind.X, r.state.day.loadingKind)
            r.model.generateX()
            r.model.generateUserX()
            r.model.generateZ()
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { f.engine.generateXReport(any()) }
            coVerify(exactly = 0) { f.engine.generateUserXReport(any()) }
            assertFalse(r.state.day.zArmed)
            assertNull(r.state.day.loadingKind)
        }

    // ── Z ─────────────────────────────────────────────────────────

    @Test
    fun theFirstTapOnZOnlyArmsTheButton() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.generateZ()
            runCurrent()

            assertTrue(r.state.day.zArmed)
            coVerify(exactly = 0) { f.engine.generateZReport(any()) }
            assertTrue(r.toastTexts().isEmpty())
        }

    @Test
    fun theZButtonDisarmsAfterFiveSeconds() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.generateZ()
            runCurrent()

            advanceTimeBy(DayState.Z_ARM_WINDOW_MS - 1L)
            assertTrue(r.state.day.zArmed)
            advanceTimeBy(2L)
            assertFalse(r.state.day.zArmed)

            r.model.generateZ()
            runCurrent()
            coVerify(exactly = 0) { f.engine.generateZReport(any()) }
            assertTrue(r.state.day.zArmed)
        }

    @Test
    fun theSecondTapWithinTheWindowGeneratesTheZAndReloadsTheDay() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateZReport(OPERATOR_INSZ) } returns reportResult(reportNo = 12L, type = "Z")
            coEvery { f.engine.listZReports() } returnsMany listOf(emptyList(), listOf(zSummary(12L)))
            coEvery { f.engine.needsZReport() } returnsMany listOf(true, false)
            val r = start(f)
            r.model.load()
            runCurrent()
            assertTrue(r.state.day.zPending)

            r.model.generateZ()
            advanceTimeBy(4000L)
            r.model.generateZ()
            runCurrent()

            coVerify(exactly = 1) { f.engine.generateZReport(OPERATOR_INSZ) }
            assertFalse(r.state.day.zArmed)
            assertEquals(ReportKind.Z, r.state.day.lastReport?.kind)
            assertEquals(listOf(12L), r.state.day.zHistory.map { it.id })
            assertFalse(r.state.day.zPending)
            assertEquals(listOf("${r.text("report_generated")} (Z #12)"), r.toastTexts())
            // The disarm timer of the first tap does not fire later.
            advanceTimeBy(DayState.Z_ARM_WINDOW_MS)
            assertFalse(r.state.day.zArmed)
        }

    @Test
    fun aZWithoutReportNumberIsNamedZ() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateZReport(any()) } returns reportResult(reportNo = null, type = "Z")
            val r = start(f)

            r.model.generateZ()
            r.model.generateZ()
            runCurrent()

            assertEquals(listOf("${r.text("report_generated")} (Z)"), r.toastTexts())
        }

    @Test
    fun aRefusedZKeepsTheHistoryAndShowsTheError() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateZReport(any()) } returns reportResult(success = false, error = "order_no_lines", type = "Z")
            val r = start(f)
            r.model.load()
            runCurrent()

            r.model.generateZ()
            r.model.generateZ()
            runCurrent()

            assertEquals(r.errorText("order_no_lines"), r.state.day.lastReport?.failure)
            assertFalse(r.state.day.zArmed)
            coVerify(exactly = 1) { f.engine.listZReports() }
        }

    @Test
    fun leavingTheDayTabDisarmsTheZ() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.generateZ()
            runCurrent()
            assertTrue(r.state.day.zArmed)

            r.model.selectTab(ReportsTab.CASH)
            runCurrent()
            r.model.selectTab(ReportsTab.DAY)
            runCurrent()
            r.model.generateZ()
            runCurrent()

            coVerify(exactly = 0) { f.engine.generateZReport(any()) }
            assertTrue(r.state.day.zArmed)
        }

    @Test
    fun theZHistoryDialogOpensWithAFreshListAndCloses() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listZReports() } returns listOf(zSummary(3L))
            val r = start(f)

            r.model.openZList()
            runCurrent()
            assertTrue(r.state.day.zListOpen)
            assertEquals(listOf(3L), r.state.day.zHistory.map { it.id })

            r.model.closeZList()
            assertFalse(r.state.day.zListOpen)
        }

    @Test
    fun withoutAnOperatorNoReportIsCalled() =
        runTest {
            val f = fixture()
            val r = start(f, loggedIn = false)

            r.model.generateX()
            r.model.generateUserX()
            r.model.generateZ()
            r.model.generateZ()
            r.model.setCashAmount(1000L)
            r.model.submitCash()
            r.model.declareDrawer(0L)
            r.model.openDrawer()
            runCurrent()

            assertFalse(r.state.signedIn)
            assertFalse(r.state.day.zArmed)
            coVerify(exactly = 0) { f.engine.generateXReport(any()) }
            coVerify(exactly = 0) { f.engine.generateUserXReport(any()) }
            coVerify(exactly = 0) { f.engine.generateZReport(any()) }
            coVerify(exactly = 0) { f.engine.moneyInOut(any(), any(), any(), any()) }
            coVerify(exactly = 0) { f.engine.openDrawerFiscal(any()) }
        }

    @Test
    fun aClosedEngineNeverCrashesAnAction() =
        runTest {
            val f = fixture()
            val r = start(f)
            f.holder.close()
            runCurrent()

            r.model.load()
            r.model.generateX()
            r.model.setCashAmount(500L)
            r.model.submitCash()
            r.model.openDrawer()
            runCurrent()

            // The session forgets the operator when the engine goes away (UI-PD311): the actions are no-ops, the
            // reads fail with the starting message, and nothing crashes.
            val starting = r.model.describeError(EngineUnavailableException(EngineState.Opening))
            assertFalse(r.state.signedIn)
            assertNull(r.state.day.lastReport)
            assertNull(r.state.cash.lastOutcome)
            assertTrue(r.toastTexts().all { it == starting })
            assertTrue(r.state.day.zHistory.isEmpty())
        }

    // ── Cash ──────────────────────────────────────────────────────

    @Test
    fun moneyInIsSentWithTheAmountAsEntered() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial()
            val r = start(f)

            r.model.setCashAmount(2000L)
            assertFalse(r.state.cash.custom)
            r.model.submitCash()
            runCurrent()

            coVerify(exactly = 1) { f.engine.moneyInOut(OPERATOR_INSZ, 2000L, MoneyDirection.IN, "fond_de_caisse") }
            assertEquals(0L, r.state.cash.amountCents)
            assertFalse(r.state.cash.loading)
            assertEquals(true, r.state.cash.lastOutcome?.success)
            assertEquals(listOf(r.text("cash_success")), r.toastTexts())
        }

    @Test
    fun moneyOutIsSentWithTheDirectionAndTheReasonOfTheForm() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial()
            val r = start(f)

            r.model.setCashDirection(CashDirection.OUT)
            assertEquals("retrait", r.state.cash.reason)
            assertEquals(listOf("retrait", "pourboire"), r.state.cash.reasons)
            r.model.setCashReason("pourboire")
            r.model.setCashAmount(1234L)
            assertTrue(r.state.cash.custom)
            r.model.submitCash()
            runCurrent()

            coVerify(exactly = 1) { f.engine.moneyInOut(OPERATOR_INSZ, 1234L, MoneyDirection.OUT, "pourboire") }
        }

    @Test
    fun aReasonThatFitsBothDirectionsSurvivesTheSwitch() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.setCashReason("pourboire")
            r.model.setCashDirection(CashDirection.OUT)
            assertEquals("pourboire", r.state.cash.reason)
            r.model.setCashDirection(CashDirection.IN)
            assertEquals("pourboire", r.state.cash.reason)
            r.model.setCashReason("fond_de_caisse")
            r.model.setCashDirection(CashDirection.OUT)
            assertEquals("retrait", r.state.cash.reason)
        }

    @Test
    fun aMovementWithoutAmountIsNotSent() =
        runTest {
            val f = fixture()
            val r = start(f)

            assertFalse(r.state.cash.canSubmit)
            r.model.submitCash()
            runCurrent()

            coVerify(exactly = 0) { f.engine.moneyInOut(any(), any(), any(), any()) }
        }

    @Test
    fun aRefusedMovementKeepsTheAmountAndShowsTheError() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial(success = false, error = "order_no_lines")
            val r = start(f)
            r.model.setCashAmount(5000L)

            r.model.submitCash()
            runCurrent()

            assertEquals(5000L, r.state.cash.amountCents)
            assertEquals(false, r.state.cash.lastOutcome?.success)
            assertEquals(r.errorText("order_no_lines"), r.state.cash.lastOutcome?.failure)
            assertEquals(listOf(r.errorText("order_no_lines")), r.toastTexts())
        }

    @Test
    fun aRefusalWithoutTextGivesTheCashFailedText() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial(success = false, error = null)
            val r = start(f)
            r.model.setCashAmount(5000L)

            r.model.submitCash()
            runCurrent()

            assertEquals(r.text("cash_failed"), r.state.cash.lastOutcome?.failure)
            assertEquals(listOf(r.text("cash_failed")), r.toastTexts())
        }

    @Test
    fun aFailingMovementCallKeepsTheAmountAndFreesTheForm() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } throws refused
            val r = start(f)
            r.model.setCashAmount(5000L)

            r.model.submitCash()
            runCurrent()

            assertEquals(5000L, r.state.cash.amountCents)
            assertFalse(r.state.cash.loading)
            assertEquals(r.model.describeError(refused), r.state.cash.lastOutcome?.failure)
        }

    @Test
    fun theDrawerCountMayBeZero() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial()
            val r = start(f)

            r.model.declareDrawer(0L)
            runCurrent()

            coVerify(exactly = 1) { f.engine.moneyInOut(OPERATOR_INSZ, 0L, MoneyDirection.DECLARATION, "declaration_tiroir") }
            assertEquals(listOf(r.text("cash_declared")), r.toastTexts())
        }

    @Test
    fun theDrawerCountIsSentAsEnteredAndKeepsTheForm() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial()
            val r = start(f)
            r.model.setCashAmount(700L)

            r.model.declareDrawer(31250L)
            runCurrent()

            coVerify(exactly = 1) { f.engine.moneyInOut(OPERATOR_INSZ, 31250L, MoneyDirection.DECLARATION, "declaration_tiroir") }
            assertEquals(700L, r.state.cash.amountCents)
        }

    @Test
    fun aNegativeDrawerCountIsNotSent() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.declareDrawer(-1L)
            runCurrent()

            coVerify(exactly = 0) { f.engine.moneyInOut(any(), any(), any(), any()) }
        }

    @Test
    fun aRefusedDrawerCountShowsTheError() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial(success = false, error = "order_no_lines")
            val r = start(f)

            r.model.declareDrawer(0L)
            runCurrent()

            assertEquals(r.errorText("order_no_lines"), r.state.cash.lastOutcome?.failure)
        }

    @Test
    fun theDrawerOpensOutsideOfASale() =
        runTest {
            val f = fixture()
            coEvery { f.engine.openDrawerFiscal(OPERATOR_INSZ) } returns financial()
            val r = start(f)

            r.model.openDrawer()
            runCurrent()

            coVerify(exactly = 1) { f.engine.openDrawerFiscal(OPERATOR_INSZ) }
            assertEquals(listOf(r.text("cash_drawer_opened")), r.toastTexts())
        }

    @Test
    fun aRefusedDrawerOpeningShowsTheError() =
        runTest {
            val f = fixture()
            coEvery { f.engine.openDrawerFiscal(any()) } returns financial(success = false, error = null)
            val r = start(f)

            r.model.openDrawer()
            runCurrent()

            assertEquals(listOf(r.text("cash_failed")), r.toastTexts())
            assertEquals(false, r.state.cash.lastOutcome?.success)
        }

    @Test
    fun aFailingDrawerOpeningShowsTheError() =
        runTest {
            val f = fixture()
            coEvery { f.engine.openDrawerFiscal(any()) } throws refused
            val r = start(f)

            r.model.openDrawer()
            runCurrent()

            assertEquals(listOf(r.model.describeError(refused)), r.toastTexts())
        }

    // ── Tickets tab ───────────────────────────────────────────────

    @Test
    fun openingTheTicketsTabReadsBothListsAndMergesThemNewestFirst() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listCopyableTickets(null) } returns
                listOf(
                    copyable(10L),
                    copyable(11L, label = "T"),
                    copyable(12L, label = "P"),
                    copyable(13L, totalCents = -500L),
                )
            coEvery { f.engine.listRefundableEvents(20L) } returns listOf(refundable(10L), refundable(9L))
            val r = start(f)

            r.model.selectTab(ReportsTab.TICKETS)
            runCurrent()

            coVerify(exactly = 1) { f.engine.listCopyableTickets(null) }
            coVerify(exactly = 1) { f.engine.listRefundableEvents(20L) }
            val rows = r.state.tickets.rows
            assertEquals(listOf(13L, 12L, 11L, 10L, 9L), rows.map { it.fiscalEventId })
            assertEquals(
                listOf(TicketKind.REFUND, TicketKind.PRE_BILL, TicketKind.TRAINING, TicketKind.SALE, TicketKind.SALE),
                rows.map { it.kind },
            )
            val both = rows.single { it.fiscalEventId == 10L }
            assertNotNull(both.copyable)
            assertNotNull(both.refundable)
            val refundOnly = rows.single { it.fiscalEventId == 9L }
            assertNull(refundOnly.copyable)
            assertNotNull(refundOnly.refundable)
            assertTrue(r.state.tickets.loaded)
            assertFalse(r.state.tickets.loading)
        }

    @Test
    fun theTicketListsAreNotReadBeforeTheTab() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.load()
            runCurrent()

            coVerify(exactly = 0) { f.engine.listCopyableTickets(any()) }
            assertFalse(r.state.tickets.loaded)
        }

    @Test
    fun aFailingCopyableListShowsTheErrorAndKeepsTheRefundableOnes() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listCopyableTickets(any()) } throws refused
            coEvery { f.engine.listRefundableEvents(any()) } returns listOf(refundable(4L))
            val r = start(f)

            r.model.selectTab(ReportsTab.TICKETS)
            runCurrent()

            assertTrue(r.state.tickets.copyable.isEmpty())
            assertEquals(listOf(4L), r.state.tickets.rows.map { it.fiscalEventId })
            assertEquals(listOf(r.model.describeError(refused)), r.toastTexts())
        }

    @Test
    fun aFailingRefundableListIsSilent() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listCopyableTickets(any()) } returns listOf(copyable(4L))
            coEvery { f.engine.listRefundableEvents(any()) } throws refused
            val r = start(f)

            r.model.selectTab(ReportsTab.TICKETS)
            runCurrent()

            assertEquals(listOf(4L), r.state.tickets.rows.map { it.fiscalEventId })
            assertTrue(r.toastTexts().isEmpty())
        }

    // ── Copy flow ─────────────────────────────────────────────────

    @Test
    fun theCopyFlowOpensOnThePickerWithoutSigning() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.openCopy(7L, CopyTargetKind.TICKET)

            val flow = r.state.copyFlow!!
            assertEquals(7L, flow.fiscalEventId)
            assertEquals(CopyStep.PICKER, flow.step)
            assertEquals(CopyTargetKind.TICKET, flow.kind)
            coVerify(exactly = 0) { f.engine.copyTicket(any(), any(), any()) }
        }

    @Test
    fun anEventWithoutFiscalIdOpensNoCopyFlow() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.openCopy(null, CopyTargetKind.Z)

            assertNull(r.state.copyFlow)
        }

    @Test
    fun confirmingTheCopySignsItWithTheDeliveryRequest() =
        runTest {
            val f = fixture()
            val request = delivery(RequestedMedium.DIGITAL, "a@b.be")
            coEvery { f.engine.copyTicket(7L, OPERATOR_INSZ, request) } returns copyResult()
            val r = start(f)
            r.model.openCopy(7L, CopyTargetKind.TICKET)

            r.model.confirmCopy(request)
            runCurrent()

            coVerify(exactly = 1) { f.engine.copyTicket(7L, OPERATOR_INSZ, request) }
            val flow = r.state.copyFlow!!
            assertEquals(CopyStep.RESULT, flow.step)
            assertEquals(true, flow.result?.success)
            assertEquals(listOf(r.text("report_reprint_ok")), r.toastTexts())
        }

    @Test
    fun aPaperCopyIsSentWithoutRequest() =
        runTest {
            val f = fixture()
            coEvery { f.engine.copyTicket(7L, OPERATOR_INSZ, null) } returns copyResult()
            val r = start(f)
            r.model.openCopy(7L, CopyTargetKind.TICKET)

            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            runCurrent()

            coVerify(exactly = 1) { f.engine.copyTicket(7L, OPERATOR_INSZ, null) }
        }

    @Test
    fun theCopyCanBeStartedDirectlyFromAnEvent() =
        runTest {
            val f = fixture()
            val request = delivery(RequestedMedium.PAPER_DIGITAL, "c@d.be")
            coEvery { f.engine.copyTicket(any(), any(), any()) } returns copyResult()
            val r = start(f)

            r.model.copy(9L, request)
            runCurrent()

            coVerify(exactly = 1) { f.engine.copyTicket(9L, OPERATOR_INSZ, request) }
            assertEquals(CopyStep.RESULT, r.state.copyFlow?.step)
        }

    @Test
    fun aCopyIsNotSignedTwice() =
        runTest {
            val f = fixture()
            val gate = CompletableDeferred<Unit>()
            coEvery { f.engine.copyTicket(any(), any(), any()) } coAnswers
                {
                    gate.await()
                    copyResult()
                }
            val r = start(f)
            r.model.openCopy(7L, CopyTargetKind.TICKET)

            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            runCurrent()
            assertEquals(CopyStep.PROCESSING, r.state.copyFlow?.step)
            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { f.engine.copyTicket(any(), any(), any()) }
        }

    @Test
    fun aRefusedCopyShowsTheErrorOnTheResultStep() =
        runTest {
            val f = fixture()
            coEvery { f.engine.copyTicket(any(), any(), any()) } returns copyResult(success = false, error = "order_no_lines")
            val r = start(f)
            r.model.openCopy(7L, CopyTargetKind.TICKET)

            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            runCurrent()

            val flow = r.state.copyFlow!!
            assertEquals(CopyStep.RESULT, flow.step)
            assertEquals(false, flow.result?.success)
            assertEquals(listOf(r.errorText("order_no_lines")), r.toastTexts())
        }

    @Test
    fun aFailingCopyCallClosesTheFlow() =
        runTest {
            val f = fixture()
            coEvery { f.engine.copyTicket(any(), any(), any()) } throws refused
            val r = start(f)
            r.model.openCopy(7L, CopyTargetKind.TICKET)

            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            runCurrent()

            assertNull(r.state.copyFlow)
            assertEquals(listOf(r.model.describeError(refused)), r.toastTexts())
        }

    @Test
    fun closingASignedTicketCopyReloadsTheTickets() =
        runTest {
            val f = fixture()
            coEvery { f.engine.copyTicket(any(), any(), any()) } returns copyResult()
            val r = start(f)
            r.model.openCopy(7L, CopyTargetKind.TICKET)
            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            runCurrent()
            coVerify(exactly = 0) { f.engine.listCopyableTickets(any()) }

            r.model.closeCopy()
            runCurrent()

            assertNull(r.state.copyFlow)
            coVerify(exactly = 1) { f.engine.listCopyableTickets(null) }
        }

    @Test
    fun closingACopyThatWasNotSignedReloadsNothing() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.openCopy(7L, CopyTargetKind.TICKET)

            r.model.closeCopy()
            runCurrent()

            assertNull(r.state.copyFlow)
            coVerify(exactly = 0) { f.engine.listCopyableTickets(any()) }
        }

    @Test
    fun reprintingAZFromTheHistoryCopiesItAndLeavesTheTicketsAlone() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listZReports() } returns listOf(zSummary(6L, fiscalEventId = 33L))
            coEvery { f.engine.copyTicket(33L, OPERATOR_INSZ, null) } returns copyResult()
            val r = start(f)
            r.model.openZList()
            runCurrent()

            r.model.reprintZ(r.state.day.zHistory.first().fiscalEventId)
            assertFalse(r.state.day.zListOpen)
            assertEquals(CopyTargetKind.Z, r.state.copyFlow?.kind)
            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            runCurrent()
            r.model.closeCopy()
            runCurrent()

            coVerify(exactly = 1) { f.engine.copyTicket(33L, OPERATOR_INSZ, null) }
            coVerify(exactly = 0) { f.engine.listCopyableTickets(any()) }
        }

    @Test
    fun aZWithoutFiscalEventCannotBeReprinted() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.reprintZ(null)

            assertNull(r.state.copyFlow)
        }

    // ── Refund flow ───────────────────────────────────────────────

    private suspend fun TestScope.refundRig(
        f: EngineFixture,
        vararg quantities: Int,
    ): ReportsRig {
        val lines = quantities.mapIndexed { index, quantity -> orderLine(id = index + 1L, quantity = quantity) }
        coEvery { f.engine.listOrderLines(50L) } returns lines
        val r = start(f)
        r.model.openRefund(refundable(5L, orderId = 50L))
        runCurrent()
        return r
    }

    @Test
    fun theRefundFlowStartsOnTheListOfRefundableSales() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listRefundableEvents(20L) } returns listOf(refundable(5L))
            val r = start(f)

            r.model.startRefund()
            runCurrent()

            assertEquals(RefundStep.CHOOSE_EVENT, r.state.refundFlow?.step)
            assertEquals(listOf(5L), r.state.tickets.refundable.map { it.fiscalEventId })
        }

    @Test
    fun choosingASaleReadsItsLines() =
        runTest {
            val f = fixture()
            val r = refundRig(f, 2, 1)

            coVerify(exactly = 1) { f.engine.listOrderLines(50L) }
            val flow = r.state.refundFlow!!
            assertEquals(RefundStep.CHOOSE_LINES, flow.step)
            assertEquals(5L, flow.event?.fiscalEventId)
            assertEquals(listOf(1L, 2L), flow.lines.map { it.id })
            assertFalse(flow.hasSelection)
        }

    @Test
    fun aFailingLineReadGivesNoLine() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listOrderLines(any()) } throws refused
            val r = start(f)

            r.model.openRefund(refundable(5L, orderId = 50L))
            runCurrent()

            assertEquals(RefundStep.CHOOSE_LINES, r.state.refundFlow?.step)
            assertTrue(r.state.refundFlow!!.lines.isEmpty())
        }

    @Test
    fun tickingALineTakesItsFullQuantityAndTickingAgainRemovesIt() =
        runTest {
            val f = fixture()
            val r = refundRig(f, 3, 2)

            r.model.toggleRefundLine(1L)
            r.model.toggleRefundLine(2L)
            assertEquals(mapOf(1L to 3, 2L to 2), r.state.refundFlow?.checked)
            r.model.toggleRefundLine(1L)

            assertEquals(mapOf(2L to 2), r.state.refundFlow?.checked)
            assertEquals(listOf(2), r.state.refundFlow!!.selected.map { it.quantity })
            r.model.toggleRefundLine(99L)
            assertEquals(mapOf(2L to 2), r.state.refundFlow?.checked)
        }

    @Test
    fun theRefundQuantityStaysBetweenOneAndTheQuantityOfTheLine() =
        runTest {
            val f = fixture()
            val r = refundRig(f, 3)
            r.model.toggleRefundLine(1L)

            r.model.adjustRefundQuantity(1L, 1)
            assertEquals(3, r.state.refundFlow?.checked?.get(1L))
            r.model.adjustRefundQuantity(1L, -1)
            r.model.adjustRefundQuantity(1L, -1)
            assertEquals(1, r.state.refundFlow?.checked?.get(1L))
            r.model.adjustRefundQuantity(1L, -1)
            assertEquals(1, r.state.refundFlow?.checked?.get(1L))
            r.model.adjustRefundQuantity(1L, 5)
            assertEquals(3, r.state.refundFlow?.checked?.get(1L))
            r.model.adjustRefundQuantity(1L, -9)
            assertEquals(1, r.state.refundFlow?.checked?.get(1L))
        }

    @Test
    fun steppingAnUntickedLineTicksItAtOne() =
        runTest {
            val f = fixture()
            val r = refundRig(f, 4)

            r.model.adjustRefundQuantity(1L, 1)
            assertEquals(1, r.state.refundFlow?.checked?.get(1L))
            r.model.adjustRefundQuantity(77L, 1)
            assertEquals(mapOf(1L to 1), r.state.refundFlow?.checked)
        }

    @Test
    fun theDeliveryStepOpensOnlyWithALineTicked() =
        runTest {
            val f = fixture()
            val r = refundRig(f, 2)

            r.model.goToRefundDelivery()
            assertEquals(RefundStep.CHOOSE_LINES, r.state.refundFlow?.step)
            r.model.toggleRefundLine(1L)
            r.model.goToRefundDelivery()
            assertEquals(RefundStep.DELIVERY, r.state.refundFlow?.step)
            r.model.backToRefundLines()
            assertEquals(RefundStep.CHOOSE_LINES, r.state.refundFlow?.step)
            assertTrue(r.state.refundFlow!!.hasSelection)
        }

    @Test
    fun confirmingTheRefundSignsTheTickedLinesInTheOrderTheyWereTicked() =
        runTest {
            val f = fixture()
            val request = delivery(RequestedMedium.DIGITAL, "a@b.be")
            val items =
                listOf(
                    RefundItem(menuItemId = 200L, quantity = 1, lineId = 2L),
                    RefundItem(menuItemId = 100L, quantity = 3, lineId = 1L),
                )
            coEvery { f.engine.refundCheckout(5L, items, OPERATOR_INSZ, request) } returns checkoutResult()
            val r = refundRig(f, 3, 2)
            r.model.toggleRefundLine(2L)
            r.model.toggleRefundLine(1L)
            r.model.adjustRefundQuantity(2L, -1)
            r.model.goToRefundDelivery()

            r.model.refund(request)
            runCurrent()

            coVerify(exactly = 1) { f.engine.refundCheckout(5L, items, OPERATOR_INSZ, request) }
            val flow = r.state.refundFlow!!
            assertEquals(RefundStep.RESULT, flow.step)
            assertEquals(true, flow.result?.success)
            assertEquals(listOf(r.text("refund_success")), r.toastTexts())
        }

    @Test
    fun aPaperRefundIsSentWithoutRequest() =
        runTest {
            val f = fixture()
            coEvery { f.engine.refundCheckout(any(), any(), any(), null) } returns checkoutResult()
            val r = refundRig(f, 1)
            r.model.toggleRefundLine(1L)
            r.model.goToRefundDelivery()

            r.model.refund(delivery(RequestedMedium.PAPER))
            runCurrent()

            coVerify(exactly = 1) { f.engine.refundCheckout(5L, any(), OPERATOR_INSZ, null) }
        }

    @Test
    fun aRefundOnlyStartsFromTheDeliveryStep() =
        runTest {
            val f = fixture()
            val r = refundRig(f, 1)
            r.model.toggleRefundLine(1L)

            r.model.refund(delivery(RequestedMedium.PAPER))
            runCurrent()

            coVerify(exactly = 0) { f.engine.refundCheckout(any(), any(), any(), any()) }
            assertEquals(RefundStep.CHOOSE_LINES, r.state.refundFlow?.step)
        }

    @Test
    fun aRefundIsNotSignedTwice() =
        runTest {
            val f = fixture()
            val gate = CompletableDeferred<Unit>()
            coEvery { f.engine.refundCheckout(any(), any(), any(), any()) } coAnswers
                {
                    gate.await()
                    checkoutResult()
                }
            val r = refundRig(f, 1)
            r.model.toggleRefundLine(1L)
            r.model.goToRefundDelivery()

            r.model.refund(delivery(RequestedMedium.PAPER))
            runCurrent()
            assertEquals(RefundStep.PROCESSING, r.state.refundFlow?.step)
            r.model.refund(delivery(RequestedMedium.PAPER))
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { f.engine.refundCheckout(any(), any(), any(), any()) }
        }

    @Test
    fun aRefusedRefundShowsTheErrorOnTheResultStep() =
        runTest {
            val f = fixture()
            coEvery { f.engine.refundCheckout(any(), any(), any(), any()) } returns
                checkoutResult(success = false, error = "order_no_lines")
            val r = refundRig(f, 1)
            r.model.toggleRefundLine(1L)
            r.model.goToRefundDelivery()

            r.model.refund(delivery(RequestedMedium.PAPER))
            runCurrent()

            val flow = r.state.refundFlow!!
            assertEquals(RefundStep.RESULT, flow.step)
            assertEquals(false, flow.result?.success)
            assertEquals(listOf(r.errorText("order_no_lines")), r.toastTexts())
        }

    @Test
    fun aFailingRefundCallGoesBackToTheLines() =
        runTest {
            val f = fixture()
            coEvery { f.engine.refundCheckout(any(), any(), any(), any()) } throws refused
            val r = refundRig(f, 1)
            r.model.toggleRefundLine(1L)
            r.model.goToRefundDelivery()

            r.model.refund(delivery(RequestedMedium.PAPER))
            runCurrent()

            val flow = r.state.refundFlow!!
            assertEquals(RefundStep.CHOOSE_LINES, flow.step)
            assertTrue(flow.hasSelection)
            assertEquals(listOf(r.model.describeError(refused)), r.toastTexts())
        }

    @Test
    fun closingASignedRefundReloadsTheTickets() =
        runTest {
            val f = fixture()
            coEvery { f.engine.refundCheckout(any(), any(), any(), any()) } returns checkoutResult()
            val r = refundRig(f, 1)
            r.model.toggleRefundLine(1L)
            r.model.goToRefundDelivery()
            r.model.refund(delivery(RequestedMedium.PAPER))
            runCurrent()

            r.model.closeRefund()
            runCurrent()

            assertNull(r.state.refundFlow)
            coVerify(exactly = 1) { f.engine.listRefundableEvents(20L) }
        }

    @Test
    fun closingARefundThatWasNotSignedReloadsNothing() =
        runTest {
            val f = fixture()
            val r = refundRig(f, 1)

            r.model.closeRefund()
            runCurrent()

            assertNull(r.state.refundFlow)
            coVerify(exactly = 0) { f.engine.listRefundableEvents(any()) }
        }

    @Test
    fun aClosedEngineNeverCrashesTheTicketFlows() =
        runTest {
            val f = fixture()
            val r = start(f)
            f.holder.close()
            runCurrent()

            r.model.selectTab(ReportsTab.TICKETS)
            r.model.openRefund(refundable(1L))
            r.model.openCopy(7L, CopyTargetKind.TICKET)
            r.model.confirmCopy(delivery(RequestedMedium.PAPER))
            runCurrent()

            assertTrue(r.state.tickets.loaded)
            assertTrue(r.state.tickets.rows.isEmpty())
            assertTrue(r.state.refundFlow!!.lines.isEmpty())
            // No operator after the engine went away (UI-PD311): the copy stays at the picker, nothing is sent.
            assertFalse(r.state.signedIn)
            assertEquals(CopyStep.PICKER, r.state.copyFlow?.step)
            coVerify(exactly = 0) { f.engine.copyTicket(any(), any(), any()) }
            val starting = r.model.describeError(EngineUnavailableException(EngineState.Opening))
            assertTrue(r.toastTexts().all { it == starting })
        }

    // ── Keypads of the cash tab (T9) ──────────────────────────────

    @Test
    fun theOtherAmountKeypadOpensAndTakesTheAmountAsTyped() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.setCashAmount(1000L)

            r.model.openOtherAmount()
            runCurrent()
            assertTrue(r.state.cash.otherAmountOpen)

            r.model.pickOtherAmount(1250L)
            runCurrent()

            assertFalse(r.state.cash.otherAmountOpen)
            assertEquals(1250L, r.state.cash.amountCents)
            assertTrue(r.state.cash.custom)
        }

    @Test
    fun theOtherAmountKeypadClosesWithoutChangingTheAmount() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.setCashAmount(2000L)
            r.model.openOtherAmount()

            r.model.closeOtherAmount()
            runCurrent()

            assertFalse(r.state.cash.otherAmountOpen)
            assertEquals(2000L, r.state.cash.amountCents)
        }

    @Test
    fun theDrawerCountKeypadOpensAndClosesWithoutRecording() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.openDeclaration()
            runCurrent()
            assertTrue(r.state.cash.declarationOpen)

            r.model.closeDeclaration()
            runCurrent()

            assertFalse(r.state.cash.declarationOpen)
            coVerify(exactly = 0) { f.engine.moneyInOut(any(), any(), any(), any()) }
        }

    @Test
    fun declaringTheDrawerClosesItsKeypadEvenWhenTheCountIsRefused() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial(success = false, error = "no")
            val r = start(f)
            r.model.openDeclaration()

            r.model.declareDrawer(500L)
            runCurrent()

            assertFalse(r.state.cash.declarationOpen)
            assertEquals(false, r.state.cash.lastOutcome?.success)
        }

    @Test
    fun theReportLabelsAreTheOnesOfTheToast() {
        assertEquals("X", reportLabel(ReportKind.X, 5L))
        assertEquals("X util.", reportLabel(ReportKind.USER_X, null))
        assertEquals("Z #5", reportLabel(ReportKind.Z, 5L))
        assertEquals("Z", reportLabel(ReportKind.Z, 0L))
        assertEquals("Z", reportLabel(ReportKind.Z, null))
    }

    // ── Payment correction flow (event F) ─────────────────────────

    private suspend fun TestScope.correctionRig(
        f: EngineFixture,
        payments: SalePayments = salePayments(5L),
    ): ReportsRig {
        coEvery { f.engine.listSalePayments(5L) } returns payments
        val r = start(f)
        r.model.openCorrection(
            TicketRow(5L, 5L, 4, 1000L, "2026-10-09T12:00:00", TicketKind.SALE, copyable(5L), null),
        )
        runCurrent()
        return r
    }

    @Test
    fun openingTheCorrectionReadsThePaymentLinesAndPicksTheOnlyOne() =
        runTest {
            val f = fixture()
            val r = correctionRig(f)

            val flow = r.state.correctionFlow!!
            assertEquals(5L, flow.fiscalEventId)
            assertEquals(0, flow.lineIndex)
            assertNull(flow.method)
            assertFalse(flow.canConfirm)
        }

    @Test
    fun withTwoFreeLinesNoneIsPickedUntilTheOperatorChooses() =
        runTest {
            val f = fixture()
            val r = correctionRig(f, salePayments(5L, listOf(paymentLine(0), paymentLine(1, "CARD_DEBIT"))))

            assertNull(r.state.correctionFlow?.lineIndex)

            r.model.pickCorrectionLine(1)
            r.model.pickCorrectionMethod(PaymentMethod.CASH)

            assertEquals(1, r.state.correctionFlow?.lineIndex)
            assertTrue(r.state.correctionFlow!!.canConfirm)
        }

    @Test
    fun aCardTheTerminalTookCannotBePicked() =
        runTest {
            val f = fixture()
            val r = correctionRig(f, salePayments(5L, listOf(paymentLine(0, "CARD_DEBIT", input = "AUTOMATIC"), paymentLine(1))))

            // The only free line is picked on its own; the terminal card never is.
            assertEquals(1, r.state.correctionFlow?.lineIndex)

            r.model.pickCorrectionLine(0)

            assertEquals(1, r.state.correctionFlow?.lineIndex)
        }

    @Test
    fun theMethodTheLineAlreadyHasIsNotAChoice() =
        runTest {
            val f = fixture()
            val r = correctionRig(f)

            r.model.pickCorrectionMethod(PaymentMethod.CASH)
            assertNull(r.state.correctionFlow?.method)

            r.model.pickCorrectionMethod(PaymentMethod.CARD)
            assertEquals(PaymentMethod.CARD, r.state.correctionFlow?.method)

            r.model.pickCorrectionMethod(PaymentMethod.ONLINE)
            assertEquals(PaymentMethod.CARD, r.state.correctionFlow?.method)
        }

    @Test
    fun theConfirmButtonAsksForThePinOnlyOnceALineAndAMethodArePicked() =
        runTest {
            val f = fixture()
            val r = correctionRig(f)

            r.model.askCorrectionPin()
            assertFalse(r.state.correctionFlow!!.askPin)

            r.model.pickCorrectionMethod(PaymentMethod.CARD)
            r.model.askCorrectionPin()
            assertTrue(r.state.correctionFlow!!.askPin)

            r.model.cancelCorrectionPin()
            assertFalse(r.state.correctionFlow!!.askPin)
        }

    @Test
    fun aFailingReadIsShownInTheDialog() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listSalePayments(5L) } throws refused
            val r = start(f)

            r.model.openCorrection(TicketRow(5L, 5L, 4, 1000L, "2026-10-09T12:00:00", TicketKind.SALE, copyable(5L), null))
            runCurrent()

            assertNotNull(r.state.correctionFlow?.loadFailure)
            assertNull(r.state.correctionFlow?.payments)
        }

    @Test
    fun thePinSignsTheCorrectionOfThePickedLineForTheManager() =
        runTest {
            val f = fixture()
            coEvery { f.engine.correctPayment(any(), any(), any(), any(), any()) } returns financial()
            val r = correctionRig(f)
            r.model.pickCorrectionMethod(PaymentMethod.MEAL_VOUCHER)

            r.model.correctPayment("1234")

            coVerify(exactly = 1) { f.engine.correctPayment(5L, 0, PaymentMethod.MEAL_VOUCHER, OPERATOR_INSZ, "1234") }
        }

    @Test
    fun aRefusedCorrectionIsThrownToThePad() =
        runTest {
            val f = fixture()
            coEvery { f.engine.correctPayment(any(), any(), any(), any(), any()) } returns
                financial(success = false, error = "payment_correction_period_closed")
            val r = correctionRig(f)
            r.model.pickCorrectionMethod(PaymentMethod.CARD)

            val failure = runCatching { r.model.correctPayment("1234") }.exceptionOrNull()

            assertNotNull(failure)
            assertNotNull(r.state.correctionFlow)
        }

    @Test
    fun aSignedCorrectionClosesTheDialogToastsAndReadsTheListsAgain() =
        runTest {
            val f = fixture()
            val r = correctionRig(f)

            r.model.correctionDone()
            runCurrent()

            assertNull(r.state.correctionFlow)
            assertTrue(r.toastTexts().contains(r.text("reports_pc_done")))
            coVerify(atLeast = 1) { f.engine.listCopyableTickets(any()) }
        }

    @Test
    fun closingTheCorrectionDropsTheFlow() =
        runTest {
            val f = fixture()
            val r = correctionRig(f)

            r.model.closeCorrection()

            assertNull(r.state.correctionFlow)
        }

    @Test
    fun aManagerSeesWhichSalesAlreadyHaveACorrection() =
        runTest {
            val f = fixture()
            val done = CorrectionDone("CASH", "CARD_DEBIT", 1000L, "2026-10-09T13:00:00")
            coEvery { f.engine.listCopyableTickets(any()) } returns listOf(copyable(5L))
            coEvery { f.engine.listSalePayments(5L) } returns salePayments(5L, corrected = done, correctable = false)
            val r = start(f)

            r.model.loadTickets()
            runCurrent()

            assertEquals(done, r.state.tickets.corrections[5L])
            assertTrue(r.state.tickets.canCorrect)
        }
}
