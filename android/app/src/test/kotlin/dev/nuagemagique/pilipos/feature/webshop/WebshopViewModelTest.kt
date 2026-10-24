package dev.nuagemagique.pilipos.feature.webshop

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.Attention
import uniffi.pos_ffi.HandoverRequest
import uniffi.pos_ffi.PatchStatusRequest
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RefundRequest
import uniffi.pos_ffi.SetConfirmedTimeRequest
import uniffi.pos_ffi.WebshopOrderSummary
import java.time.Instant

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class WebshopViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(vararg orders: WebshopOrderSummary): EngineFixture = webshopFixture(tmp.root, orders.toList())

    /** Taps the main button of order [id] and lets the action finish. */
    private fun TestScope.tap(
        r: WebshopRig,
        id: Long,
    ) {
        r.model.primary(r.order(id))
        runCurrent()
    }

    // ── Opening the screen ────────────────────────────────────────

    @Test
    fun withNobodyLoggedInTheScreenIsSignedOutAndTheListIsNotReadAgain() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)
            coVerify(exactly = 1) { f.engine.hubriseListWebshopOrders() }

            r.model.refresh()
            runCurrent()

            assertFalse(r.state.signedIn)
            coVerify(exactly = 1) { f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun openingTheScreenWithAUserReadsTheListAndShowsTheOperator() =
        runTest {
            val f = fixture(webshopOrder(1), webshopOrder(2, hubrise = "accepted"))
            val r = startWebshop(f, context)
            r.login()

            r.model.refresh()
            runCurrent()

            coVerify(exactly = 2) { f.engine.hubriseListWebshopOrders() }
            assertTrue(r.state.signedIn)
            assertTrue(r.state.isManager)
            assertEquals("Alice", r.state.operator)
            assertEquals(listOf(1L, 2L), r.state.orders.map { it.id })
            assertTrue(r.state.hubriseVisible)
        }

    @Test
    fun whileHubriseIsOffTheScreenReadsNothingAndSaysSo() =
        runTest {
            val f = webshopFixture(tmp.root, listOf(webshopOrder(1)), hubrise = false)
            val r = startWebshop(f, context)
            r.login()

            r.model.refresh()
            runCurrent()

            assertFalse(r.state.hubriseVisible)
            coVerify(exactly = 0) { f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun theEngineClosedGivesAnEmptySignedOutStateWithoutAnyCall() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context, open = false)

            r.model.refresh()
            runCurrent()

            assertFalse(r.state.signedIn)
            assertEquals(emptyList<WebshopOrderSummary>(), r.state.orders)
            coVerify(exactly = 0) { f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun aFailedReadShowsItsTextInTheState() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.hubriseListWebshopOrders() } throws PosException.Coded("hubrise_not_connected", emptyMap())

            r.model.refresh()
            runCurrent()

            assertTrue(r.state.error != null)
            assertEquals(1, r.state.orders.size)
        }

    // ── The primary action of every status ────────────────────────

    @Test
    fun aNewOrderIsAcceptedThroughTheStore() =
        runTest {
            val f = fixture(webshopOrder(1, source = "deliveroo", service = "collection"))
            val r = startWebshop(f, context)
            r.login()
            val request = slot<uniffi.pos_ffi.AcceptRequest>()
            coEvery { f.engine.webshopAcceptOrder(capture(request)) } returns accepted()

            tap(r, 1)

            assertEquals(1L, request.captured.orderId)
            assertEquals("2026-10-09T11:00:00Z", request.captured.confirmedTime)
        }

    @Test
    fun aReceivedOrderIsAcceptedToo() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "received"))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.webshopAcceptOrder(any()) } returns accepted()

            tap(r, 1)

            coVerify(exactly = 1) { f.engine.webshopAcceptOrder(any()) }
        }

    @Test
    fun anOrderToCheckHasNoPrimaryActionAndNothingIsSent() =
        runTest {
            val review = webshopState(1, "pending", resolution = "review", attention = listOf(Attention.NoItems))
            val f = fixture(webshopOrder(1, state = review))
            val r = startWebshop(f, context)
            r.login()

            tap(r, 1)

            assertNull(WebshopRules.nextAction(r.order(1), isManager = true))
            coVerify(exactly = 0) { f.engine.webshopAcceptOrder(any()) }
        }

    @Test
    fun anAcceptedOrderStartsPreparing() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"))
            val r = startWebshop(f, context)
            r.login()
            val request = slot<PatchStatusRequest>()
            coEvery { f.engine.hubrisePatchOrderStatus(capture(request)) } returns Unit

            tap(r, 1)

            assertEquals("h1", request.captured.hubriseOrderId)
            assertEquals("in_preparation", request.captured.newStatus)
        }

    @Test
    fun anOrderInPreparationIsMarkedReady() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "in_preparation", fiscal = "signed_n"))
            val r = startWebshop(f, context)
            r.login()
            val request = slot<PatchStatusRequest>()
            coEvery { f.engine.hubrisePatchOrderStatus(capture(request)) } returns Unit

            tap(r, 1)

            assertEquals("awaiting_collection", request.captured.newStatus)
        }

    @Test
    fun aReadyDeliveryOfTheOwnWebshopLeavesForDelivery() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "awaiting_collection", fiscal = "signed_n", source = "Webshop", service = "delivery"))
            val r = startWebshop(f, context)
            r.login()
            val request = slot<PatchStatusRequest>()
            coEvery { f.engine.hubrisePatchOrderStatus(capture(request)) } returns Unit

            tap(r, 1)

            assertEquals("in_delivery", request.captured.newStatus)
        }

    @Test
    fun aReadyPrepaidOrderOfAnAggregatorIsCompleted() =
        runTest {
            val f =
                fixture(webshopOrder(1, hubrise = "awaiting_collection", fiscal = "signed_n", source = "ubereats", service = "delivery"))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.webshopCompleteOrder(1L) } returns Unit

            tap(r, 1)

            coVerify(exactly = 1) { f.engine.webshopCompleteOrder(1L) }
        }

    @Test
    fun aReadyUnpaidOrderIsCollectedInCash() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "awaiting_collection", fiscal = "signed_p", totalCents = 1_850))
            val r = startWebshop(f, context)
            r.login()
            val request = slot<HandoverRequest>()
            coEvery { f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            tap(r, 1)

            assertEquals(PaymentMethod.CASH, request.captured.payments.single().method)
        }

    @Test
    fun anOrderInDeliveryIsCompletedOrCollectedByItsPayment() =
        runTest {
            val f =
                fixture(
                    webshopOrder(1, hubrise = "in_delivery", fiscal = "signed_n", source = "Webshop", service = "delivery"),
                    webshopOrder(2, hubrise = "in_delivery", fiscal = "signed_p", source = "Webshop", service = "delivery"),
                )
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.webshopCompleteOrder(1L) } returns Unit
            coEvery { f.engine.webshopHandoverOrder(any()) } returns sale()

            tap(r, 1)
            tap(r, 2)

            coVerify(exactly = 1) { f.engine.webshopCompleteOrder(1L) }
            coVerify(exactly = 1) { f.engine.webshopHandoverOrder(any()) }
        }

    @Test
    fun aCancelledAfterRegistrationOrderIsClosed() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "cancelled", fiscal = "cancel_pending"))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.webshopCloseCancelledOrder(1L) } returns sale()

            tap(r, 1)

            coVerify(exactly = 1) { f.engine.webshopCloseCancelledOrder(1L) }
        }

    @Test
    fun finishedOrdersHaveNoPrimaryActionAndSendNothing() =
        runTest {
            val f =
                fixture(
                    webshopOrder(1, hubrise = "completed", fiscal = "signed_n"),
                    webshopOrder(2, hubrise = "rejected", fiscal = "rejected"),
                    webshopOrder(3, hubrise = "cancelled", fiscal = "refunded"),
                    webshopOrder(4, hubrise = "mystery", fiscal = "signed_n"),
                )
            val r = startWebshop(f, context)
            r.login()

            listOf(1L, 2L, 3L, 4L).forEach { tap(r, it) }

            listOf(1L, 2L, 3L, 4L).forEach { assertNull(WebshopRules.nextAction(r.order(it), isManager = true)) }
            coVerify(exactly = 0) { f.engine.webshopAcceptOrder(any()) }
            coVerify(exactly = 0) { f.engine.hubrisePatchOrderStatus(any()) }
            coVerify(exactly = 0) { f.engine.webshopCompleteOrder(any()) }
            coVerify(exactly = 0) { f.engine.webshopHandoverOrder(any()) }
        }

    @Test
    fun anOrderWithoutAHubriseIdHasNoPrimaryAction() =
        runTest {
            val order = webshopOrder(1, hubrise = "accepted", fiscal = "signed_n").copy(hubriseOrderId = null)
            val f = fixture(order)
            val r = startWebshop(f, context)
            r.login()

            tap(r, 1)

            assertNull(WebshopRules.nextAction(order, isManager = true))
            coVerify(exactly = 0) { f.engine.hubrisePatchOrderStatus(any()) }
        }

    // ── Refund: a manager, after a confirmation ───────────────────

    @Test
    fun aRefundAsksTheManagerToConfirmThenSendsHisInsz() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "cancelled", fiscal = "refund_pending", totalCents = 2_450))
            val r = startWebshop(f, context)
            r.login(admin)
            val request = slot<RefundRequest>()
            coEvery { f.engine.webshopRefundOrder(capture(request)) } returns sale()

            tap(r, 1)
            assertEquals(WebshopDialog.RefundConfirm(1L), r.state.dialog)
            coVerify(exactly = 0) { f.engine.webshopRefundOrder(any()) }

            r.model.confirmRefund()
            runCurrent()

            assertNull(r.state.dialog)
            assertEquals(1L, request.captured.orderId)
            assertEquals(ADMIN_INSZ, request.captured.employeeInsz)
        }

    @Test
    fun aRefundNeedsAManagerSoACashierHasNoRefundButton() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "cancelled", fiscal = "refund_pending"))
            val r = startWebshop(f, context)
            r.login(cashier)

            tap(r, 1)

            assertFalse(r.state.isManager)
            assertNull(r.state.dialog)
            assertNull(WebshopRules.nextAction(r.order(1), isManager = r.state.isManager))
            coVerify(exactly = 0) { f.engine.webshopRefundOrder(any()) }
        }

    @Test
    fun aRefundConfirmedAfterTheManagerLeftIsRefusedWithAWarning() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "cancelled", fiscal = "refund_pending"))
            val r = startWebshop(f, context)
            r.login(admin)
            tap(r, 1)
            r.session.lock()
            runCurrent()

            r.model.confirmRefund()
            runCurrent()

            coVerify(exactly = 0) { f.engine.webshopRefundOrder(any()) }
            assertEquals(ToastVariant.Warning, r.lastToast().variant)
            assertEquals(r.text("webshop_manager_needed"), r.lastToast().message)
            assertNull(r.state.dialog)
        }

    @Test
    fun cancellingTheRefundDialogSendsNothing() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "cancelled", fiscal = "refund_pending"))
            val r = startWebshop(f, context)
            r.login(admin)
            tap(r, 1)

            r.model.dismissDialog()
            runCurrent()

            assertNull(r.state.dialog)
            coVerify(exactly = 0) { f.engine.webshopRefundOrder(any()) }
        }

    // ── Reject with a confirmation ────────────────────────────────

    @Test
    fun rejectingAsksFirstAndSendsOnlyOnConfirm() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.webshopRejectOrder(1L) } returns Unit

            r.model.askReject(r.order(1))
            runCurrent()
            assertEquals(WebshopDialog.RejectConfirm(1L), r.state.dialog)
            coVerify(exactly = 0) { f.engine.webshopRejectOrder(any()) }

            r.model.confirmReject()
            runCurrent()

            assertNull(r.state.dialog)
            coVerify(exactly = 1) { f.engine.webshopRejectOrder(1L) }
        }

    @Test
    fun aSignedOrderCannotBeRejected() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"))
            val r = startWebshop(f, context)
            r.login()

            r.model.askReject(r.order(1))
            runCurrent()

            assertNull(r.state.dialog)
            assertFalse(WebshopRules.canReject(r.order(1)))
        }

    @Test
    fun aRefusedRejectionIsShownAndTheBoardIsReadAgain() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.webshopRejectOrder(1L) } throws PosException.Coded("webshop_already_handled", emptyMap())

            r.model.askReject(r.order(1))
            r.model.confirmReject()
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            coVerify(exactly = 2) { f.engine.hubriseListWebshopOrders() }
        }

    // ── Delay ─────────────────────────────────────────────────────

    @Test
    fun theDelayDialogOpensOnAKitchenOrderAndPushesTheTimeForward() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "accepted", fiscal = "signed_n", confirmed = "2026-10-09T10:45:00Z"))
            val r = startWebshop(f, context)
            r.login()
            val request = slot<SetConfirmedTimeRequest>()
            coEvery { f.engine.hubriseSetConfirmedTime(capture(request)) } returns Unit

            r.model.askDelay(r.order(1))
            runCurrent()
            assertEquals(WebshopDialog.Delay(1L), r.state.dialog)

            r.model.delay(30)
            runCurrent()

            assertNull(r.state.dialog)
            assertEquals("h1", request.captured.hubriseOrderId)
            assertEquals("2026-10-09T11:15:00Z", request.captured.confirmedTime)
            assertEquals(listOf(15, 30, 45), DELAY_CHOICES)
        }

    @Test
    fun aNewOrderCannotBeDelayed() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)
            r.login()

            r.model.askDelay(r.order(1))
            runCurrent()

            assertNull(r.state.dialog)
        }

    @Test
    fun aDelayWithoutAnOpenDialogDoesNothing() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"))
            val r = startWebshop(f, context)
            r.login()

            r.model.delay(15)
            runCurrent()

            coVerify(exactly = 0) { f.engine.hubriseSetConfirmedTime(any()) }
        }

    @Test
    fun aDialogWhoseOrderLeftTheListSendsNothing() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)
            r.login()
            r.model.askReject(r.order(1))
            coEvery { f.engine.hubriseListWebshopOrders() } returns emptyList()
            r.model.refresh()
            runCurrent()

            r.model.confirmReject()
            runCurrent()

            coVerify(exactly = 0) { f.engine.webshopRejectOrder(any()) }
        }

    // ── Card, resync, prints, columns, clock ──────────────────────

    @Test
    fun collectingByCardSendsTheTotalOfTheOrderAsCard() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "awaiting_collection", fiscal = "signed_p", totalCents = 1_850))
            val r = startWebshop(f, context)
            r.login()
            val request = slot<HandoverRequest>()
            coEvery { f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            r.model.collectCard(r.order(1))
            runCurrent()

            val part = request.captured.payments.single()
            assertEquals(PaymentMethod.CARD, part.method)
            assertEquals(1_850L, part.amountCents)
        }

    @Test
    fun cardCollectionIsOnlyForAReadyUnpaidOrder() =
        runTest {
            val f =
                fixture(
                    webshopOrder(1, hubrise = "accepted", fiscal = "signed_p"),
                    webshopOrder(2, hubrise = "awaiting_collection", fiscal = "signed_n"),
                )
            val r = startWebshop(f, context)
            r.login()

            r.model.collectCard(r.order(1))
            r.model.collectCard(r.order(2))
            runCurrent()

            coVerify(exactly = 0) { f.engine.webshopHandoverOrder(any()) }
        }

    @Test
    fun resyncFetchesTheCatalogForAnOrderToCheckOnly() =
        runTest {
            val review = webshopState(1, "pending", resolution = "review", attention = listOf(Attention.NoItems))
            val f = fixture(webshopOrder(1, state = review), webshopOrder(2))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.hubriseSyncCatalog() } returns uniffi.pos_ffi.ApplyOutcome.Unchanged

            r.model.resync(r.order(2))
            runCurrent()
            coVerify(exactly = 0) { f.engine.hubriseSyncCatalog() }

            r.model.resync(r.order(1))
            runCurrent()
            coVerify(exactly = 1) { f.engine.hubriseSyncCatalog() }
        }

    @Test
    fun theReprintChoicesOpenOnTheFirstTapAndCloseOnTheSecond() =
        runTest {
            val f =
                fixture(
                    webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"),
                    webshopOrder(2, hubrise = "accepted", fiscal = "signed_n"),
                )
            val r = startWebshop(f, context)
            r.login()

            r.model.toggleReprint(r.order(1))
            runCurrent()
            assertEquals(1L, r.state.reprintOpenId)

            r.model.toggleReprint(r.order(2))
            runCurrent()
            assertEquals(2L, r.state.reprintOpenId)

            r.model.toggleReprint(r.order(2))
            runCurrent()
            assertNull(r.state.reprintOpenId)
        }

    @Test
    fun printingTheKitchenOrTheCustomerTicketClosesTheChoices() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.printWebshopKitchen(1L) } returns Unit
            coEvery { f.engine.printWebshopCustomer(1L) } returns Unit

            r.model.toggleReprint(r.order(1))
            r.model.printKitchen(r.order(1))
            runCurrent()
            assertNull(r.state.reprintOpenId)
            coVerify(exactly = 1) { f.engine.printWebshopKitchen(1L) }

            r.model.toggleReprint(r.order(1))
            r.model.printCustomer(r.order(1))
            runCurrent()
            assertNull(r.state.reprintOpenId)
            coVerify(exactly = 1) { f.engine.printWebshopCustomer(1L) }
            assertEquals(ToastVariant.Success, r.lastToast().variant)
        }

    @Test
    fun aMissingPrinterGivesAWarning() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.printWebshopKitchen(1L) } throws PosException.Coded("printer_not_configured", emptyMap())

            r.model.printKitchen(r.order(1))
            runCurrent()

            assertEquals(ToastVariant.Warning, r.lastToast().variant)
        }

    @Test
    fun theThirdColumnSwitchesBetweenReadyAndFinishedOrders() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)

            assertFalse(r.state.showDone)
            r.model.toggleDone()
            runCurrent()
            assertTrue(r.state.showDone)
            r.model.toggleDone()
            runCurrent()
            assertFalse(r.state.showDone)
        }

    @Test
    fun theClockMovesTheAgesEveryFifteenSeconds() =
        runTest {
            val f = fixture(webshopOrder(1))
            val r = startWebshop(f, context)
            val job = backgroundScope.launch { r.model.runClock() }
            val before = r.state.now
            r.clock.now = Instant.parse("2026-10-09T10:36:00Z")

            advanceTimeBy(WEBSHOP_CLOCK_TICK_MS + 1)
            runCurrent()

            assertEquals(NOW, before)
            assertEquals(Instant.parse("2026-10-09T10:36:00Z"), r.state.now)
            job.cancel()
        }

    // ── One action at a time ──────────────────────────────────────

    @Test
    fun aSecondTapWhileAnActionRunsIsIgnored() =
        runTest {
            val f =
                fixture(
                    webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"),
                    webshopOrder(2, hubrise = "accepted", fiscal = "signed_n"),
                )
            val r = startWebshop(f, context)
            r.login()
            val gate = CompletableDeferred<Unit>()
            coEvery { f.engine.hubrisePatchOrderStatus(any()) } coAnswers { gate.await() }

            r.model.primary(r.order(1))
            runCurrent()
            assertTrue(1L in r.state.busy)
            r.model.primary(r.order(1))
            r.model.primary(r.order(2))
            runCurrent()
            coVerify(exactly = 1) { f.engine.hubrisePatchOrderStatus(any()) }

            gate.complete(Unit)
            runCurrent()
            assertTrue(r.state.busy.isEmpty())

            r.model.primary(r.order(2))
            runCurrent()
            coVerify(exactly = 2) { f.engine.hubrisePatchOrderStatus(any()) }
        }

    @Test
    fun aRefusedActionShowsTheStoresErrorAndReleasesTheBusyMark() =
        runTest {
            val f = fixture(webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"))
            val r = startWebshop(f, context)
            r.login()
            coEvery { f.engine.hubrisePatchOrderStatus(any()) } throws PosException.Coded("hubrise_status_not_allowed", emptyMap())

            tap(r, 1)

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertTrue(r.state.error != null)
            assertTrue(r.state.busy.isEmpty())
        }
}
