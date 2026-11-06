package dev.nuagemagique.pilipos.ui.state

import app.cash.turbine.test
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.coVerifyOrder
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
import uniffi.pos_ffi.CartPriceChange
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.DiscountInput
import uniffi.pos_ffi.DiscountKind
import uniffi.pos_ffi.DiscountReason
import uniffi.pos_ffi.DiscountResult
import uniffi.pos_ffi.DiscountTarget
import uniffi.pos_ffi.HeldCourse
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.NextCourse
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.QuickNotes
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TableTransferResult
import uniffi.pos_ffi.TakeawayTag
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
        priceChanges: List<CartPriceChange> = emptyList(),
        kitchenNote: String? = null,
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
        discountCents = 0,
        priceChanges = priceChanges,
        kitchenNote = kitchenNote,
        course = 0,
        kitchenState = "unsent",
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
        hasModifiers = false,
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
    fun theOpenOrderOnScreenAtALockIsRememberedOnceForTheNextLogin() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 5), emptyList())

            r.events.send(PosEvent.SessionLocked(mockk(relaxed = true)))
            runCurrent()

            assertNull(r.store.current.value)
            assertEquals(5L, r.store.takeLockedOrderId())
            assertNull(r.store.takeLockedOrderId())
        }

    @Test
    fun anOnlineOrderAtItsCheckoutIsNotRememberedAtAnIdleLock() =
        runTest {
            val r = rig()
            r.stubReads(order(id = 9, totalCents = 1_500), listOf(line(1, 10)))
            r.store.beginOnlineCollect(9L, PaymentMethod.CASH)

            r.events.send(PosEvent.SessionLocked(mockk(relaxed = true)))
            runCurrent()

            // It was never the engine's active order: the same user logging back in lands on Tables.
            assertNull(r.store.current.value)
            assertNull(r.store.takeLockedOrderId())
        }

    @Test
    fun aLockWithNoOrderOnScreenRemembersNone() =
        runTest {
            val r = rig()

            r.events.send(PosEvent.SessionLocked(mockk(relaxed = true)))
            runCurrent()

            assertNull(r.store.takeLockedOrderId())
        }

    @Test
    fun theLastLockWinsOverAnOlderOneNobodyTook() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 5), emptyList())
            r.events.send(PosEvent.SessionLocked(mockk(relaxed = true)))
            runCurrent()

            // Nothing on screen at the second lock: the old order is not brought back.
            r.events.send(PosEvent.SessionLocked(mockk(relaxed = true)))
            runCurrent()

            assertNull(r.store.takeLockedOrderId())
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

    @Test
    fun transferringTheCurrentOrderMovesItAndReadsItAgain() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, unsent = 1)))
            val moved = TableTransferResult(1L, 3, 5, SlipOutcome.PRINTED, SlipOutcome.PRINTED)
            coEvery { r.f.engine.transferTable(1L, 5) } returns moved
            r.stubReads(order(id = 1, table = 5), listOf(line(1, 10, unsent = 0)))

            val result = r.store.transferTable(5)

            assertEquals(moved, result)
            assertEquals(5, r.store.current.value?.tableNumber)
            assertEquals(0, r.store.unsentCount.value)
        }

    @Test
    fun transferringAnotherOrderLeavesTheCurrentOneAlone() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), emptyList())
            coEvery { r.f.engine.transferTable(9L, 5) } returns
                TableTransferResult(9L, 4, 5, SlipOutcome.NOTHING_TO_PRINT, SlipOutcome.PRINTED)

            r.store.transferOrder(9L, 5)

            assertEquals(3, r.store.current.value?.tableNumber)
            coVerify(exactly = 1) { r.f.engine.getOrder(1L) }
        }

    @Test
    fun aTransferSlipThatDidNotPrintIsEmittedOnSlipFailures() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), emptyList())
            coEvery { r.f.engine.transferTable(1L, 5) } returns TableTransferResult(1L, 3, 5, SlipOutcome.FAILED, SlipOutcome.FAILED)
            r.stubReads(order(id = 1, table = 5), emptyList())

            r.store.slipFailures.test {
                r.store.transferTable(5)
                awaitItem()
                awaitItem()
            }
        }

    // ── Courses ───────────────────────────────────────────────────

    private fun Rig.stubCourses(
        enabled: Boolean,
        held: List<HeldCourse> = emptyList(),
    ) {
        coEvery { f.engine.getCourseSettings() } returns CourseSettings(enabled, emptyList())
        coEvery { f.engine.listHeldCourses(any()) } returns held
    }

    @Test
    fun theCoursesSettingAndTheHeldCoursesAreReadWithTheOrder() =
        runTest {
            val r = rig()
            r.stubCourses(true, listOf(HeldCourse(2, 1, 1)))
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))

            assertTrue(r.store.courseSettings.value.enabled)
            assertEquals(listOf(HeldCourse(2, 1, 1)), r.store.heldCourses.value)
        }

    @Test
    fun leavingTheOrderReadsTheHeldCoursesAgainSoTheNextCourseIsOfferedAtOnce() =
        runTest {
            val r = rig()
            r.stubCourses(true, emptyList())
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10, unsent = 1)))
            assertEquals(emptyList<HeldCourse>(), r.store.heldCourses.value)
            // The first slip carried the starters and held the mains.
            coEvery { r.f.engine.leaveOrder(1L, LeaveReason.SEND_TO_KITCHEN) } returns SlipOutcome.PRINTED
            coEvery { r.f.engine.listCartLines(1L) } returns listOf(line(1, 10, unsent = 0))
            r.stubCourses(true, listOf(HeldCourse(2, 1, 1)))

            r.store.leave(LeaveReason.SEND_TO_KITCHEN)

            assertEquals(listOf(HeldCourse(2, 1, 1)), r.store.heldCourses.value)
            coVerifyOrder {
                r.f.engine.leaveOrder(1L, LeaveReason.SEND_TO_KITCHEN)
                r.f.engine.listHeldCourses(1L)
            }
        }

    @Test
    fun aTransferRefusalIsThrownToTheCaller() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), emptyList())
            val refusal = PosException.Coded("table_transfer_target_busy", emptyMap())
            coEvery { r.f.engine.transferTable(any(), any()) } throws refusal

            try {
                r.store.transferTable(5)
                fail("expected the refusal")
            } catch (e: PosException.Coded) {
                assertSame(refusal, e)
            }
            assertEquals(3, r.store.current.value?.tableNumber)
        }

    private fun halfOff() = listOf(CartPriceChange("TILL-DISCOUNT", "Remise 50 %", "EVENT", -250))

    private fun discount(
        kind: DiscountKind = DiscountKind.PERCENT,
        value: Long? = 50,
    ) = DiscountInput(DiscountTarget.Order, kind, value, DiscountReason.STAFF_MEAL, null)

    @Test
    fun addingTheSameProductAgainOpensANewLineWhenTheLineHasAPriceChange() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, priceChanges = halfOff())))
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(10)

            r.store.addItem(item(10))

            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun addingTheSameProductAgainStillMergesIntoAPlainLine() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10)))
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 2) } returns Unit

            r.store.addItem(item(10))

            coVerify(exactly = 1) { r.f.engine.updateOrderLineQuantity(1L, 2) }
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun addingTheSameProductAgainOpensANewLineWhenTheLineHasAKitchenNote() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, kitchenNote = "sans wasabi")))
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(10)

            r.store.addItem(item(10))

            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun aKitchenNoteIsSentAndTheLinesAreReadAgain() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10)))
            coEvery { r.f.engine.setLineNote(1L, "sans wasabi") } returns SlipOutcome.NOTHING_TO_PRINT
            r.stubReads(order(id = 1, table = 3), listOf(line(1, 10, kitchenNote = "sans wasabi")))

            assertTrue(r.store.setLineNote(1L, "  sans wasabi "))

            assertEquals("sans wasabi", r.store.lines.value.single().kitchenNote)
            coVerify(exactly = 1) { r.f.engine.setLineNote(1L, "sans wasabi") }
        }

    @Test
    fun anEmptyKitchenNoteClearsIt() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10, kitchenNote = "sans wasabi")))
            coEvery { r.f.engine.setLineNote(1L, null) } returns SlipOutcome.NOTHING_TO_PRINT
            r.stubReads(order(id = 1, table = 3), listOf(line(1, 10)))

            assertTrue(r.store.setLineNote(1L, "   "))

            assertNull(r.store.lines.value.single().kitchenNote)
            coVerify(exactly = 1) { r.f.engine.setLineNote(1L, null) }
        }

    @Test
    fun aNoteSlipThatDidNotPrintIsEmittedOnSlipFailures() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10)))
            coEvery { r.f.engine.setLineNote(1L, "sans wasabi") } returns SlipOutcome.FAILED
            r.stubReads(order(id = 1, table = 3), listOf(line(1, 10, kitchenNote = "sans wasabi")))

            r.store.slipFailures.test {
                r.store.setLineNote(1L, "sans wasabi")
                awaitItem()
            }
        }

    @Test
    fun aRefusedNoteIsReportedAndTheCartKeepsItsLines() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3), listOf(line(1, 10)))
            val refusal = PosException.Coded("note_too_long", emptyMap())
            coEvery { r.f.engine.setLineNote(any(), any()) } throws refusal

            r.store.failures.test {
                assertFalse(r.store.setLineNote(1L, "x".repeat(61)))
                assertSame(refusal, awaitItem())
            }
            assertNull(r.store.lines.value.single().kitchenNote)
        }

    @Test
    fun theQuickNotesAreReadFromTheEngineEachTime() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getQuickNotes() } returnsMany
                listOf(QuickNotes(listOf("Sans wasabi", "Bien cuit")), QuickNotes(listOf("Épicé")))

            assertEquals(listOf("Sans wasabi", "Bien cuit"), r.store.quickNotes())
            assertEquals(listOf("Épicé"), r.store.quickNotes())
        }

    @Test
    fun aTillThatDoesNotAnswerHasNoQuickNotes() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getQuickNotes() } throws PosException.Coded("internal", emptyMap())

            assertEquals(emptyList<String>(), r.store.quickNotes())
        }

    @Test
    fun anEmptyListStaysEmpty() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getQuickNotes() } returns QuickNotes(emptyList())

            assertEquals(emptyList<String>(), r.store.quickNotes())
        }

    @Test
    fun applyingADiscountSendsTheInputWithThePinAndReadsTheOrderAgain() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))
            val input = discount()
            coEvery { r.f.engine.applyDiscount(1L, input, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3, totalCents = 250), SlipOutcome.NOTHING_TO_PRINT, null, false, null)
            r.stubReads(order(id = 1, table = 3, totalCents = 250), listOf(line(1, 10, priceChanges = halfOff())))

            r.store.applyDiscount(input, "insz", "4321")

            assertEquals(250L, r.store.current.value?.totalCents)
            assertEquals(1, r.store.lines.value.single().priceChanges.size)
            coVerify(exactly = 1) { r.f.engine.applyDiscount(1L, input, "insz", "4321") }
        }

    @Test
    fun aDiscountThatCouldNotBeRegisteredIsWarnedAndKept() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))
            val input = discount()
            coEvery { r.f.engine.applyDiscount(1L, input, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3), SlipOutcome.NOTHING_TO_PRINT, "fdm_unreachable", false, null)
            r.stubReads(order(id = 1, table = 3, totalCents = 250), listOf(line(1, 10, priceChanges = halfOff())))

            r.store.discountWarnings.test {
                r.store.applyDiscount(input, "insz", "4321")
                awaitItem()
            }
            assertEquals(250L, r.store.current.value?.totalCents)
        }

    @Test
    fun aDiscountSlipThatDidNotPrintIsEmittedOnSlipFailures() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))
            val input = discount()
            coEvery { r.f.engine.applyDiscount(1L, input, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3), SlipOutcome.FAILED, null, false, null)
            r.stubReads(order(id = 1, table = 3), listOf(line(1, 10)))

            r.store.slipFailures.test {
                r.store.applyDiscount(input, "insz", "4321")
                awaitItem()
            }
        }

    @Test
    fun withCoursesOffTheHeldCoursesAreNotRead() =
        runTest {
            val r = rig()
            r.stubCourses(false)
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))

            assertFalse(r.store.courseSettings.value.enabled)
            assertEquals(emptyList<HeldCourse>(), r.store.heldCourses.value)
            coVerify(exactly = 0) { r.f.engine.listHeldCourses(any()) }
        }

    @Test
    fun anEngineThatCannotAnswerLeavesCoursesOff() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getCourseSettings() } throws PosException.Coded("not_ready", emptyMap())
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))

            assertFalse(r.store.courseSettings.value.enabled)
            assertEquals(1, r.store.lines.value.size)
        }

    @Test
    fun aCourseChangeIsStoredAndTheCartAndHeldCoursesAreReadAgain() =
        runTest {
            val r = rig()
            r.stubCourses(true, listOf(HeldCourse(2, 1, 1)))
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))
            coEvery { r.f.engine.setLineCourse(1, 1) } returns Unit
            r.stubCourses(true, emptyList())

            assertTrue(r.store.setLineCourse(1, 1))

            coVerify(exactly = 1) { r.f.engine.setLineCourse(1, 1) }
            assertEquals(emptyList<HeldCourse>(), r.store.heldCourses.value)
        }

    @Test
    fun aRefusedCourseChangeIsReportedAndAnswersFalse() =
        runTest {
            val r = rig()
            r.stubCourses(true)
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))
            coEvery { r.f.engine.setLineCourse(any(), any()) } throws PosException.Coded("course_already_sent", emptyMap())

            r.store.failures.test {
                assertFalse(r.store.setLineCourse(1, 1))
                assertTrue(awaitItem() is PosException)
            }
        }

    @Test
    fun sendNextCourseAnswersTheEnginesResultAndReadsTheCartAgain() =
        runTest {
            val r = rig()
            r.stubCourses(true, listOf(HeldCourse(2, 1, 1), HeldCourse(3, 1, 1)))
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))
            val next = NextCourse(course = 2, signed = null, slip = SlipOutcome.PRINTED)
            coEvery { r.f.engine.sendNextCourse(1) } returns next
            r.stubCourses(true, listOf(HeldCourse(3, 1, 1)))

            assertEquals(next, r.store.sendNextCourse())

            assertEquals(listOf(HeldCourse(3, 1, 1)), r.store.heldCourses.value)
        }

    @Test
    fun aSlipThatDidNotPrintIsReportedOnTheSlipFailures() =
        runTest {
            val r = rig()
            r.stubCourses(true, listOf(HeldCourse(2, 1, 1)))
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))
            coEvery { r.f.engine.sendNextCourse(1) } returns NextCourse(2, null, SlipOutcome.FAILED)

            r.store.slipFailures.test {
                r.store.sendNextCourse()
                awaitItem()
            }
        }

    @Test
    fun aDiscountRefusalIsThrownAndTheCartKeepsItsLines() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))
            val refusal = PosException.Coded("pin_wrong", emptyMap())
            coEvery { r.f.engine.applyDiscount(any(), any(), any(), any()) } throws refusal

            try {
                r.store.applyDiscount(discount(), "insz", "0000")
                fail("expected the refusal")
            } catch (e: PosException.Coded) {
                assertSame(refusal, e)
            }
            assertEquals(500L, r.store.current.value?.totalCents)
            assertTrue(r.store.lines.value.single().priceChanges.isEmpty())
        }

    @Test
    fun aTypedPinIsHintedForTheOrderForAsLongAsTheEngineSaid() =
        runTest {
            val r = rig()
            var now = 10_000L
            r.store.clockMs = { now }
            r.store.seed(r, order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))
            val input = discount()
            coEvery { r.f.engine.applyDiscount(1L, input, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3, totalCents = 250), SlipOutcome.NOTHING_TO_PRINT, null, false, 120_000L)
            r.stubReads(order(id = 1, table = 3, totalCents = 250), listOf(line(1, 10, priceChanges = halfOff())))
            assertFalse(r.store.keepsManagerPin())

            val typed = r.store.applyDiscount(input, "insz", "4321")

            assertFalse(typed.pinReused)
            assertTrue(r.store.keepsManagerPin())
            now += 119_999L
            assertTrue(r.store.keepsManagerPin())
            now += 1L
            assertFalse(r.store.keepsManagerPin())
        }

    @Test
    fun theNextDiscountOfTheOrderPassesNoPinAndTheHintFollowsTheAnswer() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))
            val input = discount()
            coEvery { r.f.engine.applyDiscount(1L, input, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3), SlipOutcome.NOTHING_TO_PRINT, null, false, 120_000L)
            coEvery { r.f.engine.applyDiscount(1L, input, "insz", null) } returns
                DiscountResult(order(id = 1, table = 3), SlipOutcome.NOTHING_TO_PRINT, null, true, 90_000L)
            r.stubReads(order(id = 1, table = 3, totalCents = 250), listOf(line(1, 10)))
            r.store.applyDiscount(input, "insz", "4321")

            val reused = r.store.applyDiscount(input, "insz", null)

            assertTrue(reused.pinReused)
            coVerify(exactly = 1) { r.f.engine.applyDiscount(1L, input, "insz", null) }
            assertTrue(r.store.keepsManagerPin())
        }

    @Test
    fun aRemovalWithoutTheGrantLeavesNoHintAndForgettingDropsIt() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 250), listOf(line(1, 10, priceChanges = halfOff())))
            coEvery { r.f.engine.removeDiscount(1L, null, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3, totalCents = 500), SlipOutcome.NOTHING_TO_PRINT, null, false, null)
            r.stubReads(order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))

            r.store.removeDiscount(null, "insz", "4321")
            assertFalse(r.store.keepsManagerPin())

            coEvery { r.f.engine.applyDiscount(any(), any(), any(), any()) } returns
                DiscountResult(order(id = 1, table = 3), SlipOutcome.NOTHING_TO_PRINT, null, false, 120_000L)
            r.store.applyDiscount(discount(), "insz", "4321")
            assertTrue(r.store.keepsManagerPin())
            r.store.forgetManagerPin()
            assertFalse(r.store.keepsManagerPin())
        }

    @Test
    fun theHintIsForTheOrderThatWasDiscountedOnly() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))
            coEvery { r.f.engine.applyDiscount(any(), any(), any(), any()) } returns
                DiscountResult(order(id = 1, table = 3), SlipOutcome.NOTHING_TO_PRINT, null, false, 120_000L)
            r.stubReads(order(id = 1, table = 3, totalCents = 250), listOf(line(1, 10)))
            r.store.applyDiscount(discount(), "insz", "4321")
            assertTrue(r.store.keepsManagerPin())

            r.store.seed(r, order(id = 2, table = 4, totalCents = 500), listOf(line(2, 10)))

            assertFalse(r.store.keepsManagerPin())
        }

    @Test
    fun removingAnOrderDiscountPassesNoLineAndReadsTheOrderAgain() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 3, totalCents = 250), listOf(line(1, 10, priceChanges = halfOff())))
            coEvery { r.f.engine.removeDiscount(1L, null, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3, totalCents = 500), SlipOutcome.NOTHING_TO_PRINT, null, false, null)
            r.stubReads(order(id = 1, table = 3, totalCents = 500), listOf(line(1, 10)))

            r.store.removeDiscount(null, "insz", "4321")

            assertEquals(500L, r.store.current.value?.totalCents)
            assertTrue(r.store.lines.value.single().priceChanges.isEmpty())
        }

    @Test
    fun removingAnOfferedLinePassesTheLineId() =
        runTest {
            val r = rig()
            r.store.seed(
                r,
                order(id = 1, table = 3),
                listOf(line(4, 10, priceChanges = listOf(CartPriceChange("TILL-FREE-LINE", "Offert", "LINE", -500)))),
            )
            coEvery { r.f.engine.removeDiscount(1L, 4L, "insz", "4321") } returns
                DiscountResult(order(id = 1, table = 3), SlipOutcome.NOTHING_TO_PRINT, null, false, null)
            r.stubReads(order(id = 1, table = 3, totalCents = 500), listOf(line(4, 10)))

            r.store.removeDiscount(4L, "insz", "4321")

            coVerify(exactly = 1) { r.f.engine.removeDiscount(1L, 4L, "insz", "4321") }
        }

    @Test
    fun aRefusedReleaseIsThrownToTheCaller() =
        runTest {
            val r = rig()
            r.stubCourses(true, listOf(HeldCourse(2, 1, 1)))
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))
            coEvery { r.f.engine.sendNextCourse(1) } throws PosException.Coded("fdm_unreachable", emptyMap())

            val refusal = runCatching { r.store.sendNextCourse() }.exceptionOrNull()

            assertTrue(refusal is PosException)
            assertEquals(listOf(HeldCourse(2, 1, 1)), r.store.heldCourses.value)
        }

    @Test
    fun forgettingTheOrderForgetsItsHeldCourses() =
        runTest {
            val r = rig()
            r.stubCourses(true, listOf(HeldCourse(2, 1, 1)))
            r.store.seed(r, order(id = 1, table = 4), listOf(line(1, 10)))

            r.store.resetLocal()

            assertEquals(emptyList<HeldCourse>(), r.store.heldCourses.value)
        }

    // ── Adds: the beep and the flash (C9), the digit after an add (C7) ──

    @Test
    fun aNewLineAddIsNotedWithItsLineAndUnits() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())
            coEvery { r.f.engine.addOrderLine(any()) } returns orderLine(10)
            r.stubReads(order(), listOf(line(99, 10, quantity = 2, unsent = 2)))

            val add = r.store.addProduct(item(10), quantity = 2)

            assertEquals(99L, add?.lineId)
            assertEquals(10L, add?.itemId)
            assertEquals(2, add?.units)
            assertEquals(add, r.store.lastAdd.value)
        }

    @Test
    fun aMergedAddIsNotedOnTheMergedLineAndEachAddHasItsOwnSequence() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10, quantity = 2, unsent = 2)))
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 3) } returns Unit
            r.stubReads(order(), listOf(line(1, 10, quantity = 3, unsent = 3)))

            val first = r.store.addProduct(item(10))
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 4) } returns Unit
            r.stubReads(order(), listOf(line(1, 10, quantity = 4, unsent = 4)))
            val second = r.store.addProduct(item(10))

            assertEquals(1L, first?.lineId)
            assertTrue((second?.seq ?: 0) > (first?.seq ?: 0))
        }

    @Test
    fun aRefusedAddNotesNothing() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())
            coEvery { r.f.engine.addOrderLine(any()) } throws PosException.Coded("order_not_open", emptyMap())

            r.store.failures.test {
                assertNull(r.store.addProduct(item(10)))
                awaitItem()
            }
            assertNull(r.store.lastAdd.value)
        }

    @Test
    fun aLineAddedByAnotherCommandIsNotedToo() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())
            r.stubReads(order(), listOf(line(99, 10, unsent = 1)))

            assertTrue(r.store.addLineWith(item(10), 1) { orderLine(10) })

            assertEquals(99L, r.store.lastAdd.value?.lineId)
        }

    @Test
    fun addUnitsAddsTheExtraUnitsToTheSameLineAndKeepsEarlierOnes() =
        runTest {
            val r = rig()
            // The line already had 4 units; the tap merged one more, the digit asks for 3 in all: 2 extra.
            r.store.seed(r, order(), listOf(line(1, 10, quantity = 5, unsent = 5)))
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 7) } returns Unit
            r.stubReads(order(), listOf(line(1, 10, quantity = 7, unsent = 7)))

            assertTrue(r.store.addUnits(1, 2))

            coVerify { r.f.engine.updateOrderLineQuantity(1L, 7) }
            assertEquals(7, r.store.lines.value.single().quantity)
            assertEquals(2, r.store.lastAdd.value?.units)
        }

    @Test
    fun addUnitsNeverGoesUnderTheUnitsAlreadySentToTheKitchen() =
        runTest {
            val r = rig()
            // 5 units, 2 sent: whatever the delta, the quantity stays at 3 or more.
            r.store.seed(r, order(), listOf(line(1, 10, quantity = 5, unsent = 3)))
            coEvery { r.f.engine.updateOrderLineQuantity(1L, 3) } returns Unit
            r.stubReads(order(), listOf(line(1, 10, quantity = 3, unsent = 1)))

            assertTrue(r.store.addUnits(1, -10))

            coVerify { r.f.engine.updateOrderLineQuantity(1L, 3) }
        }

    @Test
    fun addUnitsWithNothingToChangeCallsNoCommandButStillConfirms() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10, quantity = 1, unsent = 1)))

            assertTrue(r.store.addUnits(1, 0))

            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
            assertEquals(1L, r.store.lastAdd.value?.lineId)
        }

    @Test
    fun addUnitsOnALineThatIsGoneDoesNothing() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), emptyList())

            assertFalse(r.store.addUnits(42, 2))

            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
            assertNull(r.store.lastAdd.value)
        }

    @Test
    fun resetLocalForgetsTheLastAdd() =
        runTest {
            val r = rig()
            r.store.seed(r, order(), listOf(line(1, 10, unsent = 1)))
            r.store.addUnits(1, 0)

            r.store.resetLocal()

            assertNull(r.store.lastAdd.value)
        }

    // ── Numbered takeaway (UX audit T2) ───────────────────────────

    private fun tag(
        orderId: Long = 5,
        name: String? = null,
        phone: String? = null,
    ) = TakeawayTag(orderId = orderId, day = "2026-10-09", number = 61, label = "E-61", customerName = name, customerPhone = phone)

    @Test
    fun aTakeawayTagIsReadWithTheOrder() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getTakeawayTag(5L) } returns tag(name = "Dupont")
            r.store.seed(r, order(id = 5), emptyList())

            assertEquals("E-61", r.store.takeawayTag.value?.label)
            assertEquals("Dupont", r.store.takeawayTag.value?.customerName)
        }

    @Test
    fun aTableHasNoTagAndTheEngineIsNotAskedForOne() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1, table = 4), emptyList())

            assertNull(r.store.takeawayTag.value)
            coVerify(exactly = 0) { r.f.engine.getTakeawayTag(any()) }
        }

    @Test
    fun aTillThatCannotAnswerShowsNoNumber() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getTakeawayTag(5L) } throws PosException.Coded("internal", emptyMap())
            r.store.seed(r, order(id = 5), emptyList())

            assertNull(r.store.takeawayTag.value)
            assertEquals(5L, r.store.current.value?.id)
        }

    @Test
    fun theCustomerIsSentWithTheOrderAndTheAnswerBecomesTheTag() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getTakeawayTag(5L) } returns tag()
            r.store.seed(r, order(id = 5), emptyList())
            coEvery { r.f.engine.setTakeawayCustomer(5L, "Dupont", "0470 12 34 56") } returns tag(name = "Dupont", phone = "0470 12 34 56")

            assertTrue(r.store.setTakeawayCustomer("Dupont", "0470 12 34 56"))

            assertEquals("Dupont", r.store.takeawayTag.value?.customerName)
            assertEquals("0470 12 34 56", r.store.takeawayTag.value?.customerPhone)
        }

    @Test
    fun aRefusedCustomerKeepsTheTagAndReportsTheFailure() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getTakeawayTag(5L) } returns tag()
            r.store.seed(r, order(id = 5), emptyList())
            coEvery { r.f.engine.setTakeawayCustomer(5L, any(), any()) } throws PosException.Coded("takeaway_name_invalid", emptyMap())

            assertFalse(r.store.setTakeawayCustomer("x".repeat(40), null))

            assertNull(r.store.takeawayTag.value?.customerName)
        }

    @Test
    fun forgettingTheOrderForgetsItsTag() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getTakeawayTag(5L) } returns tag()
            r.store.seed(r, order(id = 5), emptyList())
            assertTrue(r.store.takeawayTag.value != null)

            r.store.resetLocal()

            assertNull(r.store.takeawayTag.value)
        }

    // ── Online collect and the parts kept (UX audit O2, P4) ───────

    @Test
    fun anOnlineOrderIsLoadedWithoutBecomingTheEnginesActiveOrder() =
        runTest {
            val r = rig()
            r.stubReads(order(id = 9, totalCents = 1_500), listOf(line(1, 10)))

            r.store.beginOnlineCollect(9L, PaymentMethod.CARD)

            assertEquals(9L, r.store.current.value?.id)
            assertEquals(1, r.store.lines.value.size)
            assertEquals(PaymentMethod.CARD, r.store.onlineCollect.value?.method)
            coVerify(exactly = 0) { r.f.engine.setActiveOrder(any()) }
        }

    @Test
    fun leavingAnOnlineCheckoutRegistersNothingAndForgetsTheOrder() =
        runTest {
            val r = rig()
            r.stubReads(order(id = 9), listOf(line(1, 10, unsent = 1)))
            coEvery { r.f.engine.setActiveOrder(null) } returns Unit
            r.store.beginOnlineCollect(9L, PaymentMethod.CASH)
            r.store.holdParts(9L, listOf(HeldPart(PaymentMethod.CASH, 500L, 0L)))

            r.store.leave(LeaveReason.NAVIGATE)

            coVerify(exactly = 0) { r.f.engine.leaveOrder(any(), any()) }
            assertNull(r.store.current.value)
            assertNull(r.store.onlineCollect.value)
            assertNull(r.store.heldParts.value)
        }

    @Test
    fun theKeptPartsBelongToTheirOrderOnly() =
        runTest {
            val r = rig()
            val part = HeldPart(PaymentMethod.CASH, 700L, 0L)

            r.store.holdParts(4L, listOf(part))

            assertEquals(listOf(part), r.store.heldPartsFor(4L))
            assertTrue(r.store.heldPartsFor(5L).isEmpty())
            r.store.holdParts(4L, emptyList())
            assertNull(r.store.heldParts.value)
        }

    @Test
    fun anOrdinaryLeaveKeepsThePartsAndResettingForgetsThem() =
        runTest {
            val r = rig()
            r.store.seed(r, order(id = 1), listOf(line(1, 10)))
            coEvery { r.f.engine.leaveOrder(1L, LeaveReason.NAVIGATE) } returns SlipOutcome.NOTHING_TO_PRINT
            r.store.holdParts(1L, listOf(HeldPart(PaymentMethod.CASH, 100L, 0L)))

            r.store.leave(LeaveReason.NAVIGATE)
            assertEquals(1, r.store.heldPartsFor(1L).size)

            r.store.resetLocal()
            assertTrue(r.store.heldPartsFor(1L).isEmpty())
        }

    // ── Covers and zones (UX audit T5, T1) ────────────────────────

    @Test
    fun theCoversOfATableOrderAreReadWithTheOrder() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getOrderCovers(1L) } returns 4
            r.store.seed(r, order(id = 1, table = 4), emptyList())

            assertEquals(4, r.store.covers.value)
        }

    @Test
    fun aTakeawayHasNoCoversAndTheEngineIsNotAskedForThem() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getTakeawayTag(5L) } returns tag()
            r.store.seed(r, order(id = 5), emptyList())

            assertNull(r.store.covers.value)
            coVerify(exactly = 0) { r.f.engine.getOrderCovers(any()) }
        }

    @Test
    fun aTillThatCannotAnswerShowsNoCovers() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getOrderCovers(1L) } throws PosException.Coded("internal", emptyMap())
            r.store.seed(r, order(id = 1, table = 4), emptyList())

            assertNull(r.store.covers.value)
            assertEquals(1L, r.store.current.value?.id)
        }

    @Test
    fun theCoversAreSentWithTheOrderAndTheAnswerBecomesTheState() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getOrderCovers(1L) } returns null
            r.store.seed(r, order(id = 1, table = 4), emptyList())
            coEvery { r.f.engine.setOrderCovers(1L, 6) } returns 6

            assertTrue(r.store.setCovers(6))
            assertEquals(6, r.store.covers.value)

            coEvery { r.f.engine.setOrderCovers(1L, null) } returns null
            assertTrue(r.store.setCovers(null))
            assertNull(r.store.covers.value)
            coVerify { r.f.engine.setOrderCovers(1L, null) }
        }

    @Test
    fun aRefusedCoversKeepTheValueAndReportTheFailure() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getOrderCovers(1L) } returns 3
            r.store.seed(r, order(id = 1, table = 4), emptyList())
            coEvery { r.f.engine.setOrderCovers(1L, any()) } throws PosException.Coded("covers_invalid", mapOf("max" to "30"))

            r.store.failures.test {
                assertFalse(r.store.setCovers(31))
                assertTrue(awaitItem() is PosException)
            }
            assertEquals(3, r.store.covers.value)
        }

    @Test
    fun theCoversGoWithTheLocalOrder() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getOrderCovers(1L) } returns 3
            r.store.seed(r, order(id = 1, table = 4), emptyList())

            r.store.resetLocal()

            assertNull(r.store.covers.value)
        }

    private fun zonedPlan() =
        uniffi.pos_ffi.TablePlan(
            plan = "A 1-3, B 4-5",
            tableCount = 5L,
            zoneNames = "",
            zones = listOf(uniffi.pos_ffi.TableZone("A", null, listOf(1, 2, 3)), uniffi.pos_ffi.TableZone("B", null, listOf(4, 5))),
            log = emptyList(),
        )

    @Test
    fun withoutAnAnswerFromThePlanTheTablesAreWrittenAsNumbers() =
        runTest {
            val r = rig()

            assertEquals("19", r.store.tableLabel(19))
            assertEquals("", r.store.tableLabel(null))
            assertEquals("T19", r.store.tableShortLabel(19))
        }

    @Test
    fun theEngineBeingReadyReadsThePlanAndLabelsTheTables() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.stream.next() } coAnswers { Channel<PosEvent?>().receive() }
            coEvery { f.engine.getTablePlan() } returns zonedPlan()
            val store = OrderStore(f.gateway(this), f.events, backgroundScope)
            f.holder.open()
            runCurrent()

            assertEquals("A-2", store.tableLabel(2))
            assertEquals("B-5", store.tableLabel(5))
            // A table the plan leaves out has no zone.
            assertEquals("9", store.tableLabel(9))
            assertEquals("A-2", store.tableShortLabel(2))
            assertEquals("T9", store.tableShortLabel(9))
        }

    @Test
    fun aResyncReadsThePlanAgain() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.getTablePlan() } returns zonedPlan()

            r.events.send(PosEvent.Resync(missed = 1L))
            runCurrent()

            assertEquals("B-4", r.store.tableLabel(4))
        }

    @Test
    fun aPlanTheManagerSavedIsAppliedAndAPlanWithoutZonesWritesNumbersAgain() =
        runTest {
            val r = rig()

            r.store.applyTablePlan(zonedPlan())
            assertEquals("A-1", r.store.tableLabel(1))
            r.store.applyTablePlan(zonedPlan().copy(zones = emptyList()))
            assertEquals("1", r.store.tableLabel(1))
        }

    @Test
    fun aListOfTablesTeachesTheZonesOfEachOne() =
        runTest {
            val r = rig()
            r.store.applyTablePlan(zonedPlan())

            val base = uniffi.pos_ffi.TableInfo(1, null, null, "1", null, null, null, 0L, 0L, "free", null, false, null)
            r.store.learnTables(listOf(base, base.copy(tableNumber = 7, zone = "C", label = "C-7")))

            assertEquals("1", r.store.tableLabel(1))
            assertEquals("C-7", r.store.tableLabel(7))
            assertEquals("A-2", r.store.tableLabel(2))
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
