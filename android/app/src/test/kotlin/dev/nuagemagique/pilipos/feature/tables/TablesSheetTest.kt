package dev.nuagemagique.pilipos.feature.tables

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
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
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TableTransferResult
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole

/** The action sheet of a long pressed table (UX audit T7): the state holder side. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class TablesSheetTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val free = table(1)
    private val busy = table(2, orderId = 40, lines = 3, totalCents = 2_550, openedAt = "2026-10-09 10:00:00")
    private val waiter = User(insz = ADMIN_INSZ, name = "Wen", role = UserRole.WAITER, active = true, hasPin = true)

    private fun TestScope.fixture(): EngineFixture = tablesFixture(tmp.root, listOf(free, busy))

    private suspend fun TestScope.started(f: EngineFixture): TablesRig {
        val r = startTables(f, context)
        r.login()
        r.model.load()
        runCurrent()
        return r
    }

    @Test
    fun aLongPressOnAnOccupiedTableOpensItsSheet() =
        runTest {
            val r = started(fixture())

            r.model.openSheet(busy)

            assertEquals(busy, r.state.sheet?.table)
            assertTrue(r.state.sheet?.canTakePayment == true)
        }

    @Test
    fun aFreeTableHasNoSheet() =
        runTest {
            val r = started(fixture())

            r.model.openSheet(free)

            assertNull(r.state.sheet)
        }

    @Test
    fun theMoveModeHasNoSheet() =
        runTest {
            val r = started(fixture())
            r.model.toggleMove()

            r.model.openSheet(busy)

            assertNull(r.state.sheet)
        }

    @Test
    fun closingTheSheetDoesNothingElse() =
        runTest {
            val f = fixture()
            val r = started(f)
            r.model.openSheet(busy)

            r.model.closeSheet()
            runCurrent()

            assertNull(r.state.sheet)
            assertEquals(emptyList<TablesEffect>(), r.effects)
            coVerify(exactly = 0) { f.engine.signPreBill(any()) }
            coVerify(exactly = 0) { f.engine.transferTable(any(), any()) }
        }

    @Test
    fun aWaiterIsNotOfferedTheBillNorThePayment() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            coEvery { f.engine.login(ADMIN_INSZ, any()) } returns waiter
            r.session.login(ADMIN_INSZ, "1234")
            r.model.load()
            runCurrent()

            r.model.openSheet(busy)

            assertFalse(r.state.sheet?.canTakePayment == true)
            assertNotNull(r.state.sheet)
        }

    @Test
    fun ouvrirOpensTheTableLikeATap() =
        runTest {
            val f = fixture()
            val r = started(f)
            coEvery { f.engine.getOrder(40) } returns order(40, table = 2)
            r.model.openSheet(busy)

            r.model.sheetOpen()
            runCurrent()

            assertNull(r.state.sheet)
            coVerify(exactly = 1) { f.engine.getOrder(40) }
            assertEquals(listOf<TablesEffect>(TablesEffect.OpenOrder(40)), r.effects)
        }

    @Test
    fun additionPrintsTheProvisionalBillOfThatOrderAndStaysOnThePlan() =
        runTest {
            val f = fixture()
            val r = started(f)
            coEvery { f.engine.signPreBill(40) } returns SlipOutcome.PRINTED
            r.model.openSheet(busy)

            r.model.sheetBill()
            runCurrent()

            assertNull(r.state.sheet)
            coVerify(exactly = 1) { f.engine.signPreBill(40) }
            // The tables are read again: the table shows its bill tag.
            coVerify(exactly = 2) { f.engine.listTables() }
            assertEquals(ToastVariant.Success, r.lastToast().variant)
            assertEquals("Addition provisoire imprimée", r.lastToast().message)
            assertEquals(emptyList<TablesEffect>(), r.effects)
        }

    @Test
    fun aRefusedAdditionShowsTheError() =
        runTest {
            val f = fixture()
            val r = started(f)
            coEvery { f.engine.signPreBill(40) } throws PosException.Coded("order_not_open", emptyMap())
            r.model.openSheet(busy)

            r.model.sheetBill()
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertTrue(r.lastToast().message.startsWith("Addition non enregistrée"))
            assertEquals(emptyList<TablesEffect>(), r.effects)
        }

    @Test
    fun deplacerStartsTheMoveModeWithThisTableChosenThenOneTapMovesIt() =
        runTest {
            val f = fixture()
            val r = started(f)
            val moved = TableTransferResult(40L, 2, 1, SlipOutcome.PRINTED, SlipOutcome.PRINTED)
            coEvery { f.engine.transferTable(40L, 1) } returns moved
            r.model.openSheet(busy)

            r.model.sheetMove()

            assertNull(r.state.sheet)
            assertTrue(r.state.move.moving)
            assertEquals(busy, r.state.move.source)

            r.model.selectTable(free)
            runCurrent()

            coVerify(exactly = 1) { f.engine.transferTable(40L, 1) }
            assertEquals("Table 2 déplacée en 1", r.lastToast().message)
            assertEquals("Annuler", r.lastToast().action?.label)
            assertFalse(r.state.move.moving)
        }

    @Test
    fun payerLoadsTheOrderAndOpensItsCheckout() =
        runTest {
            val f = fixture()
            val r = started(f)
            coEvery { f.engine.getOrder(40) } returns order(40, table = 2)
            r.model.openSheet(busy)

            r.model.sheetPay()
            runCurrent()

            assertNull(r.state.sheet)
            coVerify(exactly = 1) { f.engine.getOrder(40) }
            coVerify(exactly = 1) { f.engine.setActiveOrder(40) }
            assertEquals(listOf<TablesEffect>(TablesEffect.OpenCheckout(40)), r.effects)
        }

    @Test
    fun aRefusedPaymentLoadStaysOnThePlanWithAnError() =
        runTest {
            val f = fixture()
            val r = started(f)
            coEvery { f.engine.getOrder(40) } throws PosException.Coded("order_not_open", emptyMap())
            r.model.openSheet(busy)

            r.model.sheetPay()
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertEquals(emptyList<TablesEffect>(), r.effects)
        }
}
