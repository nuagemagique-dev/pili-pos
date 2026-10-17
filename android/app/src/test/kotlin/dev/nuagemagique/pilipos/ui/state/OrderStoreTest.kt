package dev.nuagemagique.pilipos.ui.state

import app.cash.turbine.test
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.VatCode

@Suppress("LongParameterList", "LargeClass", "TooManyFunctions")
class OrderStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private fun order(
        id: Long = 1,
        table: Int? = null,
        type: OrderType = if (table != null) OrderType.DINE_IN else OrderType.TAKEAWAY,
        totalCents: Long = 0,
    ) = Order(
        id = id,
        tableNumber = table,
        orderType = type,
        status = OrderStatus.OPEN,
        userInsz = "insz",
        createdAt = "2026-10-09T10:00:00",
        closedAt = null,
        subtotalCents = totalCents,
        totalCents = totalCents,
    )

    private fun line(
        id: Long,
        menuItemId: Long,
        quantity: Int = 1,
        unsent: Long = 0,
        modifiers: String? = null,
        unitPriceCents: Long = 500,
    ) = CartLine(
        id = id,
        orderId = 1,
        menuItemId = menuItemId,
        quantity = quantity,
        unitPriceCents = unitPriceCents,
        vatCode = VatCode.B,
        name = "item$menuItemId",
        modifiersJson = modifiers,
        createdAt = "2026-10-09T10:00:00",
        departmentId = "d",
        departmentName = "Dept",
        nameFr = null,
        nameZh = null,
        code = null,
        unsentQuantity = unsent,
    )

    private fun item(
        id: Long,
        category: Long = 7,
        openPrice: Boolean = false,
    ) = MenuItem(
        id = id,
        categoryId = category,
        name = "item$id",
        nameFr = null,
        nameZh = null,
        code = null,
        priceCents = 500,
        vatCode = VatCode.B,
        vatCodeTakeaway = null,
        openPrice = openPrice,
        kitchenPrinter = true,
        sortOrder = 1,
        active = true,
        soldOut = false,
        soldOutUntil = null,
    )

    private fun orderLine(menuItemId: Long) =
        OrderLine(
            id = 99,
            orderId = 1,
            menuItemId = menuItemId,
            quantity = 1,
            unitPriceCents = 500,
            vatCode = VatCode.B,
            name = "item$menuItemId",
            modifiersJson = "[]",
            createdAt = "2026-10-09T10:00:00",
            departmentId = "d",
            departmentName = "Dept",
        )

    private fun result(
        success: Boolean,
        error: String? = null,
        slip: SlipOutcome? = null,
    ) = CheckoutResult(
        success = success,
        ticket = null,
        error = error,
        fdmConnected = true,
        fiscalEventId = null,
        delivery = null,
        kitchenSlip = slip,
    )

    private class Rig(
        val f: EngineFixture,
        val store: OrderStore,
        val events: Channel<PosEvent?>,
    )

    /** An open engine whose event stream is fed by [Rig.events], and a store wired over it. */
    private suspend fun TestScope.rig(): Rig {
        val f = EngineFixture(this, tmp.root)
        val events = Channel<PosEvent?>(Channel.UNLIMITED)
        coEvery { f.stream.next() } coAnswers { events.receive() }
        val store = OrderStore(f.gateway(this), f.events, backgroundScope)
        f.holder.open()
        // Subscribed stores and the pump are running; let both reach their first suspension.
        runCurrent()
        return Rig(f, store, events)
    }

    private fun Rig.stubReads(
        order: Order,
        cart: List<CartLine>,
    ) {
        coEvery { f.engine.getOrder(order.id) } returns order
        coEvery { f.engine.listCartLines(order.id) } returns cart
    }

    @Test
    fun takeawayStartCreatesAnEmptyOrderAndSetsItActive() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.createOrder(any()) } returns order(id = 5)
            coEvery { r.f.engine.setActiveOrder(5L) } returns Unit

            r.store.startOrder(null, "insz")

            val input = io.mockk.slot<CreateOrderInput>()
            coVerify { r.f.engine.createOrder(capture(input)) }
            assertEquals(OrderType.TAKEAWAY, input.captured.orderType)
            assertNull(input.captured.tableNumber)
            assertEquals("insz", input.captured.userInsz)
            assertEquals(5L, r.store.current.value?.id)
            assertTrue(r.store.lines.value.isEmpty())
            coVerify(exactly = 0) { r.f.engine.listCartLines(any()) }
            coVerify { r.f.engine.setActiveOrder(5L) }
        }

    @Test
    fun dineInStartOnABusyTableLoadsItsLines() =
        runTest {
            val r = rig()
            val busy = listOf(line(1, 10, quantity = 2, unsent = 2), line(2, 11, unsent = 1))
            coEvery { r.f.engine.createOrder(any()) } returns order(id = 6, table = 4)
            coEvery { r.f.engine.listCartLines(6L) } returns busy
            coEvery { r.f.engine.setActiveOrder(6L) } returns Unit

            r.store.startOrder(4, "insz")

            val input = io.mockk.slot<CreateOrderInput>()
            coVerify { r.f.engine.createOrder(capture(input)) }
            assertEquals(OrderType.DINE_IN, input.captured.orderType)
            assertEquals(4, input.captured.tableNumber)
            assertEquals(busy, r.store.lines.value)
            assertEquals(3, r.store.unsentCount.value)
            coVerify { r.f.engine.setActiveOrder(6L) }
        }

    @Test
    fun loadOrderReadsTheOrderAndItsLinesThenSetsItActive() =
        runTest {
            val r = rig()
            val cart = listOf(line(1, 10, quantity = 3))
            r.stubReads(order(id = 8, table = 2, totalCents = 1500), cart)
            coEvery { r.f.engine.setActiveOrder(8L) } returns Unit

            r.store.loadOrder(8)

            assertEquals(8L, r.store.current.value?.id)
            assertEquals(cart, r.store.lines.value)
            coVerify { r.f.engine.setActiveOrder(8L) }
        }

    @Test
    fun addingAFixedPriceItemTwiceMergesIntoTheExistingLine() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(10)
            r.stubReads(order(), listOf(line(1, 10, unsent = 1)))

            assertTrue(r.store.addItem(item(10)))
            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }

            coEvery { r.f.engine.updateOrderLineQuantity(1L, 3) } returns Unit
            r.stubReads(order(), listOf(line(1, 10, quantity = 3, unsent = 3)))
            assertTrue(r.store.addItem(item(10), quantity = 2))

            coVerify { r.f.engine.updateOrderLineQuantity(1L, 3) }
            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            assertEquals(3, r.store.lines.value.single().quantity)
            assertEquals(3, r.store.unsentCount.value)
            assertEquals(7L, r.store.categoryOf(10))
        }

    @Test
    fun addPassesTheOrderItemAndNoPriceForAFixedPriceItem() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), emptyList())
            val input = io.mockk.slot<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(input)) } returns orderLine(10)
            r.stubReads(order(), listOf(line(1, 10)))

            assertTrue(r.store.addItem(item(10), quantity = 2, unitPriceCents = 999))

            assertEquals(1L, input.captured.orderId)
            assertEquals(10L, input.captured.menuItemId)
            assertEquals(2, input.captured.quantity)
            assertNull(input.captured.modifiersJson)
            assertNull(input.captured.unitPriceCents)
        }

    @Test
    fun anOpenPriceItemGetsItsOwnLineWithTheEnteredPrice() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 20, unitPriceCents = 300)))
            val input = io.mockk.slot<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(input)) } returns orderLine(20)
            r.stubReads(order(), listOf(line(1, 20, unitPriceCents = 300), line(2, 20, unitPriceCents = 450)))

            assertTrue(r.store.addItem(item(20, openPrice = true), unitPriceCents = 450))

            assertEquals(450L, input.captured.unitPriceCents)
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
            assertEquals(2, r.store.lines.value.size)
        }

    @Test
    fun aLineWithModifiersIsNeverMergedInto() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10, modifiers = "[{\"id\":1}]")))
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(10)
            r.stubReads(order(), listOf(line(1, 10, modifiers = "[{\"id\":1}]"), line(2, 10)))

            assertTrue(r.store.addItem(item(10)))

            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun anEmptyModifiersTextCountsAsNoModifiers() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10, modifiers = "")))
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 2) } returns Unit
            r.stubReads(order(), listOf(line(1, 10, quantity = 2, modifiers = "")))

            assertTrue(r.store.addItem(item(10)))

            coVerify { r.f.engine.updateOrderLineQuantity(1L, 2) }
        }

    @Test
    fun addLineWithShowsTheLineAndRemembersTheCategory() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), emptyList())
            r.stubReads(order(), listOf(line(2, 30, modifiers = "[{\"id\":1}]", unsent = 1)))
            var seenOrderId = 0L

            val ok =
                r.store.addLineWith(item(30, category = 9), 1) { orderId ->
                    seenOrderId = orderId
                    orderLine(30)
                }

            assertTrue(ok)
            assertEquals(1L, seenOrderId)
            assertEquals(1, r.store.lines.value.size)
            assertEquals(1, r.store.unsentCount.value)
            assertEquals(9L, r.store.categoryOf(30))
        }

    @Test
    fun addLineWithRefusalReturnsFalseAndIsReported() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())
            val refusal = PosException.Coded("product_unavailable", emptyMap())

            r.store.failures.test {
                assertFalse(r.store.addLineWith(item(30), 1) { throw refusal })
                assertSame(refusal, awaitItem())
            }
        }

    @Test
    fun addingWithoutAnOrderIsReportedNotThrown() =
        runTest {
            val r = rig()
            r.store.failures.test {
                assertFalse(r.store.addItem(item(10)))
                assertTrue(awaitItem() is IllegalStateException)
            }
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun anEngineRefusalOnAddReturnsFalseAndIsEmitted() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())
            val refusal = PosException.Coded("order_not_open", emptyMap())
            coEvery { r.f.engine.addOrderLine(any()) } throws refusal

            r.store.failures.test {
                assertFalse(r.store.addItem(item(10)))
                assertSame(refusal, awaitItem())
            }
            assertEquals(null, r.store.categoryOf(10))
        }

    @Test
    fun updateQuantityBelowOneDoesNothing() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10, quantity = 2)))

            assertFalse(r.store.updateQuantity(1, 0))
            assertFalse(r.store.updateQuantity(1, -3))

            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
            assertEquals(2, r.store.lines.value.single().quantity)
        }

    @Test
    fun updateQuantityCallsTheEngineAndReReads() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10, quantity = 2)))
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 5) } returns Unit
            r.stubReads(order(totalCents = 2500), listOf(line(1, 10, quantity = 5, unsent = 3)))

            assertTrue(r.store.updateQuantity(1, 5))

            assertEquals(5, r.store.lines.value.single().quantity)
            assertEquals(3, r.store.unsentCount.value)
            assertEquals(2500L, r.store.current.value?.totalCents)
        }

    @Test
    fun updateQuantityRefusalIsReported() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10)))
            val refusal = PosException.Coded("order_not_open", emptyMap())
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 2) } throws refusal

            r.store.failures.test {
                assertFalse(r.store.updateQuantity(1, 2))
                assertSame(refusal, awaitItem())
            }
        }

    @Test
    fun removeLineCallsTheEngineAndReReads() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10), line(2, 11)))
            coEvery { r.f.engine.removeOrderLine(1L) } returns Unit
            r.stubReads(order(), listOf(line(2, 11)))

            assertTrue(r.store.removeLine(1))

            assertEquals(listOf(2L), r.store.lines.value.map { it.id })
        }

    @Test
    fun removeLineRefusalReturnsFalse() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10)))
            val refusal = PosException.Coded("order_not_open", emptyMap())
            coEvery { r.f.engine.removeOrderLine(1L) } throws refusal

            r.store.failures.test {
                assertFalse(r.store.removeLine(1))
                assertSame(refusal, awaitItem())
            }
            assertEquals(1, r.store.lines.value.size)
        }

    @Test
    fun anUnavailableEngineIsReportedAsAFailure() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())
            r.events.send(null)
            r.f.holder.close()

            r.store.failures.test {
                assertFalse(r.store.removeLine(1))
                assertTrue(awaitItem() is EngineUnavailableException)
            }
        }

    @Test
    fun loadOpenOrdersFillsTheList() =
        runTest {
            val r = rig()
            val open = listOf(order(1, table = 1), order(2, table = 2))
            coEvery { r.f.engine.listOpenOrders() } returns open

            r.store.loadOpenOrders()

            assertEquals(open, r.store.openOrders.value)
        }

    @Test
    fun leaveATakeawayOrderReReadsTheLines() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10, unsent = 2)))
            coEvery { r.f.engine.leaveOrder(1L, LeaveReason.NAVIGATE) } returns SlipOutcome.NOTHING_TO_PRINT
            coEvery { r.f.engine.listCartLines(1L) } returns listOf(line(1, 10, unsent = 2))

            r.store.slipFailures.test {
                r.store.leave(LeaveReason.NAVIGATE)
                expectNoEvents()
            }

            assertEquals(2, r.store.unsentCount.value)
            assertEquals(1L, r.store.current.value?.id)
        }

    @Test
    fun leaveADineInOrderPicksUpTheNewUnsentQuantities() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, unsent = 2), line(2, 11, unsent = 1)))
            coEvery { r.f.engine.leaveOrder(1L, LeaveReason.SEND_TO_KITCHEN) } returns SlipOutcome.PRINTED
            coEvery { r.f.engine.listCartLines(1L) } returns listOf(line(1, 10, unsent = 0), line(2, 11, unsent = 0))

            r.store.leave(LeaveReason.SEND_TO_KITCHEN)

            assertEquals(0, r.store.unsentCount.value)
            assertEquals(2, r.store.lines.value.size)
        }

    @Test
    fun leaveWithoutAnOrderDoesNothing() =
        runTest {
            val r = rig()

            r.store.leave(LeaveReason.NAVIGATE)

            coVerify(exactly = 0) { r.f.engine.leaveOrder(any(), any()) }
        }

    @Test
    fun aFailedSlipIsEmittedOnSlipFailures() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, unsent = 1)))
            coEvery { r.f.engine.leaveOrder(1L, LeaveReason.SEND_TO_KITCHEN) } returns SlipOutcome.FAILED
            coEvery { r.f.engine.listCartLines(1L) } returns listOf(line(1, 10, unsent = 0))

            r.store.slipFailures.test {
                r.store.leave(LeaveReason.SEND_TO_KITCHEN)
                awaitItem()
            }
        }

    @Test
    fun aLeaveRefusalIsThrownToTheCaller() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), emptyList())
            val refusal = PosException.Coded("session_closed", emptyMap())
            coEvery { r.f.engine.leaveOrder(any(), any()) } throws refusal

            try {
                r.store.leave(LeaveReason.NAVIGATE)
                fail("expected the refusal")
            } catch (e: PosException.Coded) {
                assertSame(refusal, e)
            }
        }

    @Test
    fun leaveIfActiveWithoutAnOrderIsTrue() =
        runTest {
            val r = rig()

            assertTrue(r.store.leaveIfActive(LeaveReason.LOCK))

            coVerify(exactly = 0) { r.f.engine.leaveOrder(any(), any()) }
        }

    @Test
    fun leaveIfActiveLeavesTheOrderAndIsTrue() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, unsent = 1)))
            coEvery { r.f.engine.leaveOrder(1L, LeaveReason.LOCK) } returns SlipOutcome.PRINTED
            coEvery { r.f.engine.listCartLines(1L) } returns listOf(line(1, 10, unsent = 0))

            assertTrue(r.store.leaveIfActive(LeaveReason.LOCK))

            coVerify { r.f.engine.leaveOrder(1L, LeaveReason.LOCK) }
            assertEquals(0, r.store.unsentCount.value)
        }

    @Test
    fun leaveIfActiveThrowsARefusal() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), emptyList())
            coEvery { r.f.engine.leaveOrder(any(), any()) } throws PosException.Coded("session_closed", emptyMap())

            try {
                r.store.leaveIfActive(LeaveReason.IDLE)
                fail("expected the refusal")
            } catch (e: PosException.Coded) {
                assertEquals("session_closed", e.code)
            }
        }

    @Test
    fun preBillSignsThenReReadsAndReportsAFailedSlip() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, unsent = 2)))
            coEvery { r.f.engine.signPreBill(1L) } returns SlipOutcome.FAILED
            r.stubReads(order(id = 1, table = 3, totalCents = 1000), listOf(line(1, 10, unsent = 0)))

            r.store.slipFailures.test {
                r.store.preBill()
                awaitItem()
            }

            assertEquals(0, r.store.unsentCount.value)
            assertEquals(1000L, r.store.current.value?.totalCents)
        }

    @Test
    fun preBillWithoutAnOrderThrows() =
        runTest {
            val r = rig()

            try {
                r.store.preBill()
                fail("expected an error")
            } catch (e: IllegalStateException) {
                assertEquals("No active order", e.message)
            }
        }

    @Test
    fun preBillRefusalIsThrown() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), emptyList())
            coEvery { r.f.engine.signPreBill(1L) } throws PosException.Coded("order_no_lines", emptyMap())

            try {
                r.store.preBill()
                fail("expected the refusal")
            } catch (e: PosException.Coded) {
                assertEquals("order_no_lines", e.code)
            }
        }

    @Test
    fun closeUnpaidRefusalThrowsWithTheRawCodeAndKeepsTheOrder() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10)))
            coEvery { r.f.engine.closeUnpaid(1L, "insz", "0000") } returns result(false, "pin_locked|seconds=30")

            try {
                r.store.closeUnpaid("insz", "0000")
                fail("expected the refusal")
            } catch (e: OrderRefusedException) {
                assertEquals("pin_locked|seconds=30", e.raw)
            }

            assertEquals(1L, r.store.current.value?.id)
            coVerify(exactly = 0) { r.f.engine.setActiveOrder(null) }
        }

    @Test
    fun closeUnpaidRefusalWithoutAnErrorHasAnEmptyRaw() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), emptyList())
            coEvery { r.f.engine.closeUnpaid(any(), any(), any()) } returns result(false, null)

            try {
                r.store.closeUnpaid("insz", "1")
                fail("expected the refusal")
            } catch (e: OrderRefusedException) {
                assertEquals("", e.raw)
            }
        }

    @Test
    fun closeUnpaidSuccessClearsTheOrderAndTellsTheEngine() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10, unsent = 2)))
            coEvery { r.f.engine.closeUnpaid(1L, "insz", "1234") } returns result(true, slip = SlipOutcome.FAILED)
            coEvery { r.f.engine.setActiveOrder(null) } returns Unit

            r.store.slipFailures.test {
                r.store.closeUnpaid("insz", "1234")
                awaitItem()
            }

            assertNull(r.store.current.value)
            assertTrue(r.store.lines.value.isEmpty())
            assertEquals(0, r.store.unsentCount.value)
            coVerify { r.f.engine.setActiveOrder(null) }
        }

    @Test
    fun closeUnpaidWithoutAnOrderThrows() =
        runTest {
            val r = rig()

            try {
                r.store.closeUnpaid("insz", "1234")
                fail("expected an error")
            } catch (e: IllegalStateException) {
                assertEquals("No active order", e.message)
            }
        }

    @Test
    fun clearOrderIgnoresAFailureOfSetActiveOrder() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10)))
            coEvery { r.f.engine.setActiveOrder(null) } throws PosException.EngineClosed()

            r.store.clearOrder()

            assertNull(r.store.current.value)
            assertTrue(r.store.lines.value.isEmpty())
            assertNull(r.store.categoryOf(10))
        }

    @Test
    fun aSessionLockedEventResetsTheLocalState() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), emptyList())
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(10)
            r.stubReads(order(), listOf(line(1, 10, unsent = 1)))
            assertTrue(r.store.addItem(item(10)))
            assertEquals(1, r.store.unsentCount.value)

            r.events.send(PosEvent.SessionLocked(mockk(relaxed = true)))
            runCurrent()

            assertNull(r.store.current.value)
            assertTrue(r.store.lines.value.isEmpty())
            assertEquals(0, r.store.unsentCount.value)
            assertNull(r.store.categoryOf(10))
            // Local only: the engine already cleared its active order.
            coVerify(exactly = 0) { r.f.engine.setActiveOrder(null) }
        }

    @Test
    fun aResetDuringARefreshInFlightIsNotUndoneByIt() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10, unsent = 1)))
            val gate = CompletableDeferred<Order>()
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(11)
            coEvery { r.f.engine.getOrder(1L) } coAnswers { gate.await() }
            coEvery { r.f.engine.listCartLines(1L) } returns listOf(line(1, 10, unsent = 1), line(2, 11, unsent = 1))
            val added = async { r.store.addItem(item(11)) }
            runCurrent()

            // The refresh is suspended on getOrder; the session locks and the store forgets its order.
            r.events.send(PosEvent.SessionLocked(mockk(relaxed = true)))
            runCurrent()
            assertNull(r.store.current.value)
            gate.complete(order(id = 1))
            runCurrent()

            assertTrue(added.await())
            assertNull(r.store.current.value)
            assertTrue(r.store.lines.value.isEmpty())
            assertEquals(0, r.store.unsentCount.value)
        }

    @Test
    fun aResetDuringALoadInFlightDoesNotActivateTheOldOrder() =
        runTest {
            val r = rig()
            val gate = CompletableDeferred<Order>()
            coEvery { r.f.engine.getOrder(3L) } coAnswers { gate.await() }
            coEvery { r.f.engine.listCartLines(3L) } returns listOf(line(1, 10))
            coEvery { r.f.engine.setActiveOrder(3L) } returns Unit
            val loading = async { r.store.loadOrder(3L) }
            runCurrent()

            r.store.resetLocal()
            gate.complete(order(id = 3))
            runCurrent()
            loading.await()

            assertNull(r.store.current.value)
            coVerify(exactly = 0) { r.f.engine.setActiveOrder(3L) }
        }

    @Test
    fun anOrderLoadedAfterAResetIsShown() =
        runTest {
            val r = rig()
            r.store.resetLocal()

            r.store.seed(r, order(id = 4), listOf(line(1, 10)))

            assertEquals(4L, r.store.current.value?.id)
            assertEquals(1, r.store.lines.value.size)
        }

    @Test
    fun otherEventsLeaveTheStateAlone() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10)))

            r.events.send(PosEvent.Resync(1L))
            runCurrent()

            assertEquals(1L, r.store.current.value?.id)
        }

    @Test
    fun theTotalIsTheEnginesNotTheSumOfTheLines() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), emptyList())
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(10)
            // Two lines of 500 would sum to 1000; the engine total includes a rule the client never sees.
            r.stubReads(order(totalCents = 850), listOf(line(1, 10, unitPriceCents = 500), line(2, 11, unitPriceCents = 500)))

            assertTrue(r.store.addItem(item(10)))

            assertEquals(850L, r.store.current.value?.totalCents)
        }

    @Test
    fun unsentCountSumsUnitsAndIgnoresNegatives() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10, quantity = 4, unsent = 3), line(2, 11, unsent = 0), line(3, 12, unsent = -2)))

            assertEquals(3, r.store.unsentCount.value)
        }

    /** Puts [order] and [cart] in the store through the engine, as a screen would (loadOrder). */
    private suspend fun OrderStore.seed(
        r: Rig,
        order: Order,
        cart: List<CartLine>,
    ) {
        r.stubReads(order, cart)
        coEvery { r.f.engine.setActiveOrder(order.id) } returns Unit
        loadOrder(order.id)
    }
}
