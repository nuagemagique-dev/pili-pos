package dev.nuagemagique.pilipos.ui.scaffold

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.feature.order.OrderRoutes
import dev.nuagemagique.pilipos.feature.reports.ReportsRoutes
import dev.nuagemagique.pilipos.feature.settings.SettingsRoutes
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.feature.webshop.WebshopRoutes
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.LockOutcome
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class HeaderViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(hubrise: Boolean = false): EngineFixture = scaffoldFixture(tmp.root, hubrise)

    private suspend fun TestScope.start(
        f: EngineFixture,
        area: HeaderArea,
        open: Boolean = true,
    ): ScaffoldRig = startScaffold(f, context, area, open)

    private val refusal = PosException.Coded("pin_wrong", emptyMap())

    // ── State ─────────────────────────────────────────────────────

    @Test
    fun withNobodyLoggedInTheHeaderIsEmptyAndNothingNavigates() =
        runTest {
            val r = start(fixture(), HeaderArea.TABLES, open = false)

            assertEquals("", r.state.userName)
            assertNull(r.state.roleKey)
            assertFalse(r.state.isManager)
            assertNull(r.state.tableNumber)
            assertEquals(emptyList<HeaderEffect>(), r.effects)
        }

    @Test
    fun theStateFollowsTheOperatorAndTheirRole() =
        runTest {
            val r = start(fixture(), HeaderArea.TABLES)

            r.login(admin)
            runCurrent()
            assertEquals("Alice", r.state.userName)
            assertEquals("role_admin", r.state.roleKey)
            assertTrue(r.state.isManager)

            r.login(cashier)
            runCurrent()
            assertEquals("Bob", r.state.userName)
            assertEquals("role_cashier", r.state.roleKey)
            assertFalse(r.state.isManager)
        }

    @Test
    fun theTableNumberShowsOnTheOrderAndCheckoutScreensOnly() =
        runTest {
            val orderScreen = start(fixture(), HeaderArea.ORDER)
            val tablesScreen = HeaderViewModel(orderScreen.entry, HeaderArea.TABLES)
            orderScreen.login(admin)
            orderScreen.openOrder(order(7, table = 12))
            runCurrent()

            assertEquals(12, orderScreen.state.tableNumber)
            assertTrue(orderScreen.state.dineInOrder)
            assertTrue(orderScreen.state.tablesActive)
            assertFalse(orderScreen.state.takeawayActive)
            assertNull(tablesScreen.state.value.tableNumber)
            assertTrue(tablesScreen.state.value.tablesActive)
        }

    @Test
    fun aTakeawayOrderMarksTheTakeawayButtonAndShowsNoTable() =
        runTest {
            val r = start(fixture(), HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(8))
            runCurrent()

            assertNull(r.state.tableNumber)
            assertTrue(r.state.takeawayActive)
            assertFalse(r.state.tablesActive)
        }

    @Test
    fun theOnlineOrdersButtonFollowsTheHubriseSwitchAndTheBadgeTheWaitingOrders() =
        runTest {
            val withoutHubrise = start(fixture(), HeaderArea.TABLES)
            assertFalse(withoutHubrise.state.hubriseVisible)

            val f = fixture(hubrise = true)
            val r = start(f, HeaderArea.TABLES)
            runCurrent()

            assertTrue(r.state.hubriseVisible)
            assertEquals(0, r.state.pendingOrders)
        }

    // ── Workspaces and the leave guard ────────────────────────────

    @Test
    fun tablesFromTheOrderScreenRegistersTheOrderFirst() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(7, table = 3))

            r.model.goTables()
            runCurrent()

            coVerify(exactly = 1) { f.engine.leaveOrder(7, LeaveReason.NAVIGATE) }
            assertEquals(listOf<HeaderEffect>(HeaderEffect.Navigate(TablesRoutes.TABLES)), r.effects)
        }

    @Test
    fun aRefusedLeaveKeepsTheOperatorOnTheScreenAndShowsTheRegistrationError() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(7, table = 3))
            coEvery { f.engine.leaveOrder(7, LeaveReason.NAVIGATE) } throws refusal

            r.model.goTables()
            runCurrent()

            assertEquals(emptyList<HeaderEffect>(), r.effects)
            val toast = r.lastToast()
            assertEquals(ToastVariant.Error, toast.variant)
            val reason = r.errorTexts.describe(refusal, AppLanguage.FR)
            assertEquals(r.text("order_register_failed") + " : " + reason, toast.message)
            // Still on the order: it is the current one and the guard can be tried again.
            assertEquals(7L, r.order.current.value?.id)

            coEvery { f.engine.leaveOrder(7, LeaveReason.NAVIGATE) } returns SlipOutcome.NOTHING_TO_PRINT
            r.model.goTables()
            runCurrent()
            assertEquals(listOf<HeaderEffect>(HeaderEffect.Navigate(TablesRoutes.TABLES)), r.effects)
        }

    @Test
    fun theRefusalToastLastsSixSeconds() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.CHECKOUT)
            r.login(admin)
            r.openOrder(order(7, table = 3))
            coEvery { f.engine.leaveOrder(any(), any()) } throws refusal

            r.model.goTables()
            runCurrent()
            assertEquals(1, r.toasts.toasts.value.size)

            testScheduler.advanceTimeBy(REGISTER_FAILED_TOAST_MS - 1)
            runCurrent()
            assertEquals(1, r.toasts.toasts.value.size)
            testScheduler.advanceTimeBy(2)
            runCurrent()
            assertEquals(0, r.toasts.toasts.value.size)
        }

    @Test
    fun tablesFromTheTablesScreenDoesNothing() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)
            r.login(admin)

            r.model.goTables()
            runCurrent()

            assertEquals(emptyList<HeaderEffect>(), r.effects)
            coVerify(exactly = 0) { f.engine.leaveOrder(any(), any()) }
        }

    @Test
    fun leavingAnAreaWithoutAnOrderNeverTouchesTheEngine() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)

            r.model.goTables()
            runCurrent()

            coVerify(exactly = 0) { f.engine.leaveOrder(any(), any()) }
            assertEquals(listOf<HeaderEffect>(HeaderEffect.Navigate(TablesRoutes.TABLES)), r.effects)
        }

    @Test
    fun reportsAndSettingsAreForManagersOnly() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)
            r.login(cashier)

            r.model.goReports()
            r.model.goSettings()
            runCurrent()
            assertEquals(emptyList<HeaderEffect>(), r.effects)

            r.login(admin)
            r.model.goReports()
            runCurrent()
            r.model.goSettings()
            runCurrent()
            assertEquals(
                listOf<HeaderEffect>(HeaderEffect.Navigate(ReportsRoutes.REPORTS), HeaderEffect.Navigate(SettingsRoutes.SETTINGS)),
                r.effects,
            )
        }

    @Test
    fun theOnlineOrdersButtonOpensTheBoardOnlyWhileHubriseIsVisible() =
        runTest {
            val hidden = start(fixture(), HeaderArea.TABLES)
            hidden.login(admin)
            hidden.model.goOnlineOrders()
            runCurrent()
            assertEquals(emptyList<HeaderEffect>(), hidden.effects)

            val shown = start(fixture(hubrise = true), HeaderArea.TABLES)
            shown.login(admin)
            runCurrent()
            shown.model.goOnlineOrders()
            runCurrent()
            assertEquals(listOf<HeaderEffect>(HeaderEffect.Navigate(WebshopRoutes.WEBSHOP)), shown.effects)
        }

    @Test
    fun aSecondTapWhileTheFirstIsRunningIsIgnored() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(7, table = 3))
            val gate = CompletableDeferred<SlipOutcome>()
            coEvery { f.engine.leaveOrder(7, LeaveReason.NAVIGATE) } coAnswers { gate.await() }

            r.model.goTables()
            runCurrent()
            r.model.goTables()
            runCurrent()
            gate.complete(SlipOutcome.NOTHING_TO_PRINT)
            runCurrent()

            coVerify(exactly = 1) { f.engine.leaveOrder(7, LeaveReason.NAVIGATE) }
            assertEquals(1, r.effects.size)
        }

    // ── New takeaway ──────────────────────────────────────────────

    @Test
    fun aNewTakeawayRegistersTheOrderInHandStartsOneAndOpensIt() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)
            r.login(admin)
            r.openOrder(order(7, table = 3))
            coEvery { f.engine.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, ADMIN_INSZ)) } returns order(9)

            r.model.newTakeaway()
            runCurrent()

            coVerifyOrder {
                f.engine.leaveOrder(7, LeaveReason.SWITCH_ORDER)
                f.engine.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, ADMIN_INSZ))
            }
            assertEquals(listOf<HeaderEffect>(HeaderEffect.Navigate(OrderRoutes.order(9))), r.effects)
        }

    @Test
    fun aNewTakeawayWithoutAnOrderInHandStartsAtOnce() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)
            r.login(admin)
            coEvery { f.engine.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, ADMIN_INSZ)) } returns order(9)

            r.model.newTakeaway()
            runCurrent()

            coVerify(exactly = 0) { f.engine.leaveOrder(any(), any()) }
            assertEquals(listOf<HeaderEffect>(HeaderEffect.Navigate(OrderRoutes.order(9))), r.effects)
        }

    @Test
    fun aSecondTapOnTheTakeawayAlreadyOpenDoesNotReplaceTheCart() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(9))

            r.model.newTakeaway()
            runCurrent()

            coVerify(exactly = 0) { f.engine.createOrder(any()) }
            assertEquals(emptyList<HeaderEffect>(), r.effects)
        }

    @Test
    fun aNewTakeawayWhoseRegistrationIsRefusedStartsNothing() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(7, table = 3))
            coEvery { f.engine.leaveOrder(7, LeaveReason.SWITCH_ORDER) } throws refusal

            r.model.newTakeaway()
            runCurrent()

            coVerify(exactly = 0) { f.engine.createOrder(any()) }
            assertEquals(emptyList<HeaderEffect>(), r.effects)
            assertTrue(r.lastToast().message.startsWith(r.text("order_register_failed") + " : "))
        }

    @Test
    fun aNewTakeawayThatCannotStartShowsTheErrorAndStays() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)
            r.login(admin)
            coEvery { f.engine.createOrder(any()) } throws refusal

            r.model.newTakeaway()
            runCurrent()

            assertEquals(emptyList<HeaderEffect>(), r.effects)
            assertEquals(r.errorTexts.describe(refusal, AppLanguage.FR), r.lastToast().message)
            assertEquals(ToastVariant.Error, r.lastToast().variant)
        }

    @Test
    fun aNewTakeawayWithNobodyLoggedInDoesNothing() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)

            r.model.newTakeaway()
            runCurrent()

            coVerify(exactly = 0) { f.engine.createOrder(any()) }
            assertEquals(emptyList<HeaderEffect>(), r.effects)
        }

    // ── Lock ──────────────────────────────────────────────────────

    @Test
    fun lockLocksThroughTheEngineThenForgetsTheOrderThenGoesToLogin() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(7, table = 3))

            r.model.lock()
            runCurrent()

            coVerify(exactly = 1) { f.engine.lockSession() }
            // The engine registers the order itself while the session is open: no leave from the header.
            coVerify(exactly = 0) { f.engine.leaveOrder(any(), any()) }
            assertNull(r.order.current.value)
            assertNull(r.session.currentUser.value)
            assertEquals(listOf<HeaderEffect>(HeaderEffect.ToLogin), r.effects)
            assertEquals(0, r.toasts.toasts.value.size)
        }

    @Test
    fun lockShowsTheOrderErrorTheEngineReturnsAndStillLocks() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(7, table = 3))
            coEvery { f.engine.lockSession() } returns LockOutcome(orderError = "pin_locked|seconds=30")

            r.model.lock()
            runCurrent()

            val toast = r.lastToast()
            assertEquals(ToastVariant.Error, toast.variant)
            assertEquals(
                r.text("order_register_failed") + " : " + r.errorTexts.describe("pin_locked|seconds=30", AppLanguage.FR),
                toast.message,
            )
            assertNull(r.order.current.value)
            assertEquals(listOf<HeaderEffect>(HeaderEffect.ToLogin), r.effects)
        }

    @Test
    fun lockStillGoesToLoginWhenTheEngineIsClosed() =
        runTest {
            val r = start(fixture(), HeaderArea.TABLES, open = false)

            r.model.lock()
            runCurrent()

            assertEquals(listOf<HeaderEffect>(HeaderEffect.ToLogin), r.effects)
        }

    @Test
    fun lockClearsTheOrderOnlyAfterTheEngineAnswered() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.ORDER)
            r.login(admin)
            r.openOrder(order(7, table = 3))
            val gate = CompletableDeferred<LockOutcome>()
            coEvery { f.engine.lockSession() } coAnswers { gate.await() }

            r.model.lock()
            runCurrent()
            assertEquals(7L, r.order.current.value?.id)
            assertEquals(emptyList<HeaderEffect>(), r.effects)

            gate.complete(LockOutcome(orderError = null))
            runCurrent()
            assertNull(r.order.current.value)
            assertEquals(listOf<HeaderEffect>(HeaderEffect.ToLogin), r.effects)
        }

    // ── Own PIN ───────────────────────────────────────────────────

    @Test
    fun theUserButtonOpensTheOwnPinDialogOnlyWithAnOperator() =
        runTest {
            val r = start(fixture(), HeaderArea.TABLES)

            r.model.openPin()
            runCurrent()
            assertFalse(r.state.pinOpen)

            r.login(admin)
            r.model.openPin()
            runCurrent()
            assertTrue(r.state.pinOpen)

            r.model.closePin()
            runCurrent()
            assertFalse(r.state.pinOpen)
        }

    @Test
    fun theOwnPinIsSavedWithoutAnOperatorPinAndConfirmedByAToast() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)
            r.login(admin)
            coEvery { f.engine.setPin(ADMIN_INSZ, "4321", null) } returns Unit

            r.model.submitPin("4321")
            runCurrent()

            coVerify(exactly = 1) { f.engine.setPin(ADMIN_INSZ, "4321", null) }
            assertEquals(r.text("pin_changed"), r.lastToast().message)
            assertEquals(ToastVariant.Success, r.lastToast().variant)
        }

    @Test
    fun aRefusedPinPropagatesToTheDialog() =
        runTest {
            val f = fixture()
            val r = start(f, HeaderArea.TABLES)
            r.login(admin)
            coEvery { f.engine.setPin(any(), any(), any()) } throws refusal

            var failure: Throwable? = null
            try {
                r.model.submitPin("4321")
            } catch (e: PosException) {
                failure = e
            }

            assertEquals(refusal, failure)
            assertEquals(r.errorTexts.describe(refusal, AppLanguage.FR), r.model.describeError(refusal))
            assertEquals(0, r.toasts.toasts.value.size)
        }
}
