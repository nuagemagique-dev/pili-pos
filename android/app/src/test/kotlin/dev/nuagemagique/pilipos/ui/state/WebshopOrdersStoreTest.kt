package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.feature.checkout.CheckoutNatives
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.every
import io.mockk.mockk
import io.mockk.slot
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
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
import uniffi.pos_ffi.AcceptRequest
import uniffi.pos_ffi.AcceptResult
import uniffi.pos_ffi.ApplyOutcome
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.HandoverRequest
import uniffi.pos_ffi.PatchStatusRequest
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RefundRequest
import uniffi.pos_ffi.SetConfirmedTimeRequest
import uniffi.pos_ffi.WebshopOrderSummary
import uniffi.pos_ffi.WebshopState
import java.time.Clock
import java.time.Instant
import java.time.ZoneOffset

@Suppress("LargeClass", "TooManyFunctions", "LongParameterList")
@RunWith(RobolectricTestRunner::class)
class WebshopOrdersStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val now = Instant.parse("2026-10-09T10:00:00Z")

    private fun state(
        fiscal: String = "pending",
        orderId: Long = 1,
    ) = WebshopState(
        orderId = orderId,
        hubriseOrderId = "h$orderId",
        channel = "Web",
        channelKind = null,
        serviceType = null,
        paymentMode = null,
        paymentType = null,
        fiscalState = fiscal,
        resolution = "ok",
        attention = emptyList(),
        warnings = emptyList(),
        lines = emptyList(),
    )

    private fun order(
        id: Long,
        hubrise: String? = "new",
        fiscal: String? = "pending",
        source: String = "deliveroo",
        service: String? = "collection",
        confirmed: String? = null,
        totalCents: Long = 2_000,
    ) = WebshopOrderSummary(
        id = id,
        hubriseOrderId = "h$id",
        status = "open",
        hubriseStatus = hubrise,
        source = source,
        serviceType = service,
        totalCents = totalCents,
        createdAt = "2026-10-09T09:55:00Z",
        items = emptyList(),
        customerNotes = null,
        customerName = "Client",
        confirmedTime = confirmed,
        customerAddress1 = null,
        customerAddress2 = null,
        customerPostalCode = null,
        customerCity = null,
        webshop = fiscal?.let { state(it, id) },
    )

    private fun sale(
        success: Boolean = true,
        error: String? = null,
    ) = CheckoutResult(
        success = success,
        ticket = null,
        error = error,
        fdmConnected = true,
        fiscalEventId = null,
        delivery = null,
        kitchenSlip = null,
    )

    private class Rig(
        val f: EngineFixture,
        val feed: EventFeed,
        val store: WebshopOrdersStore,
        val toasts: ToastStore,
        val eft: EftStore,
    )

    /** An engine that is ready, HubRise [visible] unless told otherwise, [orders] as the first answer of the list. */
    private suspend fun TestScope.rig(
        orders: List<WebshopOrderSummary> = emptyList(),
        visible: Boolean = true,
        open: Boolean = true,
        terminal: Boolean = false,
    ): Rig {
        val f = EngineFixture(this, tmp.root)
        val feed = EventFeed(f)
        coEvery { f.engine.getIntegrations() } returns integrations(enabled = visible)
        coEvery { f.engine.hubriseListWebshopOrders() } returns orders
        val gateway = f.gateway(this)
        val toasts = ToastStore(backgroundScope)
        val settings =
            mockk<PosSettingsStore> {
                every { language } returns MutableStateFlow(AppLanguage.EN)
                every { this@mockk.settings } returns MutableStateFlow(if (terminal) linkedSettings() else null)
                every { trainingMode } returns MutableStateFlow(false)
            }
        val errors = ErrorPresenter(toasts, I18nTestSupport.errorTexts())
        val eft = EftStore(gateway, errors, settings, backgroundScope)
        val store =
            WebshopOrdersStore(
                gateway,
                f.events,
                errors,
                toasts,
                IntegrationsStore(gateway, f.events, backgroundScope),
                settings,
                I18nTestSupport.catalog(),
                eft,
                EngineHelpers(CheckoutNatives(), UnconfinedTestDispatcher(testScheduler)),
                backgroundScope,
            )
        store.clock = Clock.fixed(now, ZoneOffset.UTC)
        if (open) {
            f.holder.open()
            runCurrent()
        }
        return Rig(f, feed, store, toasts, eft)
    }

    private fun Rig.lastToast() = toasts.toasts.value.last()

    @Test
    fun nothingIsFetchedBeforeTheEngineIsReady() =
        runTest {
            val r = rig(listOf(order(1)), open = false)
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.hubriseListWebshopOrders() }
            assertTrue(r.store.orders.value.isEmpty())
            assertFalse(r.store.loaded.value)
        }

    @Test
    fun theOrdersAreFetchedWhenTheEngineIsReadyAndHubriseIsVisible() =
        runTest {
            val r = rig(listOf(order(1), order(2, hubrise = "accepted")))

            coVerify(exactly = 1) { r.f.engine.hubriseListWebshopOrders() }
            assertEquals(listOf(1L, 2L), r.store.orders.value.map { it.id })
            assertTrue(r.store.loaded.value)
            assertFalse(r.store.loading.value)
            assertNull(r.store.lastError.value)
        }

    @Test
    fun nothingIsFetchedWhileHubriseIsHidden() =
        runTest {
            val r = rig(listOf(order(1)), visible = false)

            coVerify(exactly = 0) { r.f.engine.hubriseListWebshopOrders() }
            assertTrue(r.store.orders.value.isEmpty())
            assertEquals(0, r.store.pendingCount.value)
        }

    @Test
    fun theOrdersAreFetchedAsSoonAsHubriseBecomesVisible() =
        runTest {
            val r = rig(listOf(order(1)), visible = false)
            coEvery { r.f.engine.getIntegrations() } returns integrations(enabled = true)

            r.feed.send(PosEvent.Resync(missed = 1L))
            runCurrent()

            assertEquals(1, r.store.orders.value.size)
        }

    @Test
    fun aReceivedAnUpdatedAndAResyncEventFetchAgain() =
        runTest {
            val r = rig(listOf(order(1)))
            coVerify(exactly = 1) { r.f.engine.hubriseListWebshopOrders() }

            r.feed.send(PosEvent.WebshopOrderReceived(mockk(relaxed = true)))
            runCurrent()
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }

            r.feed.send(PosEvent.WebshopOrderUpdated(mockk(relaxed = true)))
            runCurrent()
            coVerify(exactly = 3) { r.f.engine.hubriseListWebshopOrders() }

            r.feed.send(PosEvent.Resync(missed = 2L))
            runCurrent()
            coVerify(exactly = 4) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun anEventWhileHubriseIsHiddenFetchesNothing() =
        runTest {
            val r = rig(listOf(order(1)), visible = false)

            r.feed.send(PosEvent.WebshopOrderReceived(mockk(relaxed = true)))
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun otherEventsFetchNothing() =
        runTest {
            val r = rig(listOf(order(1)))

            r.feed.send(PosEvent.CatalogUpdated(mockk(relaxed = true)))
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun aFailedFetchKeepsTheListAndShowsTheError() =
        runTest {
            val r = rig(listOf(order(1)))
            coEvery { r.f.engine.hubriseListWebshopOrders() } throws PosException.Coded("hubrise_disabled", emptyMap())

            val ok = r.store.fetchOrders()

            assertFalse(ok)
            assertEquals(1, r.store.orders.value.size)
            assertNotNull(r.store.lastError.value)
            assertFalse(r.store.loading.value)
            assertTrue(r.toasts.toasts.value.isEmpty())
        }

    @Test
    fun theEngineGoingAwayEmptiesTheBoard() =
        runTest {
            val r = rig(listOf(order(1)))
            assertEquals(1, r.store.pendingCount.value)

            r.f.holder.close()
            runCurrent()

            assertTrue(r.store.orders.value.isEmpty())
            assertEquals(0, r.store.pendingCount.value)
            assertFalse(r.store.loaded.value)
        }

    @Test
    fun pendingCountFollowsTheVueGetter() =
        runTest {
            val r =
                rig(
                    listOf(
                        order(1, hubrise = "new", fiscal = "pending"),
                        order(2, hubrise = "received", fiscal = "pending"),
                        order(3, hubrise = "new", fiscal = null),
                        order(4, hubrise = "accepted", fiscal = "refund_pending"),
                        order(5, hubrise = "cancelled", fiscal = "cancel_pending"),
                        order(6, hubrise = "accepted", fiscal = "signed_p"),
                        order(7, hubrise = "new", fiscal = "signed_n"),
                        order(8, hubrise = "in_preparation", fiscal = "signed_n"),
                        order(9, hubrise = "awaiting_collection", fiscal = "signed_n"),
                        order(10, hubrise = null, fiscal = "pending"),
                    ),
                )

            assertEquals(5, r.store.pendingCount.value)
            assertEquals(3, r.store.inProgressCount.value)
            assertEquals(1, r.store.readyCount.value)
        }

    @Test
    fun pendingCountFollowsTheNextFetch() =
        runTest {
            val r = rig(listOf(order(1)))
            assertEquals(1, r.store.pendingCount.value)
            coEvery { r.f.engine.hubriseListWebshopOrders() } returns listOf(order(1), order(2), order(3, hubrise = "accepted"))

            r.feed.send(PosEvent.WebshopOrderReceived(mockk(relaxed = true)))
            runCurrent()

            assertEquals(2, r.store.pendingCount.value)
        }

    @Test
    fun acceptSendsTheDefaultThirtyMinutesAndFetchesAgain() =
        runTest {
            val o = order(7, source = "deliveroo", service = "collection")
            val r = rig(listOf(o))
            val request = slot<AcceptRequest>()
            coEvery { r.f.engine.webshopAcceptOrder(capture(request)) } returns AcceptResult("signed_p", 1L, null, true)

            assertTrue(r.store.acceptOrder(o))

            assertEquals(7L, request.captured.orderId)
            assertEquals("2026-10-09T10:30:00Z", request.captured.confirmedTime)
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
            assertTrue(r.toasts.toasts.value.isEmpty())
        }

    @Test
    fun acceptGivesTheOwnWebshopDeliveryFortyMinutes() =
        runTest {
            val o = order(8, source = "Webshop", service = "delivery")
            val r = rig(listOf(o))
            val request = slot<AcceptRequest>()
            coEvery { r.f.engine.webshopAcceptOrder(capture(request)) } returns AcceptResult("signed_n", 1L, null, true)

            r.store.acceptOrder(o)

            assertEquals("2026-10-09T10:40:00Z", request.captured.confirmedTime)
        }

    @Test
    fun anAggregatorDeliveryKeepsThirtyMinutes() =
        runTest {
            val o = order(9, source = "ubereats", service = "delivery")
            val r = rig(listOf(o))
            val request = slot<AcceptRequest>()
            coEvery { r.f.engine.webshopAcceptOrder(capture(request)) } returns AcceptResult("signed_n", 1L, null, true)

            r.store.acceptOrder(o)

            assertEquals("2026-10-09T10:30:00Z", request.captured.confirmedTime)
        }

    @Test
    fun acceptKeepsTheTimeHubriseAlreadySet() =
        runTest {
            val o = order(10, confirmed = "2026-10-09T12:15:00Z")
            val r = rig(listOf(o))
            val request = slot<AcceptRequest>()
            coEvery { r.f.engine.webshopAcceptOrder(capture(request)) } returns AcceptResult("signed_n", 1L, null, true)

            r.store.acceptOrder(o)

            assertEquals("2026-10-09T12:15:00Z", request.captured.confirmedTime)
        }

    @Test
    fun aKitchenTicketThatDidNotPrintGivesAWarning() =
        runTest {
            val o = order(11)
            val r = rig(listOf(o))
            coEvery { r.f.engine.webshopAcceptOrder(any()) } returns AcceptResult("signed_n", 1L, null, false)

            assertTrue(r.store.acceptOrder(o))

            assertEquals(ToastVariant.Warning, r.lastToast().variant)
            assertEquals("Printer not configured", r.lastToast().message)
        }

    @Test
    fun rejectCallsTheEngineAndFetchesAgain() =
        runTest {
            val o = order(12)
            val r = rig(listOf(o))
            coEvery { r.f.engine.webshopRejectOrder(12L) } returns Unit

            assertTrue(r.store.rejectOrder(o))

            coVerify(exactly = 1) { r.f.engine.webshopRejectOrder(12L) }
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun theThreeStatusStepsSendTheVueStatuses() =
        runTest {
            val r = rig(listOf(order(1)))
            val requests = mutableListOf<PatchStatusRequest>()
            coEvery { r.f.engine.hubrisePatchOrderStatus(capture(requests)) } returns Unit

            assertTrue(r.store.startPreparation("h1"))
            assertTrue(r.store.markReady("h1"))
            assertTrue(r.store.markOutForDelivery("h1"))

            assertEquals(
                listOf("in_preparation", "awaiting_collection", "in_delivery"),
                requests.map { it.newStatus },
            )
            assertEquals(listOf("h1", "h1", "h1"), requests.map { it.hubriseOrderId })
            coVerify(exactly = 4) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun delayStacksOnTheCurrentTimeAndShowsAToast() =
        runTest {
            val r = rig(listOf(order(1, confirmed = "2026-10-09T10:20:00Z")))
            val request = slot<SetConfirmedTimeRequest>()
            coEvery { r.f.engine.hubriseSetConfirmedTime(capture(request)) } returns Unit

            assertTrue(r.store.delayByMinutes("h1", 15))

            assertEquals("h1", request.captured.hubriseOrderId)
            assertEquals("2026-10-09T10:35:00Z", request.captured.confirmedTime)
            assertEquals(ToastVariant.Success, r.lastToast().variant)
            assertEquals("Delay applied +15 min", r.lastToast().message)
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun delayStartsFromNowWhenTheTimeIsPastOrUnknown() =
        runTest {
            val r =
                rig(
                    listOf(
                        order(1, confirmed = "2026-10-09T09:00:00Z"),
                        order(2, confirmed = null),
                        order(3, confirmed = "soon"),
                    ),
                )
            val requests = mutableListOf<SetConfirmedTimeRequest>()
            coEvery { r.f.engine.hubriseSetConfirmedTime(capture(requests)) } returns Unit

            r.store.delayByMinutes("h1", 30)
            r.store.delayByMinutes("h2", 15)
            r.store.delayByMinutes("h3", 45)
            r.store.delayByMinutes("unknown", 15)

            assertEquals(
                listOf("2026-10-09T10:30:00Z", "2026-10-09T10:15:00Z", "2026-10-09T10:45:00Z", "2026-10-09T10:15:00Z"),
                requests.map { it.confirmedTime },
            )
        }

    @Test
    fun cashIsCollectedWithoutAnAmount() =
        runTest {
            val o = order(20, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o))
            val request = slot<HandoverRequest>()
            coEvery { r.f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            assertTrue(r.store.collectOrder(o, PaymentMethod.CASH))

            assertEquals(20L, request.captured.orderId)
            assertEquals(1, request.captured.payments.size)
            assertEquals(PaymentMethod.CASH, request.captured.payments[0].method)
            assertNull(request.captured.payments[0].amountCents)
            assertNull(request.captured.delivery)
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun aCardPaymentCarriesTheTotalOfTheOrder() =
        runTest {
            val o = order(21, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o))
            val request = slot<HandoverRequest>()
            coEvery { r.f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            assertTrue(r.store.collectOrder(o, PaymentMethod.CARD))

            assertEquals(PaymentMethod.CARD, request.captured.payments[0].method)
            assertEquals(1_850L, request.captured.payments[0].amountCents)
        }

    // ── Card on the linked terminal ───────────────────────────────

    @Test
    fun aCardOnTheTerminalIsChargedFirstThenHandedOverWithItsPaymentId() =
        runTest {
            val o = order(60, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o), terminal = true)
            coEvery { r.f.engine.listEftPayments(60L) } returns emptyList()
            coEvery { r.f.engine.eftPay(60L, 1_850L) } returns eftPayment(5, EFT_APPROVED, 1_850, orderId = 60)
            val request = slot<HandoverRequest>()
            coEvery { r.f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            assertTrue(r.store.collectOrder(o, PaymentMethod.CARD))

            coVerify(exactly = 1) { r.f.engine.eftPay(60L, 1_850L) }
            assertEquals(listOf(PaymentPart(PaymentMethod.CARD, 1_850L, 5L)), request.captured.payments)
            assertTrue(r.eft.payments.value.isEmpty())
            assertFalse(r.eft.overlay.value.visible)
        }

    @Test
    fun aDeclinedCardSignsNothingAndTheOperatorMayTryAgain() =
        runTest {
            val o = order(61, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o), terminal = true)
            coEvery { r.f.engine.listEftPayments(61L) } returns emptyList()
            coEvery { r.f.engine.eftPay(61L, 1_850L) } returns eftPayment(6, EFT_DECLINED, 1_850, orderId = 61)

            val result = async { r.store.collectOrder(o, PaymentMethod.CARD) }
            runCurrent()
            assertEquals(EftPhase.RESULT, r.eft.overlay.value.phase)
            r.eft.dismiss()
            runCurrent()

            assertFalse(result.await())
            coVerify(exactly = 0) { r.f.engine.webshopHandoverOrder(any()) }
            assertTrue(r.eft.payments.value.isEmpty())
        }

    @Test
    fun anApprovedPaymentLeftByACrashIsUsedWithoutChargingAgain() =
        runTest {
            val o = order(62, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o), terminal = true)
            coEvery { r.f.engine.listEftPayments(62L) } returns listOf(eftPayment(7, EFT_APPROVED, 1_850, orderId = 62))
            val request = slot<HandoverRequest>()
            coEvery { r.f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            assertTrue(r.store.collectOrder(o, PaymentMethod.CARD))

            coVerify(exactly = 0) { r.f.engine.eftPay(any(), any()) }
            assertEquals(listOf(PaymentPart(PaymentMethod.CARD, 1_850L, 7L)), request.captured.payments)
        }

    @Test
    fun onlyWhatIsLeftIsChargedAfterAPartialApprovedPayment() =
        runTest {
            val o = order(63, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o), terminal = true)
            val first = eftPayment(8, EFT_APPROVED, 1_000, orderId = 63)
            val second = eftPayment(9, EFT_APPROVED, 850, orderId = 63)
            coEvery { r.f.engine.listEftPayments(63L) } returns listOf(first)
            coEvery { r.f.engine.eftPay(63L, 850L) } returns second
            val request = slot<HandoverRequest>()
            coEvery { r.f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            assertTrue(r.store.collectOrder(o, PaymentMethod.CARD))

            coVerify(exactly = 1) { r.f.engine.eftPay(63L, 850L) }
            assertEquals(
                listOf(PaymentPart(PaymentMethod.CARD, 1_000L, 8L), PaymentPart(PaymentMethod.CARD, 850L, 9L)),
                request.captured.payments,
            )
        }

    @Test
    fun aPaymentWaitingForItsAnswerIsCheckedBeforeAnythingElse() =
        runTest {
            val o = order(64, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o), terminal = true)
            coEvery { r.f.engine.listEftPayments(64L) } returns listOf(eftPayment(10, EFT_UNKNOWN, 1_850, orderId = 64))
            coEvery { r.f.engine.eftCheck(10L) } returns eftPayment(10, EFT_APPROVED, 1_850, orderId = 64)
            val request = slot<HandoverRequest>()
            coEvery { r.f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            assertTrue(r.store.collectOrder(o, PaymentMethod.CARD))

            coVerify(exactly = 1) { r.f.engine.eftCheck(10L) }
            coVerify(exactly = 0) { r.f.engine.eftPay(any(), any()) }
            assertEquals(listOf(PaymentPart(PaymentMethod.CARD, 1_850L, 10L)), request.captured.payments)
        }

    @Test
    fun aStillUnknownPaymentStopsTheCollectionAndStaysListedUntilChecked() =
        runTest {
            val o = order(65, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o), terminal = true)
            val unknown = eftPayment(11, EFT_UNKNOWN, 1_850, orderId = 65)
            coEvery { r.f.engine.listEftPayments(65L) } returns listOf(unknown)
            coEvery { r.f.engine.eftCheck(11L) } returns unknown

            val result = async { r.store.collectOrder(o, PaymentMethod.CARD) }
            runCurrent()
            assertTrue(r.eft.overlay.value.unknown)
            r.eft.dismiss()
            runCurrent()

            assertFalse(result.await())
            coVerify(exactly = 0) { r.f.engine.webshopHandoverOrder(any()) }
        }

    @Test
    fun cashIsNeverSentToTheTerminal() =
        runTest {
            val o = order(66, fiscal = "signed_p", hubrise = "awaiting_collection", totalCents = 1_850)
            val r = rig(listOf(o), terminal = true)
            coEvery { r.f.engine.webshopHandoverOrder(any()) } returns sale()

            assertTrue(r.store.collectOrder(o, PaymentMethod.CASH))

            coVerify(exactly = 0) { r.f.engine.eftPay(any(), any()) }
            coVerify(exactly = 0) { r.f.engine.listEftPayments(any()) }
        }

    @Test
    fun completeCallsTheEngineAndFetchesAgain() =
        runTest {
            val o = order(22, fiscal = "signed_n", hubrise = "awaiting_collection")
            val r = rig(listOf(o))
            coEvery { r.f.engine.webshopCompleteOrder(22L) } returns Unit

            assertTrue(r.store.completeOrder(o))

            coVerify(exactly = 1) { r.f.engine.webshopCompleteOrder(22L) }
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun refundSendsTheManagerInsz() =
        runTest {
            val o = order(23, fiscal = "refund_pending")
            val r = rig(listOf(o))
            val request = slot<RefundRequest>()
            coEvery { r.f.engine.webshopRefundOrder(capture(request)) } returns sale()

            assertTrue(r.store.refundOrder(o, "85010100123"))

            assertEquals(23L, request.captured.orderId)
            assertEquals("85010100123", request.captured.employeeInsz)
            assertNull(request.captured.delivery)
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun closeCancelledCallsTheEngineAndFetchesAgain() =
        runTest {
            val o = order(24, fiscal = "cancel_pending")
            val r = rig(listOf(o))
            coEvery { r.f.engine.webshopCloseCancelledOrder(24L) } returns sale()

            assertTrue(r.store.closeCancelledOrder(o))

            coVerify(exactly = 1) { r.f.engine.webshopCloseCancelledOrder(24L) }
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun aRefusedActionShowsTheErrorAnswersFalseAndStillFetchesAgain() =
        runTest {
            val o = order(30)
            val r = rig(listOf(o))
            coEvery { r.f.engine.webshopRejectOrder(30L) } throws PosException.Coded("order_no_lines", emptyMap())

            assertFalse(r.store.rejectOrder(o))

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            val expected = I18nTestSupport.errorTexts().describe(PosException.Coded("order_no_lines", emptyMap()), AppLanguage.EN)
            assertEquals(expected, r.lastToast().message)
            assertEquals(r.lastToast().message, r.store.lastError.value)
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
            assertTrue(r.store.busyOrderIds.value.isEmpty())
        }

    @Test
    fun aSaleThatCameBackWithoutSuccessIsARefusal() =
        runTest {
            val o = order(31, fiscal = "signed_p")
            val r = rig(listOf(o))
            coEvery { r.f.engine.webshopHandoverOrder(any()) } returns sale(success = false, error = "order_no_lines")

            assertFalse(r.store.collectOrder(o, PaymentMethod.CASH))

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertEquals(I18nTestSupport.errorTexts().describe("order_no_lines", AppLanguage.EN), r.lastToast().message)
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun aSaleWithoutAnErrorTextReportsAnInternalError() =
        runTest {
            val o = order(32, fiscal = "refund_pending")
            val r = rig(listOf(o))
            coEvery { r.f.engine.webshopRefundOrder(any()) } returns sale(success = false, error = null)

            assertFalse(r.store.refundOrder(o, "1"))

            assertEquals(I18nTestSupport.errorTexts().describe("internal", AppLanguage.EN), r.lastToast().message)
        }

    @Test
    fun anOrderWithAnActionRunningRefusesASecondOne() =
        runTest {
            val o = order(40)
            val r = rig(listOf(o))
            val gate = CompletableDeferred<Unit>()
            coEvery { r.f.engine.webshopRejectOrder(40L) } coAnswers { gate.await() }

            val first = async { r.store.rejectOrder(o) }
            runCurrent()
            assertEquals(setOf(40L), r.store.busyOrderIds.value)
            assertFalse(r.store.completeOrder(o))
            coVerify(exactly = 0) { r.f.engine.webshopCompleteOrder(any()) }

            gate.complete(Unit)
            runCurrent()
            assertTrue(first.await())
            assertTrue(r.store.busyOrderIds.value.isEmpty())
        }

    @Test
    fun printKitchenAndCustomerCallTheirCommandsAndConfirm() =
        runTest {
            val r = rig(listOf(order(50)))
            coEvery { r.f.engine.printWebshopKitchen(50L) } returns Unit
            coEvery { r.f.engine.printWebshopCustomer(50L) } returns Unit

            assertTrue(r.store.printKitchen(50L))
            assertEquals(ToastVariant.Success, r.lastToast().variant)
            assertEquals("Sent to printer", r.lastToast().message)
            assertTrue(r.store.printCustomer(50L))

            coVerify(exactly = 1) { r.f.engine.printWebshopKitchen(50L) }
            coVerify(exactly = 1) { r.f.engine.printWebshopCustomer(50L) }
            coVerify(exactly = 1) { r.f.engine.hubriseListWebshopOrders() }
        }

    @Test
    fun aMissingPrinterIsAWarningAnyOtherPrintFailureAnError() =
        runTest {
            val r = rig(listOf(order(51)))
            coEvery { r.f.engine.printWebshopKitchen(51L) } throws PosException.Coded("printer_not_configured", emptyMap())
            coEvery { r.f.engine.printWebshopCustomer(51L) } throws PosException.Coded("order_no_lines", emptyMap())

            assertFalse(r.store.printKitchen(51L))
            assertEquals(ToastVariant.Warning, r.lastToast().variant)
            assertEquals("Printer not configured", r.lastToast().message)

            assertFalse(r.store.printCustomer(51L))
            assertEquals(ToastVariant.Error, r.lastToast().variant)
        }

    @Test
    fun resyncFetchesTheCatalogThenTheOrders() =
        runTest {
            val r = rig(listOf(order(60)))
            coEvery { r.f.engine.hubriseSyncCatalog() } returns ApplyOutcome.Applied("hash", 3L, 0L)

            assertTrue(r.store.resync())

            coVerify(exactly = 1) { r.f.engine.hubriseSyncCatalog() }
            coVerify(exactly = 2) { r.f.engine.hubriseListWebshopOrders() }
            assertTrue(r.toasts.toasts.value.isEmpty())
        }

    @Test
    fun aRefusedCatalogIsReported() =
        runTest {
            val r = rig(listOf(order(61)))
            coEvery { r.f.engine.hubriseSyncCatalog() } returns ApplyOutcome.Rejected(emptyList())

            assertTrue(r.store.resync())

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertEquals("Catalog refused: the previous menu stays.", r.lastToast().message)
        }
}
