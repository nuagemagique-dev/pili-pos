package dev.nuagemagique.pilipos.core.device

import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RefundItem

/**
 * Checkout, refund, X and Z, events, cancellation and concurrency of the real engine on the device, against
 * the host FDM and printer mocks reached through adb reverse. Every test has its own seeded data directory
 * under filesDir. Amounts always come from the engine: nothing here computes money.
 */
@RunWith(AndroidJUnit4::class)
class DeviceEndToEndTest {
    @Before
    fun mocksAreUp() {
        requireFdmMock()
        requirePrinterMock()
        // The printer mock is shared by the tests of the run: each test counts only its own jobs.
        clearPrinterJobs()
    }

    /** Runs [body] with a seeded, opened engine; the engine is always shut down and released. */
    private fun e2e(body: suspend DeviceEngine.() -> Unit) {
        val env = runBlocking(ioDispatcher) { DeviceEngine.open() }
        try {
            runBlocking(ioDispatcher) { withTimeout(TEST_TIMEOUT_MS) { env.body() } }
        } finally {
            env.close()
        }
    }

    // ───────────────────────────── helpers ─────────────────────────────

    private suspend fun DeviceEngine.loginManager() {
        val user = engine.login(MANAGER_INSZ, MANAGER_PIN)
        assertEquals(MANAGER_INSZ, user.insz)
    }

    /** A priced, active item of the seeded menu. */
    private suspend fun DeviceEngine.anItem(): MenuItem =
        engine.getMenuItems(null, "fr").first { it.active && !it.openPrice && it.priceCents > 0 }

