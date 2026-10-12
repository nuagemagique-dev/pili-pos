package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.state.EFT_APPROVED
import dev.nuagemagique.pilipos.ui.state.EFT_CANCELLED
import dev.nuagemagique.pilipos.ui.state.EFT_DECLINED
import dev.nuagemagique.pilipos.ui.state.EFT_UNKNOWN
import dev.nuagemagique.pilipos.ui.state.EftPhase
import dev.nuagemagique.pilipos.ui.state.eftPayment
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
import uniffi.pos_ffi.EftPayment
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart

/** The card terminal in the checkout: the payment flows of the view model over a mocked engine. */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class CheckoutEftViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val cash = PaymentPart(PaymentMethod.CASH, null)

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun CheckoutRig.type(digits: String) {
        digits.forEach { model.pressKey(NumPadKey.Digit(it)) }
    }

    private suspend fun TestScope.started(
        totalCents: Long,
        terminal: String? = "worldline_terminal_api",
        training: Boolean = false,
        listed: List<EftPayment> = emptyList(),
    ): CheckoutRig {
        val f = checkoutFixture(tmp.root, totalCents)
        coEvery { f.engine.getSettings() } returns posSettings().copy(paymentTerminal = terminal, trainingMode = training)
        coEvery { f.engine.listEftPayments(ORDER_ID) } returns listed
        coEvery { f.engine.checkout(ORDER_ID, any(), any()) } returns sale()
        return startCheckout(f, context)
    }

    // ── Approved ──────────────────────────────────────────────────

    @Test
    fun anApprovedCardIsLockedAndTheSaleCarriesItsPaymentId() =
        runTest {
            val r = started(1_152)
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } returns eftPayment(11, EFT_APPROVED, 1_152)

            r.model.selectMethod(PaymentMethod.CARD)
            assertTrue(r.state.eftActive)
            assertTrue(r.state.viaTerminal)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.eftPay(ORDER_ID, 1_152) }
            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L, 11L)), null)
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
            assertFalse(r.eft.overlay.value.visible)
            assertTrue(r.eft.payments.value.isEmpty())
        }

    @Test
    fun theOverlayWaitsForTheCardWithTheAmountAndTheSalePartsStayUnsigned() =
        runTest {
            val r = started(2_000)
            val gate = CompletableDeferred<EftPayment>()
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_000) } coAnswers { gate.await() }

            r.model.selectSplit(2)
            r.model.selectMethod(PaymentMethod.CARD)
            assertEquals(1_000L, r.state.entryCents)
            r.model.validate()
            runCurrent()

            val overlay = r.eft.overlay.value
            assertEquals(EftPhase.WAITING, overlay.phase)
            assertEquals(1_000L, overlay.amountCents)
            assertEquals(1, overlay.part)
            assertEquals(2, overlay.parts)
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }

            // Approved: the part is locked and cash takes the rest of the split sale.
            gate.complete(eftPayment(12, EFT_APPROVED, 1_000))
            runCurrent()
            assertEquals(CheckoutStep.PAY, r.state.step)
            assertEquals(listOf(true), r.state.taken.map { it.locked })
            assertEquals("Bancontact", r.state.taken.single().brand)
            assertEquals(PaymentMethod.CASH, r.state.method)
            assertEquals(1_000L, r.state.amounts.restCents)

            r.type("1000")
            r.model.validate()
            runCurrent()
            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_000L, 12L), cash), null)
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun clearingThePartsKeepsACardTheTerminalCharged() =
        runTest {
            val r = started(2_000)
            coEvery { r.f.engine.eftPay(ORDER_ID, 800) } returns eftPayment(13, EFT_APPROVED, 800)

            r.model.selectMethod(PaymentMethod.CARD)
            r.type("800")
            r.model.validate()
            runCurrent()
            assertEquals(1, r.state.taken.size)

            r.model.clearParts()
            runCurrent()
            assertEquals(listOf(13L), r.state.taken.map { it.eftPaymentId })
            assertEquals(PaymentMethod.CASH, r.state.method)
        }

    // ── Declined, cancelled, refused ──────────────────────────────

    @Test
    fun aDeclinedCardLeavesTheEntryEditableAndManualEntryTakesOver() =
        runTest {
            val r = started(1_152)
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } returns eftPayment(14, EFT_DECLINED, 1_152, detail = "refused by the bank")

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()

            val overlay = r.eft.overlay.value
            assertEquals(EftPhase.RESULT, overlay.phase)
            assertEquals(EFT_DECLINED, overlay.payment?.status)
            assertEquals("refused by the bank", overlay.payment?.detail)
            r.eft.dismiss()
            runCurrent()

            assertEquals(CheckoutStep.PAY, r.state.step)
            assertTrue(r.state.taken.isEmpty())
            assertEquals(1_152L, r.state.entryCents)
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }

            // The terminal is down: the card is typed by hand, no payment id.
            r.model.setCardManual(true)
            assertTrue(r.state.cardManual)
            assertFalse(r.state.viaTerminal)
            r.model.validate()
            runCurrent()
            coVerify(exactly = 1) { r.f.engine.eftPay(any(), any()) }
            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L)), null) }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun cancelAsksTheTerminalToStopAndTheCancelledPaymentChargesNothing() =
        runTest {
            val r = started(1_152)
            val gate = CompletableDeferred<EftPayment>()
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } coAnswers { gate.await() }
            coEvery { r.f.engine.eftAbort() } returns Unit

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            r.eft.abort()
            runCurrent()
            coVerify(exactly = 1) { r.f.engine.eftAbort() }
            assertTrue(r.eft.overlay.value.abortRequested)

            gate.complete(eftPayment(15, EFT_CANCELLED, 1_152))
            runCurrent()
            assertEquals(EFT_CANCELLED, r.eft.overlay.value.payment?.status)
            r.eft.dismiss()
            runCurrent()

            assertEquals(CheckoutStep.PAY, r.state.step)
            assertTrue(r.state.taken.isEmpty())
            assertTrue(r.state.canValidate)
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
        }

    @Test
    fun aRefusedRequestShowsItsReasonAndTheSaleStaysOpen() =
        runTest {
            val r = started(1_152)
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } throws
                uniffi.pos_ffi.PosException.Coded("eft_unreachable", mapOf("detail" to "no route"))

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()

            val overlay = r.eft.overlay.value
            assertEquals(EftPhase.RESULT, overlay.phase)
            assertNull(overlay.payment)
            assertTrue(overlay.failure.orEmpty().contains("no route"))
            r.eft.dismiss()
            runCurrent()
            assertEquals(CheckoutStep.PAY, r.state.step)
        }

    // ── Unknown answers ───────────────────────────────────────────

    @Test
    fun anUnknownAnswerBlocksTheSaleUntilTheTerminalIsAsked() =
        runTest {
            val r = started(1_152)
            val unknown = eftPayment(16, EFT_UNKNOWN, 1_152)
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } returns unknown
            coEvery { r.f.engine.eftCheck(16) } returns eftPayment(16, EFT_APPROVED, 1_152)

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            assertTrue(r.eft.overlay.value.unknown)

            // "Fermer": the payment stays, the sale is blocked and says so.
            r.eft.dismiss()
            runCurrent()
            assertEquals(listOf(16L), r.state.eftOpen.map { it.id })
            assertTrue(r.state.eftBlocked)
            assertFalse(r.state.canValidate)
            r.model.validate()
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }

            // "Verifier": approved, the part is locked and the sale goes through.
            r.model.verifyOpen(unknown)
            runCurrent()
            runCurrent()
            assertFalse(r.state.eftBlocked)
            assertEquals(listOf(16L), r.state.taken.map { it.eftPaymentId })
            assertTrue(r.state.amounts.allPaid)
            assertTrue(r.state.canValidate)
            r.model.validate()
            runCurrent()
            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L, 16L)), null)
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun theOverlayCheckButtonApprovesAnUnknownPaymentAndTheSaleContinues() =
        runTest {
            val r = started(1_152)
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } returns eftPayment(17, EFT_UNKNOWN, 1_152)
            coEvery { r.f.engine.eftCheck(17) } returns eftPayment(17, EFT_APPROVED, 1_152)

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            r.eft.check()
            runCurrent()
            runCurrent()

            // The card was charged for the whole sale: signed at once with its payment id.
            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L, 17L)), null)
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    // ── Recovery ──────────────────────────────────────────────────

    @Test
    fun anApprovedUnusedPaymentFoundOnOpeningBecomesALockedPartThatPaysTheSale() =
        runTest {
            val r = started(1_152, listed = listOf(eftPayment(20, EFT_APPROVED, 1_152)))

            assertEquals(listOf(20L), r.state.taken.map { it.eftPaymentId })
            assertTrue(r.state.taken.single().locked)
            assertTrue(r.state.amounts.allPaid)
            assertEquals(0L, r.state.amounts.restCents)
            assertTrue(r.state.canValidate)

            // It cannot be removed.
            r.model.clearParts()
            runCurrent()
            assertEquals(1, r.state.taken.size)

            r.model.validate()

            runCurrent()
            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L, 20L)), null)
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun aPartlyPaidOrderLeavesTheRestToAnotherMethod() =
        runTest {
            val r = started(1_152, listed = listOf(eftPayment(21, EFT_APPROVED, 500, brand = null)))

            assertEquals(652L, r.state.amounts.restCents)
            assertFalse(r.state.amounts.allPaid)
            r.type("700")
            assertTrue(r.state.canValidate)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 500L, 21L), cash), null)
            }
        }

    @Test
    fun aPaymentWaitingForItsAnswerOnOpeningBlocksTheSale() =
        runTest {
            val r = started(1_152, listed = listOf(eftPayment(22, EFT_UNKNOWN, 1_152)))

            assertEquals(listOf(22L), r.state.eftOpen.map { it.id })
            assertTrue(r.state.eftBlocked)
            r.model.setExact()
            assertFalse(r.state.canValidate)
            r.model.validate()
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
        }

    // ── Manual flow unchanged ─────────────────────────────────────

    @Test
    fun withoutALinkedTerminalCardsAreTypedAsBefore() =
        runTest {
            val r = started(1_152, terminal = null)

            assertFalse(r.state.eftActive)
            r.model.selectMethod(PaymentMethod.CARD)
            assertFalse(r.state.viaTerminal)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.eftPay(any(), any()) }
            coVerify(exactly = 0) { r.f.engine.listEftPayments(any()) }
            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L)), null) }
        }

    @Test
    fun inTrainingTheCardIsTypedByHandWhateverTheTerminal() =
        runTest {
            val r = started(1_152, training = true)

            assertFalse(r.state.eftActive)
            r.model.selectMethod(PaymentMethod.CARD)
            assertFalse(r.state.viaTerminal)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.eftPay(any(), any()) }
            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L)), null) }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    // ── The signature fails after the card was charged ────────────

    @Test
    fun aRefusedSaleKeepsTheLockedCardAndCanBeTriedAgain() =
        runTest {
            val r = started(1_152)
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } returns eftPayment(30, EFT_APPROVED, 1_152)
            coEvery { r.f.engine.checkout(ORDER_ID, any(), any()) } returns sale(success = false, error = "FDM not connected")

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            assertEquals(CheckoutStep.PAY, r.state.step)
            assertTrue(r.state.error.orEmpty().isNotEmpty())
            assertEquals(listOf(30L), r.state.taken.map { it.eftPaymentId })
            assertTrue(r.state.canValidate)

            coEvery { r.f.engine.checkout(ORDER_ID, any(), any()) } returns sale()
            r.model.validate()
            runCurrent()
            assertEquals(CheckoutStep.DONE, r.state.step)
            // The card was charged once.
            coVerify(exactly = 1) { r.f.engine.eftPay(any(), any()) }
        }
}
