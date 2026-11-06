package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.state.EFT_APPROVED
import dev.nuagemagique.pilipos.ui.state.eftPayment
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.GiftVoucherPrint
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TicketMedium

/**
 * The quick checkout of the UX audit (P1, P2, P6) in the view model: the exact amount is entered at once, the done
 * step returns to the tables by itself, and the rest after a part names the methods that can still pay it. The
 * rules are the engine's (the fake natives settle like it); what is proved here is what the screen asks and shows.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class CheckoutQuickViewModelTest {
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
        result: CheckoutResult = sale(),
        terminal: String? = null,
    ): CheckoutRig {
        val f = checkoutFixture(tmp.root, totalCents)
        coEvery { f.engine.getSettings() } returns posSettings().copy(paymentTerminal = terminal)
        coEvery { f.engine.listEftPayments(ORDER_ID) } returns emptyList()
        coEvery { f.engine.checkout(ORDER_ID, any(), any()) } returns result
        return startCheckout(f, context)
    }

    private fun delivery(
        email: EmailStatus = EmailStatus.NOT_REQUESTED,
        paperPrinted: Boolean = true,
        needsPaper: Boolean = false,
    ) = DeliveryOutcome(
        fiscalEventId = 99L,
        deliveryId = null,
        medium = TicketMedium.PAPER,
        email = email,
        recipientHint = null,
        paperPrinted = paperPrinted,
        needsPaper = needsPaper,
    )

    // ── P1: the exact amount is entered ───────────────────────────

    @Test
    fun cashStartsAtTheRoundedRestAndTheNextKeyReplacesIt() =
        runTest {
            val r = started(1_152)

            assertEquals(1_150L, r.state.entryCents)
            assertTrue(r.state.fresh)
            assertTrue(r.state.exactActive)
            assertTrue(r.state.canValidate)
            assertEquals(1_150L, r.state.collectCents)
            assertFalse(r.state.showReceived)
            r.type("7")
            assertEquals(7L, r.state.entryCents)
            assertFalse(r.state.fresh)
            // The primary button still collects the due, the received amount only decides the change.
            assertEquals(1_150L, r.state.collectCents)
            assertFalse(r.state.canValidate)
        }

    @Test
    fun aQuickNoteReplacesTheExactAmountAndShowsTheReceivedLine() =
        runTest {
            val r = started(1_152)

            r.model.setQuickAmount(1_500L)
            assertEquals(1_500L, r.state.entryCents)
            assertTrue(r.state.showReceived)
            assertEquals(350L, r.state.changeCents)
        }

    @Test
    fun theCashSaleIsOneTapOnTheCollectButton() =
        runTest {
            val r = started(1_152)

            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(cash), null) }
            assertEquals(CheckoutStep.DONE, r.state.step)
            assertEquals(0L, requireNotNull(r.state.done).changeCents)
        }

    @Test
    fun aSplitPartStartsAtItsOwnRoundedShare() =
        runTest {
            val r = started(1_486)

            r.model.selectSplit(2)
            // 14,86 in two parts: 7,43 each, 7,45 in cash.
            assertEquals(745L, r.state.entryCents)
            assertEquals(745L, r.state.collectCents)
            r.model.selectMethod(PaymentMethod.CARD)
            assertEquals(743L, r.state.entryCents)
            assertEquals(743L, r.state.collectCents)
        }

    @Test
    fun anEmptyOrderLeavesTheEntryEmpty() =
        runTest {
            val r = started(0)

            assertEquals(0L, r.state.entryCents)
            assertFalse(r.state.fresh)
        }

    // ── P2: the done step leaves by itself ────────────────────────

    @Test
    fun theDoneStepGoesToTheTablesAfterTwoSeconds() =
        runTest {
            val r = started(1_152)
            r.type("2000")
            r.model.validate()
            runCurrent()
            assertEquals(CheckoutStep.DONE, r.state.step)
            assertFalse(requireNotNull(r.state.done).needsAttention)

            testScheduler.advanceTimeBy(DONE_AUTO_RETURN_MS - 1)
            runCurrent()
            assertEquals(emptyList<CheckoutEffect>(), r.effects)

            testScheduler.advanceTimeBy(2)
            runCurrent()
            assertEquals(listOf<CheckoutEffect>(CheckoutEffect.NewOrder), r.effects)
            assertEquals(null, r.state.done)
        }

    @Test
    fun aTapBeforeTheTimerLeavesAtOnceAndOnlyOnce() =
        runTest {
            val r = started(1_152)
            r.model.validate()
            runCurrent()

            r.model.finish()
            runCurrent()
            assertEquals(listOf<CheckoutEffect>(CheckoutEffect.NewOrder), r.effects)

            // The timer and a second tap find nothing left to leave.
            r.model.finish()
            testScheduler.advanceTimeBy(DONE_AUTO_RETURN_MS * 2)
            runCurrent()
            assertEquals(listOf<CheckoutEffect>(CheckoutEffect.NewOrder), r.effects)
        }

    @Test
    fun finishDoesNothingBeforeASaleIsDone() =
        runTest {
            val r = started(1_152)

            r.model.finish()
            runCurrent()

            assertEquals(emptyList<CheckoutEffect>(), r.effects)
            assertEquals(CheckoutStep.PAY, r.state.step)
        }

    @Test
    fun aTakeawaySaleLeavesToTheTablesToo() =
        runTest {
            val f = checkoutFixture(tmp.root, 1_152)
            coEvery { f.engine.getOrder(ORDER_ID) } returns checkoutOrder(1_152, table = null)
            coEvery { f.engine.checkout(ORDER_ID, any(), any()) } returns sale()
            val r = startCheckout(f, context)

            r.model.validate()
            runCurrent()
            testScheduler.advanceTimeBy(DONE_AUTO_RETURN_MS + 1)
            runCurrent()

            assertEquals(listOf<CheckoutEffect>(CheckoutEffect.NewOrder), r.effects)
        }

    @Test
    fun aCardApprovedOnTheTerminalReturnsByItselfOnceTheSaleIsSigned() =
        runTest {
            val r = started(1_152, terminal = "worldline_terminal_api")
            coEvery { r.f.engine.eftPay(ORDER_ID, 1_152) } returns eftPayment(11, EFT_APPROVED, 1_152)

            // PAYER, Carte, Encaisser: the terminal approves, the sale is signed.
            r.model.selectMethod(PaymentMethod.CARD)
            assertEquals(1_152L, r.state.entryCents)
            r.model.validate()
            runCurrent()
            assertEquals(CheckoutStep.DONE, r.state.step)
            assertEquals(emptyList<CheckoutEffect>(), r.effects)

            testScheduler.advanceTimeBy(DONE_AUTO_RETURN_MS + 1)
            runCurrent()
            assertEquals(listOf<CheckoutEffect>(CheckoutEffect.NewOrder), r.effects)
        }

    private suspend fun TestScope.staysFor(result: CheckoutResult) {
        val r = started(1_152, result)
        r.model.validate()
        runCurrent()
        assertTrue(requireNotNull(r.state.done).needsAttention)

        testScheduler.advanceTimeBy(DONE_AUTO_RETURN_MS * 5)
        runCurrent()
        assertEquals(emptyList<CheckoutEffect>(), r.effects)
        assertEquals(CheckoutStep.DONE, r.state.step)

        // Only the operator leaves.
        r.model.finish()
        runCurrent()
        assertEquals(listOf<CheckoutEffect>(CheckoutEffect.NewOrder), r.effects)
    }

    @Test
    fun theDoneStepStaysWhenTheTicketCouldNotBePrinted() =
        runTest { staysFor(sale(delivery = delivery(paperPrinted = false, needsPaper = true))) }

    @Test
    fun theDoneStepStaysWhenTheDigitalTicketWaitsInTheOutbox() =
        runTest { staysFor(sale(delivery = delivery(email = EmailStatus.QUEUED, paperPrinted = false))) }

    @Test
    fun theDoneStepStaysWhenTheFdmWasNotConnected() = runTest { staysFor(sale(fdmConnected = false)) }

    @Test
    fun theDoneStepStaysWhenAGiftVoucherSlipHasANote() =
        runTest {
            val prints = listOf(GiftVoucherPrint(3L, "voucher", SlipOutcome.FAILED))
            staysFor(sale().copy(giftVoucherPrints = prints))
        }

    @Test
    fun aSentDigitalTicketDoesNotHoldTheDoneStep() =
        runTest {
            val r = started(1_152, sale(delivery = delivery(email = EmailStatus.SENT, paperPrinted = false)))

            r.model.validate()
            runCurrent()

            assertFalse(requireNotNull(r.state.done).needsAttention)
        }

    // ── P6: the rest after a part ─────────────────────────────────

    @Test
    fun noRestRowBeforeAPartIsTaken() =
        runTest {
            val r = started(1_486)

            assertFalse(r.state.showRest)
            assertEquals(listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER), r.state.methods)
        }

    @Test
    fun aMealVoucherPartLeavesARestWithEveryMethodStillOfferedAndTheMealVoucherKept() =
        runTest {
            val r = started(1_486)

            r.model.selectMethod(PaymentMethod.MEAL_VOUCHER)
            r.type("1152")
            r.model.validate()
            runCurrent()

            assertTrue(r.state.showRest)
            assertEquals(listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER), r.state.methods)
            // The method of the part taken stays selected (UX audit P3): the rest is exact until cash is chosen.
            assertEquals(PaymentMethod.MEAL_VOUCHER, r.state.method)
            assertEquals(334L, r.state.amounts.restShownCents)
            assertEquals(334L, r.state.entryCents)
            r.model.selectMethod(PaymentMethod.CASH)
            runCurrent()
            // Cash: the exact rest with the engine's rounding, ready to collect.
            assertEquals(335L, r.state.amounts.restShownCents)
            assertEquals(335L, r.state.entryCents)
            assertTrue(r.state.canValidate)
        }

    @Test
    fun tappingAMethodPreFillsTheRestSoTheNextPartIsThatTapAndCollect() =
        runTest {
            val r = started(1_486)
            r.model.selectMethod(PaymentMethod.MEAL_VOUCHER)
            r.type("1152")
            r.model.validate()
            runCurrent()

            r.model.selectMethod(PaymentMethod.CARD)
            assertEquals(334L, r.state.entryCents)
            assertEquals(334L, r.state.collectCents)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) {
                r.f.engine.checkout(
                    ORDER_ID,
                    listOf(PaymentPart(PaymentMethod.MEAL_VOUCHER, 1_152L), PaymentPart(PaymentMethod.CARD, 334L)),
                    null,
                )
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun cashStaysOfferedAndSelectedOnceACashPartIsTaken() =
        runTest {
            val r = started(6_031)

            r.model.selectSplit(2)
            r.model.validate()
            runCurrent()

            assertTrue(r.state.showRest)
            assertEquals(listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER), r.state.methods)
            assertEquals(PaymentMethod.CASH, r.state.method)
        }

    @Test
    fun noRestRowOnceNothingIsLeftToPay() =
        runTest {
            val r = started(1_152)

            r.model.selectMethod(PaymentMethod.MEAL_VOUCHER)
            r.model.validate()
            runCurrent()

            // The whole sale was one part: it is signed, no rest remains.
            assertFalse(r.state.showRest)
        }
}
