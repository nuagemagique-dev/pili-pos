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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.MoneyDirection
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException

@OptIn(ExperimentalCoroutinesApi::class)
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

            val starting = r.model.describeError(EngineUnavailableException(EngineState.Opening))
            assertEquals(starting, r.state.day.lastReport?.failure)
            assertEquals(starting, r.state.cash.lastOutcome?.failure)
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
}
