package dev.nuagemagique.pilipos.feature.reports

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.coVerifyOrder
import kotlinx.coroutines.CompletableDeferred
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
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.UserRole

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class SalesViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private suspend fun TestScope.started(f: EngineFixture = reportsFixture(tmp.root)): Pair<EngineFixture, ReportsRig> {
        coEvery { f.engine.salesReport(any(), any()) } answers { salesReport(from = firstArg(), to = secondArg()) }
        return f to startReports(f, context)
    }

    private fun ReportsRig.type(digits: String) {
        digits.forEach { salesModel.onKey(NumPadKey.Digit(it)) }
    }

    // ── Presets ───────────────────────────────────────────────────

    @Test
    fun openShowsTodayFromTheBusinessDateTheEngineGives() =
        runTest {
            val (f, r) = started()

            r.salesModel.open()
            runCurrent()

            coVerify(exactly = 1) { f.engine.salesReport("2026-10-17", "2026-10-17") }
            assertEquals(SalesPreset.TODAY, r.sales.preset)
            assertEquals("2026-10-17", r.sales.from)
            assertEquals(3, r.sales.report?.days?.size)
            assertFalse(r.sales.loading)
            assertNull(r.sales.error)
        }

    @Test
    fun aServicePastMidnightIsStillTodayOnceTheEngineSaysSo() =
        runTest {
            val f = reportsFixture(tmp.root)
            // The engine's business date is the day before the calendar date of the clock below.
            coEvery { f.engine.salesReport(any(), any()) } answers {
                salesReport(from = firstArg(), to = secondArg(), businessDate = "2026-10-16")
            }
            val r = startReports(f, context)

            r.salesModel.open()
            runCurrent()

            coVerifyOrder {
                f.engine.salesReport("2026-10-17", "2026-10-17")
                f.engine.salesReport("2026-10-16", "2026-10-16")
            }
            assertEquals("2026-10-16", r.sales.from)
            assertEquals("2026-10-16", r.sales.businessDate)
        }

    @Test
    fun theOtherPresetsAskForYesterdayTheWeekFromMondayAndTheMonthFromTheFirst() =
        runTest {
            val (f, r) = started()
            r.salesModel.open()
            runCurrent()

            r.salesModel.pick(SalesPreset.YESTERDAY)
            runCurrent()
            assertEquals("2026-10-16" to "2026-10-16", r.sales.from to r.sales.to)

            r.salesModel.pick(SalesPreset.WEEK)
            runCurrent()
            assertEquals("2026-10-12" to "2026-10-17", r.sales.from to r.sales.to)

            r.salesModel.pick(SalesPreset.MONTH)
            runCurrent()
            assertEquals("2026-10-01" to "2026-10-17", r.sales.from to r.sales.to)
            coVerify { f.engine.salesReport("2026-10-12", "2026-10-17") }
        }

    // ── Custom range ──────────────────────────────────────────────

    @Test
    fun aTypedRangeIsAskedForOnceBothDatesAreReal() =
        runTest {
            val (f, r) = started()
            r.salesModel.pick(SalesPreset.CUSTOM)
            runCurrent()
            assertFalse(r.sales.canApply)

            r.type("01102026")
            // The start date is complete: the keypad moves to the end date.
            assertEquals(SalesField.TO, r.sales.field)
            r.type("1510")
            assertFalse(r.sales.canApply)
            r.type("2026")
            assertTrue(r.sales.canApply)
            assertEquals("2026-10-01", r.sales.typedFrom)
            assertEquals("2026-10-15", r.sales.typedTo)

            r.salesModel.applyCustom()
            runCurrent()

            coVerify { f.engine.salesReport("2026-10-01", "2026-10-15") }
            assertEquals("2026-10-15", r.sales.to)
        }

    @Test
    fun theKeysEraseAndStopAtEightDigits() =
        runTest {
            val (_, r) = started()
            r.salesModel.pick(SalesPreset.CUSTOM)
            r.salesModel.setField(SalesField.TO)

            r.type("1234567890")
            assertEquals("12345678", r.sales.toDigits)
            r.salesModel.onKey(NumPadKey.Backspace)
            assertEquals("1234567", r.sales.toDigits)
            r.salesModel.onKey(NumPadKey.Clear)
            assertEquals("", r.sales.toDigits)
        }

    @Test
    fun anImpossibleDateKeepsShowDisabledAndAsksNothing() =
        runTest {
            val (f, r) = started()
            r.salesModel.pick(SalesPreset.CUSTOM)

            r.type("31022026")
            r.type("01032026")

            assertNull(r.sales.typedFrom)
            assertFalse(r.sales.canApply)
            r.salesModel.applyCustom()
            runCurrent()
            coVerify(exactly = 0) { f.engine.salesReport(any(), any()) }
        }

    @Test
    fun theEngineRefusingTheRangeIsShownInFrench() =
        runTest {
            val f = reportsFixture(tmp.root)
            coEvery { f.engine.salesReport(any(), any()) } throws PosException.Coded("sales_report_range_invalid", emptyMap())
            val r = startReports(f, context)

            r.salesModel.open()
            runCurrent()

            assertNull(r.sales.report)
            assertTrue(r.sales.error.orEmpty().contains("92 jours"))
            assertFalse(r.sales.loading)
        }

    @Test
    fun withTheEngineNotReadyTheTabIsEmptyWithoutACrash() =
        runTest {
            val f = reportsFixture(tmp.root)
            val r = startReports(f, context, open = false)

            r.salesModel.open()
            runCurrent()

            assertNull(r.sales.report)
            assertNotNull(r.sales.error)
        }

    // ── Tables ────────────────────────────────────────────────────

    @Test
    fun theProductsAreSortedByAmountThenByQuantity() =
        runTest {
            val (_, r) = started()
            r.salesModel.open()
            runCurrent()

            assertEquals(listOf("Maki saumon", "Sashimi", "Soupe"), r.sales.products.map { it.name })
            r.salesModel.setSort(SalesSort.QUANTITY)
            assertEquals(listOf("Sashimi", "Maki saumon", "Soupe"), r.sales.products.map { it.name })
        }

    // ── Printing ──────────────────────────────────────────────────

    @Test
    fun printSendsTheShownRangeAndTellsSo() =
        runTest {
            val (f, r) = started()
            coEvery { f.engine.printSalesReport(any(), any()) } returns Unit
            r.salesModel.pick(SalesPreset.WEEK)
            runCurrent()

            r.salesModel.print()
            runCurrent()

            coVerify { f.engine.printSalesReport("2026-10-12", "2026-10-17") }
            assertTrue(r.toastTexts().contains(r.text("sales_printed")))
            assertFalse(r.sales.printing)
        }

    @Test
    fun aPrinterProblemIsShownNotHidden() =
        runTest {
            val (f, r) = started()
            coEvery { f.engine.printSalesReport(any(), any()) } throws PosException.Coded("printer_not_configured", emptyMap())
            r.salesModel.open()
            runCurrent()

            r.salesModel.print()
            runCurrent()

            assertFalse(r.toastTexts().contains(r.text("sales_printed")))
            assertEquals(1, r.toastTexts().size)
            assertFalse(r.sales.printing)
        }

    @Test
    fun printDoesNothingBeforeAReportIsShown() =
        runTest {
            val (f, r) = started()

            r.salesModel.print()
            runCurrent()

            coVerify(exactly = 0) { f.engine.printSalesReport(any(), any()) }
        }

    @Test
    fun theLoadingNoticeLastsUntilTheReportArrives() =
        runTest {
            val f = reportsFixture(tmp.root)
            val gate = CompletableDeferred<Unit>()
            coEvery { f.engine.salesReport(any(), any()) } coAnswers
                {
                    gate.await()
                    salesReport(from = firstArg(), to = secondArg())
                }
            val r = startReports(f, context)

            r.salesModel.open()
            runCurrent()
            assertTrue(r.sales.loading)
            assertNull(r.sales.report)

            gate.complete(Unit)
            runCurrent()
            assertFalse(r.sales.loading)
            assertNotNull(r.sales.report)
        }

    // V11-D8: the sales report is for managers; the tab is shown to everybody (as on the Vue screen) and the
    // engine's refusal is what a cashier reads.
    @Test
    fun aCashierGetsTheEngineRefusalAndCannotPrint() =
        runTest {
            val f = reportsFixture(tmp.root)
            coEvery { f.engine.login(any(), any()) } returns operatorUser.copy(role = UserRole.CASHIER)
            coEvery { f.engine.salesReport(any(), any()) } throws PosException.Coded("forbidden", emptyMap())
            coEvery { f.engine.printSalesReport(any(), any()) } throws PosException.Coded("forbidden", emptyMap())
            val r = startReports(f, context)

            r.salesModel.open()
            runCurrent()

            assertNull(r.sales.report)
            assertFalse(r.sales.loading)
            assertEquals(r.text("err_forbidden"), r.sales.error)
            assertEquals("Action réservée à un autre profil.", r.sales.error)

            // Nothing is shown, so there is nothing to print: no engine call is made.
            r.salesModel.print()
            runCurrent()
            coVerify(exactly = 0) { f.engine.printSalesReport(any(), any()) }
        }
}
