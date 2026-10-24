package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.OrderRefusedException
import io.mockk.coEvery
import io.mockk.coVerify
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
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

/** The cart actions of the order model: steps, removal, send to kitchen, bill, close unpaid. */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class OrderCartFlowTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val refusal = PosException.Coded("order_not_open", emptyMap())

    private val world
        get() =
            OrderWorld(
                cart =
                    listOf(
                        cartLine(1, 10, "Maki saumon", quantity = 2),
                        cartLine(2, 11, "Nigiri thon", quantity = 3, unitPriceCents = 480, unsent = 3),
                    ),
            )

    private suspend fun TestScope.ready(
        w: OrderWorld = world,
        vat: List<VatSplitRow> = emptyList(),
    ): OrderRig {
        val f = orderFixture(tmp.root, w)
        f.natives.vatRows = vat
        coEvery { f.engine.updateOrderLineQuantity(any(), any()) } returns Unit
        coEvery { f.engine.removeOrderLine(any()) } returns Unit
        val r = startOrder(f, w, context)
        r.login()
        runCurrent()
        return r
    }

    private fun checkout(
        success: Boolean,
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

    // ── State ─────────────────────────────────────────────────────

    @Test
    fun theCartStateCarriesTheUnsentCountTheVatRowsAndTheManagerRight() =
        runTest {
            val rows = listOf(VatSplitRow(VatCode.B, 12, 2_000, 214))
            val r = ready(vat = rows)

            assertEquals(3, r.state.cart.unsentCount)
            assertTrue(r.state.cart.canCloseUnpaid)
            assertEquals(rows, r.state.cart.vatRows)
            assertFalse(r.state.cart.sending)
            assertFalse(r.state.cart.preBilling)
        }

    @Test
    fun aCashierMayNotCloseUnpaid() =
        runTest {
            val w = world
            val f = orderFixture(tmp.root, w)
            val r = startOrder(f, w, context)
            val cashier = User(insz = "90010100123", name = "Bob", role = UserRole.CASHIER, active = true, hasPin = true)
            coEvery { f.engine.login(cashier.insz, any()) } returns cashier
            r.session.login(cashier.insz, "1234")
            runCurrent()

            assertFalse(r.state.cart.canCloseUnpaid)
            assertTrue(r.state.cart.canPay)
            r.model.askCloseUnpaid()
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    // ── Quantity and removal ──────────────────────────────────────

    @Test
    fun thePlusAndTheMinusSetTheQuantityOfTheLineThroughTheStore() =
        runTest {
            val r = ready()

            r.model.incrementLine(1)
            runCurrent()
            r.model.decrementLine(1)
            runCurrent()

            coVerify { r.f.engine.updateOrderLineQuantity(1L, 3) }
            coVerify { r.f.engine.updateOrderLineQuantity(1L, 1) }
        }

    @Test
    fun theMinusStopsAtOneAndAnUnknownLineIsIgnored() =
        runTest {
            val w = OrderWorld(cart = listOf(cartLine(1, 10, "Maki saumon", quantity = 1)))
            val r = ready(w)

            r.model.decrementLine(1)
            r.model.incrementLine(99)
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun theDeleteControlRemovesTheLineAtAnyQuantity() =
        runTest {
            val r = ready()

            r.model.removeLine(1)
            runCurrent()

            coVerify { r.f.engine.removeOrderLine(1L) }
        }

    @Test
    fun aRefusedStepShowsTheErrorToast() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.updateOrderLineQuantity(any(), any()) } throws refusal

            r.model.incrementLine(1)
            runCurrent()

            assertEquals(1, r.toasts.toasts.value.size)
            assertEquals(ToastVariant.Error, r.lastToast().variant)
        }

    @Test
    fun cartActionsAreIgnoredWhileAnotherOrderIsCurrent() =
        runTest {
            val r = ready()
            r.order.loadOrder(3L)

            r.model.incrementLine(1)
            r.model.removeLine(1)
            r.model.sendToKitchen()
            r.model.printPreBill()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
            coVerify(exactly = 0) { r.f.engine.removeOrderLine(any()) }
            coVerify(exactly = 0) { r.f.engine.leaveOrder(any(), any()) }
            coVerify(exactly = 0) { r.f.engine.signPreBill(any()) }
        }

    // ── Send to kitchen ───────────────────────────────────────────

    @Test
    fun sendToKitchenRegistersWhatWasOrderedAndSaysHowManyItemsWentOut() =
        runTest {
            val w = world
            val r = ready(w)
            coEvery { r.f.engine.leaveOrder(ORDER_ID, LeaveReason.SEND_TO_KITCHEN) } coAnswers
                {
                    w.cart = w.cart.map { cartLine(it.id, it.menuItemId, it.name, it.quantity, it.unitPriceCents) }
                    SlipOutcome.PRINTED
                }

            r.model.sendToKitchen()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.leaveOrder(ORDER_ID, LeaveReason.SEND_TO_KITCHEN) }
            assertEquals(ToastVariant.Success, r.lastToast().variant)
            assertEquals("Envoyé en cuisine · 3 articles", r.lastToast().message)
            assertEquals(0, r.state.cart.unsentCount)
            assertFalse(r.state.cart.sending)
        }

    @Test
    fun sendToKitchenDoesNothingWhenNothingIsUnsent() =
        runTest {
            val r = ready(OrderWorld(cart = listOf(cartLine(1, 10, "Maki saumon", quantity = 2))))

            r.model.sendToKitchen()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.leaveOrder(any(), any()) }
            assertEquals(0, r.toasts.toasts.value.size)
        }

    @Test
    fun sendToKitchenRunsOnceWhileItIsRunning() =
        runTest {
            val r = ready()
            val gate = CompletableDeferred<Unit>()
            coEvery { r.f.engine.leaveOrder(any(), any()) } coAnswers
                {
                    gate.await()
                    SlipOutcome.PRINTED
                }

            r.model.sendToKitchen()
            runCurrent()
            assertTrue(r.state.cart.sending)
            r.model.sendToKitchen()
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.leaveOrder(any(), any()) }
            assertFalse(r.state.cart.sending)
        }

    @Test
    fun aRefusedSendShowsTheFdmErrorWithItsPrefixAndFreesTheButton() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.leaveOrder(any(), any()) } throws refusal

            r.model.sendToKitchen()
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertTrue(r.lastToast().message.startsWith("Commande non enregistrée par le FDM : "))
            assertFalse(r.state.cart.sending)
        }

    // ── Provisional bill ──────────────────────────────────────────

    @Test
    fun theBillIsPrintedForADineInOrderWithLines() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.signPreBill(ORDER_ID) } returns SlipOutcome.PRINTED

            r.model.printPreBill()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.signPreBill(ORDER_ID) }
            assertEquals("Addition provisoire imprimée", r.lastToast().message)
            assertFalse(r.state.cart.preBilling)
        }

    @Test
    fun aRefusedBillShowsTheErrorWithItsPrefix() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.signPreBill(any()) } throws refusal

            r.model.printPreBill()
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertTrue(r.lastToast().message.startsWith("Addition non enregistrée : "))
            assertFalse(r.state.cart.preBilling)
        }

    @Test
    fun noBillWithoutLines() =
        runTest {
            val r = ready(OrderWorld())

            r.model.printPreBill()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.signPreBill(any()) }
        }

    // ── Close unpaid ──────────────────────────────────────────────

    @Test
    fun theManagerFlowOpensThePinDialogForTheLoggedInUser() =
        runTest {
            val r = ready()

            r.model.askCloseUnpaid()

            assertEquals(OrderDialog.CloseUnpaid("Alice"), r.state.dialog)
        }

    @Test
    fun thePinGoesToTheEngineWithTheInszOfTheManagerAndTheOrderIsCleared() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.closeUnpaid(ORDER_ID, ADMIN_INSZ, "4321") } returns checkout(true)
            coEvery { r.f.engine.setActiveOrder(null) } returns Unit

            r.model.askCloseUnpaid()
            r.model.closeUnpaid("4321")
            r.model.unpaidClosed()
            runCurrent()

            coVerify { r.f.engine.closeUnpaid(ORDER_ID, ADMIN_INSZ, "4321") }
            assertNull(r.state.order)
            assertEquals(OrderDialog.None, r.state.dialog)
            assertEquals("Vente clôturée sans paiement", r.lastToast().message)
            assertEquals(ToastVariant.Success, r.lastToast().variant)
        }

    @Test
    fun aRefusedPinIsThrownToTheDialogAndTheOrderStays() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.closeUnpaid(any(), any(), any()) } returns checkout(false, "pin_locked|seconds=30")

            val failure = runCatching { r.model.closeUnpaid("0000") }.exceptionOrNull()
            runCurrent()

            assertTrue(failure is OrderRefusedException)
            assertTrue(r.model.describe(failure!!).isNotBlank())
            assertEquals(ORDER_ID, r.state.order?.id)
        }
}