    /** A takeaway order with [quantity] of the first item. */
    private suspend fun DeviceEngine.takeawayWithLine(quantity: Int = 2): Order {
        val order = engine.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, MANAGER_INSZ))
        engine.addOrderLine(AddOrderLineInput(order.id, anItem().id, quantity, null, null))
        return engine.getOrder(order.id)
    }

    private val cash = listOf(PaymentPart(PaymentMethod.CASH, null))

    private suspend fun DeviceEngine.pay(order: Order): CheckoutResult = engine.checkout(order.id, cash, null)

    private suspend fun DeviceEngine.salesOf(orderId: Long) =
        engine.listFiscalEvents(null).filter { it.orderId == orderId && it.eventLabel == "N" }

    private fun assertSigned(result: CheckoutResult) {
        assertTrue("checkout failed: ${result.error}", result.success)
        val ticket = result.ticket ?: throw AssertionError("a ticket")
        val ref = ticket.fdmRef ?: throw AssertionError("an FDM reference")
        assertTrue("fdm id", ref.fdmId.isNotBlank())
        assertTrue("event counter", ref.eventCounter >= 1)
        assertNotNull("a fiscal event id", result.fiscalEventId)
    }

    // ───────────────────────────── tests ─────────────────────────────

    @Test
    fun theSeededManagerLogsIn() =
        e2e {
            val user = engine.login(MANAGER_INSZ, MANAGER_PIN)
            assertEquals(MANAGER_INSZ, user.insz)
            val wrong = runCatching { engine.login(MANAGER_INSZ, "0000") }
            assertTrue("a wrong PIN must not log in", wrong.isFailure || wrong.getOrNull()?.insz != MANAGER_INSZ)
        }

    @Test
    fun aCashSaleIsSignedAndPrinted() =
        e2e {
            loginManager()
            val order = takeawayWithLine()
            assertTrue(order.totalCents > 0)

            val result = pay(order)
            assertSigned(result)

            val sales = salesOf(order.id)
            assertEquals("one N sale for the order", 1, sales.size)
            assertEquals(result.fiscalEventId, sales.single().id)
            assertEquals(OrderStatus.CLOSED, engine.getOrder(order.id).status)

            val jobs = awaitPrinterJobs(1)
            assertTrue("the customer ticket was printed: ${jobs.first().take(200)}", jobs.any { it.isNotBlank() })
        }

    @Test
    fun theRefundOfTheSaleIsSignedAndPrinted() =
        e2e {
            loginManager()
            val order = takeawayWithLine(2)
            val sale = pay(order)
            assertSigned(sale)
            val saleEventId = sale.fiscalEventId!!
            awaitPrinterJobs(1)

            val refundable = engine.listRefundableEvents(null).single { it.fiscalEventId == saleEventId }
            assertEquals(order.id, refundable.orderId)
            assertEquals(order.totalCents, refundable.totalCents)

            val line = engine.listOrderLines(order.id).single()
            val refund =
                engine.refundCheckout(
                    saleEventId,
                    listOf(RefundItem(line.menuItemId, line.quantity, line.id)),
                    MANAGER_INSZ,
                    null,
                )
            assertTrue("refund failed: ${refund.error}", refund.success)
            val refundTicket = refund.ticket ?: throw AssertionError("a refund ticket")
            assertNotNull("an FDM reference on the refund", refundTicket.fdmRef)
            assertNotNull("a fiscal event id on the refund", refund.fiscalEventId)

            val labels = engine.listFiscalEvents(null).map { it.eventLabel }
            assertTrue("N and a refund event, saw $labels", labels.contains("N") && labels.size >= 2)
            awaitPrinterJobs(2)
        }

    @Test
    fun xThenZReportAndNoZNeededAfterwards() =
        e2e {
            loginManager()
            assertSigned(pay(takeawayWithLine()))

            val x = engine.generateXReport(MANAGER_INSZ)
            assertTrue("X report failed: ${x.error}", x.success)
            val z = engine.generateZReport(MANAGER_INSZ)
            assertTrue("Z report failed: ${z.error}", z.success)

            val reports = engine.listZReports()
            assertTrue("the Z is listed", reports.isNotEmpty())
            z.reportNo?.let { no -> assertTrue(reports.any { it.sequenceNumber == no }) }
            assertFalse("no Z needed after the Z", engine.needsZReport())
        }

    @Test
    fun fdmStatusAfterLoginAndSessionLockedAfterTheLock() =
        e2e {
            loginManager()
            val status = awaitEvent<PosEvent.FdmStatus>(15_000)
            assertNotNull(status.health)

            engine.lockSession()
            awaitEvent<PosEvent.SessionLocked>(15_000)
        }

    @Test
    fun aCheckoutCancelledRightAfterLaunchEndsClosedWithOneSale() =
        e2e {
            loginManager()
            cancelledCheckoutKeepsAgreement(afterMs = 0)
        }

    @Test
    fun aCheckoutCancelledWhileItSignsEndsClosedWithOneSale() =
        e2e {
            loginManager()
            cancelledCheckoutKeepsAgreement(afterMs = 30)
        }

    @Test
    fun parallelCallsComplete() =
        e2e {
            loginManager()
            coroutineScope {
                val activity = (1..8).map { async(ioDispatcher) { engine.userActivity() } }
                val settings = (1..4).map { async(ioDispatcher) { engine.getSettings() } }
                activity.awaitAll()
                val all = settings.awaitAll()
                assertEquals(4, all.size)
                assertTrue(all.all { it.posId == all.first().posId && it.posId.isNotBlank() })
            }
        }

    /**
     * A coroutine that is cancelled while its checkout runs never leaves a half state: the Rust call is
     * detached and finishes, so the sale and the order agree. Once it has settled the order is CLOSED with
     * exactly one sale (a retry closes it when nothing had started).
     */
    private suspend fun DeviceEngine.cancelledCheckoutKeepsAgreement(afterMs: Long) {
        val order = takeawayWithLine()
        val job =
            coroutineScope {
                val j = launch(ioDispatcher, start = CoroutineStart.UNDISPATCHED) { pay(order) }
                if (afterMs > 0) delay(afterMs)
                j.cancel()
                j
            }
        job.join()

        // Let a detached checkout finish: either the order closes, or nothing was started.
        val deadline = System.nanoTime() + SETTLE_NS
        while (engine.getOrder(order.id).status == OrderStatus.OPEN && System.nanoTime() < deadline) delay(50)

        val status = engine.getOrder(order.id).status
        val sales = salesOf(order.id)
        println("cancellation after $afterMs ms: order $status, ${sales.size} sale(s)")
        when (status) {
            OrderStatus.CLOSED -> assertEquals("closed order has exactly one sale", 1, sales.size)
            OrderStatus.OPEN -> assertEquals("open order has no sale", 0, sales.size)
            OrderStatus.VOIDED -> error("a cancelled checkout must not void the order")
        }

        // A following checkout of the same order succeeds, or says it is already closed.
        val retry = runCatching { pay(order) }
        val retryError = retry.exceptionOrNull()
        val retryResult = retry.getOrNull()
        val alreadyClosed =
            when {
                retryError is PosException.Coded -> retryError.code == "order_already_closed"
                retryResult != null -> !retryResult.success && retryResult.error.orEmpty().startsWith("order_already_closed")
                else -> false
            }
        assertTrue(
            "retry must succeed or report order_already_closed, got result=${retryResult?.error} error=$retryError",
            (retryResult?.success == true) || alreadyClosed,
        )
        assertEquals(OrderStatus.CLOSED, engine.getOrder(order.id).status)
        assertEquals("never two sales for one order", 1, salesOf(order.id).size)
        // The print of a detached checkout lands before the engine goes away.
        withContext(ioDispatcher) { awaitPrinterJobs(1) }
    }

    private companion object {
        const val TEST_TIMEOUT_MS = 180_000L
        const val SETTLE_NS = 5_000_000_000L
    }
}
