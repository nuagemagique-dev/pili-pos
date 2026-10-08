package dev.nuagemagique.pilipos.core

import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
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
import uniffi.pos_ffi.cashDueCents
import uniffi.pos_ffi.settlePreview
import java.io.File

/**
 * Checkout, refund, X and Z, events, errors, cancellation and concurrency of the real engine
 * (host library) against the two mock processes. Every test has its own data directory, seeded
 * with src-tauri/seed_spf_test.sql, and its own pair of mocks. Amounts always come from the engine
 * or from the FFI helpers: nothing here computes money.
 */
class EndToEndTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private lateinit var fdm: MockProcess
    private lateinit var printer: MockProcess

    @Before
    fun startMocks() {
        fdm = Mocks.startFdm()
        try {
            printer = Mocks.startPrinter()
        } catch (t: Throwable) {
            fdm.stop()
            throw t
        }
    }

    @After
    fun stopMocks() {
        if (::printer.isInitialized) printer.stop()
        if (::fdm.isInitialized) fdm.stop()
    }

    /** Runs [body] with a seeded, opened engine; the engine is always shut down and released. */
    private fun e2e(body: suspend EndToEnd.() -> Unit) {
        val base = tmp.newFolder()
        val env =
            runBlocking(Dispatchers.IO) {
                EndToEnd.open(File(base, "data"), File(base, "bundle"), fdm, printer)
            }
        try {
            runBlocking(Dispatchers.IO) { withTimeout(180_000) { env.body() } }
        } finally {
            env.close()
        }
    }

    // ───────────────────────────── helpers ─────────────────────────────

    private suspend fun EndToEnd.loginManager() {
        val user = engine.login(MANAGER_INSZ, MANAGER_PIN)
        assertEquals(MANAGER_INSZ, user.insz)
    }

    /** A priced, active item of the seeded menu. */
    private suspend fun EndToEnd.anItem(): MenuItem =
        engine.getMenuItems(null, "fr").first { it.active && !it.openPrice && it.priceCents > 0 }

    /** A takeaway order with [quantity] of the first item. */
    private suspend fun EndToEnd.takeawayWithLine(quantity: Int = 2): Order {
        val order = engine.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, MANAGER_INSZ))
        engine.addOrderLine(AddOrderLineInput(order.id, anItem().id, quantity, null, null))
        return engine.getOrder(order.id)
    }

    private val cash = listOf(PaymentPart(PaymentMethod.CASH, null))

    private suspend fun EndToEnd.pay(order: Order): CheckoutResult = engine.checkout(order.id, cash, null)

    private suspend fun EndToEnd.salesOf(orderId: Long) =
        engine.listFiscalEvents(null).filter { it.orderId == orderId && it.eventLabel == "N" }

    private fun assertSigned(result: CheckoutResult) {
        assertTrue("checkout failed: ${result.error}", result.success)
        val ticket = result.ticket ?: throw AssertionError("a ticket")
        val ref = ticket.fdmRef ?: throw AssertionError("an FDM reference")
        assertTrue("fdm id", ref.fdmId.isNotBlank())
        assertTrue("event counter", ref.eventCounter >= 1)
        assertNotNull("a fiscal event id", result.fiscalEventId)
    }

    // ───────────────────────────── sale, refund, reports ─────────────────────────────

    @Test
    fun saleIsSignedAndPrinted() =
        e2e {
            loginManager()
            val order = takeawayWithLine()
            assertTrue(order.totalCents > 0)

            // The settlement helpers answer what the checkout will do (no money math here).
            val preview = settlePreview(order.totalCents, cash)
            assertTrue(preview.hasCash)
            assertEquals(preview.cashDueCents, cashDueCents(order.totalCents, cash))

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
    fun refundOfTheSaleIsSignedAndPrinted() =
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
    fun xThenZReportsAndNeedsZ() =
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

    // ───────────────────────────── events ─────────────────────────────

    @Test
    fun fdmStatusAfterLoginAndSessionLockedOnLock() =
        e2e {
            loginManager()
            val status = awaitEvent<PosEvent.FdmStatus>(15_000)
            assertNotNull(status.health)

            engine.lockSession()
            awaitEvent<PosEvent.SessionLocked>(15_000)
        }

    // ───────────────────────────── errors ─────────────────────────────

    /**
     * The checkout reports its refusals inside the result (as the Tauri command does): an order
     * without lines is a CheckoutResult with success false and the code order_no_lines, never an
     * exception and never a sale.
     */
    @Test
    fun checkoutOfAnOrderWithoutLinesIsRefusedInTheResult() =
        e2e {
            loginManager()
            val order = engine.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, MANAGER_INSZ))
            val result = engine.checkout(order.id, cash, null)
            assertFalse(result.success)
            assertEquals("order_no_lines", result.error)
            assertEquals(null, result.ticket)
            assertTrue("no sale was signed", engine.listFiscalEvents(null).none { it.orderId == order.id })
            assertEquals(OrderStatus.OPEN, engine.getOrder(order.id).status)
        }

    /** The pre-bill of a table without lines is refused with the code, which maps to Coded. */
    @Test
    fun preBillOfAnEmptyTableOrderThrowsCodedOrderNoLines() =
        e2e {
            loginManager()
            val order = engine.createOrder(CreateOrderInput(OrderType.DINE_IN, 1, MANAGER_INSZ))
            val error = runCatching { engine.signPreBill(order.id) }.exceptionOrNull()
            assertTrue("expected PosException.Coded, got $error", error is PosException.Coded)
            error as PosException.Coded
            assertEquals("order_no_lines", error.code)
            assertTrue(error.params.isEmpty())
            assertTrue("no event was signed", engine.listFiscalEvents(null).none { it.orderId == order.id })
        }

    @Test
    fun aCallAfterShutdownIsEngineClosed() =
        e2e {
            engine.shutdown()
            val error = runCatching { engine.getSettings() }.exceptionOrNull()
            assertTrue("expected EngineClosed, got $error", error is PosException.EngineClosed)
            val again = runCatching { engine.userActivity() }.exceptionOrNull()
            assertTrue("expected EngineClosed, got $again", again is PosException.EngineClosed)
        }

    // ───────────────────────────── cancellation ─────────────────────────────

    /**
     * A coroutine that is cancelled while its checkout runs never leaves a half state: the Rust
     * call is detached and finishes, so the sale and the order agree.
     */
    private suspend fun EndToEnd.cancelledCheckoutKeepsAgreement(afterMs: Long) {
        val order = takeawayWithLine()
        val job =
            coroutineScope {
                val j = launch(Dispatchers.IO, start = CoroutineStart.UNDISPATCHED) { pay(order) }
                if (afterMs > 0) delay(afterMs)
                j.cancel()
                j
            }
        job.join()

        // Let a detached checkout finish: either the order closes, or nothing was started.
        val deadline = System.nanoTime() + 5_000_000_000L
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
    }

    @Test
    fun cancellingACheckoutRightAfterLaunchLeavesNoHalfState() =
        e2e {
            loginManager()
            cancelledCheckoutKeepsAgreement(afterMs = 0)
        }

    @Test
    fun cancellingACheckoutWhileItSignsLeavesNoHalfState() =
        e2e {
            loginManager()
            cancelledCheckoutKeepsAgreement(afterMs = 30)
        }

    // ───────────────────────────── concurrency ─────────────────────────────

    @Test
    fun parallelCallsComplete() =
        e2e {
            loginManager()
            coroutineScope {
                val activity = (1..8).map { async(Dispatchers.IO) { engine.userActivity() } }
                val settings = (1..4).map { async(Dispatchers.IO) { engine.getSettings() } }
                activity.awaitAll()
                val all = settings.awaitAll()
                assertEquals(4, all.size)
                assertTrue(all.all { it.posId == all.first().posId && it.posId.isNotBlank() })
            }
        }
}
