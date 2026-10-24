package dev.nuagemagique.pilipos.feature.tables

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.EventFeed
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome
import java.time.Instant

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class TablesViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val free = table(1)
    private val busy = table(2, orderId = 40, lines = 3, totalCents = 2_550, openedAt = "2026-10-09 10:00:00")
    private val resumable = takeaway(orderId = 50, lines = 2, totalCents = 1_800)
    private val refusal = PosException.Coded("order_not_open", emptyMap())

    private fun TestScope.fixture(
        tables: List<uniffi.pos_ffi.TableInfo> = listOf(free, busy),
        takeaways: List<uniffi.pos_ffi.TakeawayInfo> = listOf(resumable),
    ): EngineFixture = tablesFixture(tmp.root, tables, takeaways)

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun withNobodyLoggedInNothingIsListedAndTheScreenIsSignedOut() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)

            assertFalse(r.state.signedIn)
            assertEquals(emptyList<uniffi.pos_ffi.TableInfo>(), r.state.tables)
            coVerify(exactly = 0) { f.engine.listTables() }
            assertEquals(emptyList<TablesEffect>(), r.effects)
        }

    @Test
    fun loadingListsTheTablesAndTheOpenTakeaways() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()

            r.model.load()
            runCurrent()

            assertTrue(r.state.signedIn)
            assertEquals(listOf(free, busy), r.state.tables)
            assertEquals(listOf(resumable), r.state.takeaways)
            assertTrue(r.state.loaded)
        }

    @Test
    fun theOrdersWithAChargedCardAreMarked() =
        runTest {
            val f = fixture()
            val charged = uniffi.pos_ffi.UnsettledCardPayment(1L, 40L, "dine_in", 2, 1_700L, "Bancontact", "2026-10-13 12:00:00")
            val takeawayCharged = charged.copy(paymentId = 2L, orderId = 50L, orderType = "takeaway", tableNumber = null)
            coEvery { f.engine.listUnsettledCardPayments() } returns listOf(charged, takeawayCharged)
            val r = startTables(f, context)
            r.login()

            r.model.load()
            runCurrent()

            assertEquals(setOf(40L, 50L), r.state.cardCharged)
        }

    @Test
    fun withoutAnAnswerForTheCardsThereIsNoBadgeAndTheListsStillLoad() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listUnsettledCardPayments() } throws refusal
            val r = startTables(f, context)
            r.login()

            r.model.load()
            runCurrent()

            assertTrue(r.state.cardCharged.isEmpty())
            assertEquals(listOf(free, busy), r.state.tables)
            assertTrue(r.state.loaded)
        }

    @Test
    fun theEngineTurningReadyAfterTheLoginLoadsTheLists() =
        runTest {
            val f = fixture()
            val r = startTables(f, context, open = false)
            assertFalse(r.state.loaded)

            f.holder.open()
            runCurrent()
            r.login()
            r.model.load()
            runCurrent()

            assertEquals(listOf(free, busy), r.state.tables)
        }

    @Test
    fun aResyncLoadsTheListsAgain() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = startTables(f, context)
            r.login()
            r.model.load()
            runCurrent()
            coEvery { f.engine.listTables() } returns listOf(free)

            feed.send(PosEvent.Resync(missed = 2L))
            runCurrent()

            assertEquals(listOf(free), r.state.tables)
        }

    @Test
    fun aFailingLoadKeepsTheListsAsTheyWere() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            r.model.load()
            runCurrent()
            coEvery { f.engine.listTables() } throws refusal

            r.model.load()
            runCurrent()

            assertEquals(listOf(free, busy), r.state.tables)
            assertEquals(0, r.toasts.toasts.value.size)
        }

    @Test
    fun theMinutesClockMovesEvery30Seconds() =
        runTest {
            val r = startTables(fixture(), context)
            backgroundScope.launch { r.model.runClock() }
            val first = r.state.now

            r.clock.now = Instant.parse("2026-10-09T10:31:00Z")
            testScheduler.advanceTimeBy(CLOCK_TICK_MS - 1)
            runCurrent()
            assertEquals(first, r.state.now)

            testScheduler.advanceTimeBy(2)
            runCurrent()
            assertEquals(Instant.parse("2026-10-09T10:31:00Z"), r.state.now)
        }

    // ── Opening an order ──────────────────────────────────────────

    @Test
    fun aFreeTableStartsAnOrderAndOpensIt() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            coEvery { f.engine.createOrder(CreateOrderInput(OrderType.DINE_IN, 1, ADMIN_INSZ)) } returns order(41, table = 1)

            r.model.selectTable(free)
            runCurrent()

            coVerify(exactly = 1) { f.engine.createOrder(CreateOrderInput(OrderType.DINE_IN, 1, ADMIN_INSZ)) }
            assertEquals(listOf<TablesEffect>(TablesEffect.OpenOrder(41)), r.effects)
        }

    @Test
    fun anOccupiedTableLoadsItsOrderAndItsLines() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            coEvery { f.engine.getOrder(40) } returns order(40, table = 2)

            r.model.selectTable(busy)
            runCurrent()

            coVerify(exactly = 1) { f.engine.getOrder(40) }
            coVerify(exactly = 1) { f.engine.listCartLines(40) }
            coVerify(exactly = 0) { f.engine.createOrder(any()) }
            assertEquals(listOf<TablesEffect>(TablesEffect.OpenOrder(40)), r.effects)
        }

    @Test
    fun anOpenTakeawayIsResumed() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            coEvery { f.engine.getOrder(50) } returns order(50)

            r.model.resumeTakeaway(resumable)
            runCurrent()

            coVerify(exactly = 1) { f.engine.getOrder(50) }
            assertEquals(listOf<TablesEffect>(TablesEffect.OpenOrder(50)), r.effects)
        }

    @Test
    fun aNewTakeawayStartsATakeawayOrder() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            coEvery { f.engine.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, ADMIN_INSZ)) } returns order(60)

            r.model.newTakeaway()
            runCurrent()

            assertEquals(listOf<TablesEffect>(TablesEffect.OpenOrder(60)), r.effects)
        }

    @Test
    fun aRefusalShowsAnErrorToastAndStaysOnTheScreen() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            coEvery { f.engine.getOrder(40) } throws refusal

            r.model.selectTable(busy)
            runCurrent()

            assertEquals(emptyList<TablesEffect>(), r.effects)
            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertEquals(r.errorTexts.describe(refusal, FR), r.lastToast().message)
        }

    @Test
    fun withNobodyLoggedInTapsDoNothing() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)

            r.model.selectTable(free)
            r.model.newTakeaway()
            r.model.resumeTakeaway(resumable)
            runCurrent()

            coVerify(exactly = 0) { f.engine.createOrder(any()) }
            coVerify(exactly = 0) { f.engine.getOrder(any()) }
            assertEquals(emptyList<TablesEffect>(), r.effects)
        }

    @Test
    fun aSecondTapWhileTheFirstIsRunningIsIgnored() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            val gate = CompletableDeferred<uniffi.pos_ffi.Order>()
            coEvery { f.engine.createOrder(any()) } coAnswers { gate.await() }

            r.model.newTakeaway()
            runCurrent()
            r.model.newTakeaway()
            runCurrent()
            gate.complete(order(60))
            runCurrent()

            coVerify(exactly = 1) { f.engine.createOrder(any()) }
            assertEquals(listOf<TablesEffect>(TablesEffect.OpenOrder(60)), r.effects)
        }

    // ── Moving a table ────────────────────────────────────────────

    private val moved = uniffi.pos_ffi.TableTransferResult(40L, 2, 1, SlipOutcome.PRINTED, SlipOutcome.PRINTED)

    @Test
    fun theMoveModeOnlyLetsAnOccupiedTableBeChosenThenAFreeOne() =
        runTest {
            val r = startTables(fixture(), context)
            r.login()
            r.model.load()
            runCurrent()

            r.model.toggleMove()
            r.model.selectTable(free)
            assertEquals(null, r.state.move.source)
            r.model.selectTable(busy)
            assertEquals(busy, r.state.move.source)
            r.model.selectTable(busy)
            assertEquals(null, r.state.move.target)
            r.model.selectTable(free)
            assertEquals(free, r.state.move.target)
            coVerify(exactly = 0) { r.f.engine.getOrder(any()) }
            assertEquals(emptyList<TablesEffect>(), r.effects)
        }

    @Test
    fun leavingTheMoveModeForgetsWhatWasChosen() =
        runTest {
            val r = startTables(fixture(), context)
            r.login()
            r.model.toggleMove()
            r.model.selectTable(busy)
            r.model.selectTable(free)
            r.model.cancelTarget()
            assertEquals(busy, r.state.move.source)
            assertEquals(null, r.state.move.target)

            r.model.toggleMove()

            assertFalse(r.state.move.moving)
            assertEquals(null, r.state.move.source)
        }

    @Test
    fun confirmingTheMoveSignsItShowsTheToastAndReadsTheTablesAgain() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            r.model.load()
            runCurrent()
            coEvery { f.engine.transferTable(40L, 1) } returns moved
            r.model.toggleMove()
            r.model.selectTable(busy)
            r.model.selectTable(free)

            r.model.confirmMove()
            runCurrent()

            coVerify(exactly = 1) { f.engine.transferTable(40L, 1) }
            coVerify(exactly = 2) { f.engine.listTables() }
            assertEquals(ToastVariant.Success, r.lastToast().variant)
            assertFalse(r.state.move.moving)
        }

    @Test
    fun aRefusedMoveShowsTheErrorAndLeavesTheMode() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            val busyTarget = PosException.Coded("table_transfer_target_busy", emptyMap())
            coEvery { f.engine.transferTable(any(), any()) } throws busyTarget
            r.model.toggleMove()
            r.model.selectTable(busy)
            r.model.selectTable(free)

            r.model.confirmMove()
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertEquals(r.errorTexts.describe(busyTarget, FR), r.lastToast().message)
            assertFalse(r.state.move.moving)
        }

    @Test
    fun confirmingWithoutATargetDoesNothing() =
        runTest {
            val f = fixture()
            val r = startTables(f, context)
            r.login()
            r.model.toggleMove()
            r.model.selectTable(busy)

            r.model.confirmMove()
            runCurrent()

            coVerify(exactly = 0) { f.engine.transferTable(any(), any()) }
        }
}
