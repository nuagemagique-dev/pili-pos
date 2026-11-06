package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
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
import uniffi.pos_ffi.HandoverRequest
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart

/** The checkout of an unpaid online order (UX audit O2) and the parts kept when going back to the cart (P4). */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class CheckoutOnlineViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private suspend fun TestScope.online(
        totalCents: Long = 2_000L,
        startOnCard: Boolean = false,
    ): CheckoutRig {
        val f = checkoutFixture(tmp.root, totalCents)
        coEvery { f.engine.webshopHandoverOrder(any()) } returns sale()
        coEvery { f.engine.checkout(any(), any(), any()) } returns sale()
        return startCheckout(f, context, online = true, startOnCard = startOnCard)
    }

    private suspend fun TestScope.ordinary(totalCents: Long): CheckoutRig {
        val f = checkoutFixture(tmp.root, totalCents)
        coEvery { f.engine.checkout(any(), any(), any()) } returns sale()
        return startCheckout(f, context)
    }

    // ── The online order (O2) ─────────────────────────────────────

    @Test
    fun anOnlineOrderOpensTheCheckoutWithTheExactAmountAndStaysOffTheActiveOrder() =
        runTest {
            val r = online(2_000L)

            assertTrue(r.state.online)
            assertTrue(r.state.hasOrder)
            assertEquals(PaymentMethod.CASH, r.state.method)
            assertEquals(2_000L, r.state.entryCents)
            assertEquals(PaymentMethod.CASH, r.order.onlineCollect.value?.method)
            coVerify(exactly = 0) { r.f.engine.setActiveOrder(any()) }
        }

    @Test
    fun theCardButtonOfTheBoardStartsOnTheCardMethod() =
        runTest {
            val r = online(2_000L, startOnCard = true)

            assertEquals(PaymentMethod.CARD, r.state.method)
        }

    @Test
    fun theSaleIsTheHandoverWithTheCashPartAndTheChange() =
        runTest {
            val r = online(2_000L)
            val request = slot<HandoverRequest>()
            coEvery { r.f.engine.webshopHandoverOrder(capture(request)) } returns sale()

            r.model.validate()
            runCurrent()

            assertEquals(ORDER_ID, request.captured.orderId)
            assertEquals(listOf(PaymentPart(PaymentMethod.CASH, null)), request.captured.payments)
            assertEquals(CheckoutStep.DONE, r.state.step)
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
        }

    @Test
    fun aRefusedHandoverStaysOnThePayStepWithItsText() =
        runTest {
            val r = online(2_000L)
            coEvery { r.f.engine.webshopHandoverOrder(any()) } returns sale(success = false, error = "fdm_unavailable")

            r.model.validate()
            runCurrent()

            assertEquals(CheckoutStep.PAY, r.state.step)
            assertTrue(r.state.error != null)
        }

    @Test
    fun backAndTheEndOfTheSaleReturnToTheBoard() =
        runTest {
            val r = online(2_000L)

            r.model.back()
            runCurrent()
            assertEquals(listOf<CheckoutEffect>(CheckoutEffect.BackToBoard), r.effects)

            r.model.validate()
            runCurrent()
            r.model.finish()
            runCurrent()

            assertEquals(listOf(CheckoutEffect.BackToBoard, CheckoutEffect.BoardAfterSale), r.effects)
        }

    // ── The parts kept (P4) ───────────────────────────────────────

    @Test
    fun aPartTakenByHandStaysWithTheOrderWhenTheOperatorGoesBack() =
        runTest {
            val r = ordinary(6_000L)
            r.model.selectSplit(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            assertEquals(1, r.state.taken.size)

            assertEquals(listOf(PaymentMethod.CARD), r.order.heldPartsFor(ORDER_ID).map { it.method })
            assertEquals(3_000L, r.order.heldPartsFor(ORDER_ID).single().cents)
        }

    @Test
    fun thePartsComeBackWhenTheCheckoutOpensAgain() =
        runTest {
            val r = ordinary(6_000L)
            r.model.selectSplit(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            val again = CheckoutViewModel(r.entry, ORDER_ID)
            runCurrent()

            assertEquals(1, again.state.value.taken.size)
            assertEquals(PaymentMethod.CARD, again.state.value.taken.single().method)
            assertEquals(3_000L, again.state.value.taken.single().cents)
            assertEquals(3_000L, again.state.value.amounts.restCents)
        }

    @Test
    fun clearingThePartsForgetsThemEverywhere() =
        runTest {
            val r = ordinary(6_000L)
            r.model.selectSplit(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()

            r.model.clearParts()
            runCurrent()

            assertTrue(r.state.taken.isEmpty())
            assertNull(r.order.heldParts.value)
        }

    @Test
    fun aPartThatNoLongerFitsDropsEveryPartAndWarns() =
        runTest {
            val r = ordinary(6_000L)
            r.model.selectSplit(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            // The order shrinks meanwhile: the kept 30,00 no longer fits in the new total.
            coEvery { r.f.engine.getOrder(ORDER_ID) } returns checkoutOrder(2_000L)
            r.order.loadOrder(ORDER_ID)
            val again = CheckoutViewModel(r.entry, ORDER_ID)
            runCurrent()

            assertTrue(again.state.value.taken.isEmpty())
            assertFalse(r.order.heldPartsFor(ORDER_ID).isNotEmpty())
            val toast = r.toasts.toasts.value.last()
            assertEquals(ToastVariant.Warning, toast.variant)
            assertEquals(r.catalog.text("parts_dropped", AppLanguage.FR), toast.message)
        }
}
