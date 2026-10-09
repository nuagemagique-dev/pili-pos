package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.just
import io.mockk.Runs
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
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
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TicketDeliveryRequest
import uniffi.pos_ffi.TicketMedium

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class CheckoutViewModelTest {
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

    private fun uniffi.pos_ffi.PosEngineInterface.stubSale(result: uniffi.pos_ffi.CheckoutResult = sale()) {
        coEvery { checkout(ORDER_ID, any(), any()) } returns result
    }

    private suspend fun TestScope.started(
        totalCents: Long,
        lines: List<uniffi.pos_ffi.CartLine> = listOf(cartLine(1, quantity = 2)),
        mail: MailStatus = mailOff,
    ): CheckoutRig {
        val f = checkoutFixture(tmp.root, totalCents, lines, mail)
        f.engine.stubSale()
        return startCheckout(f, context)
    }

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun withTheEngineReadyAndSomebodyLoggedInItLoadsTheRoutesOrderThroughTheStore() =
        runTest {
            val r = started(1_152)

            assertTrue(r.state.hasOrder)
            assertTrue(r.state.loadTried)
            assertEquals(1_152L, r.state.totalCents)
            assertEquals(5, r.state.tableNumber)
            assertEquals(2, r.state.itemCount)
            assertEquals(CheckoutStep.PAY, r.state.step)
            coVerify(exactly = 1) { r.f.engine.getOrder(ORDER_ID) }
            assertEquals(emptyList<CheckoutEffect>(), r.effects)
        }

    @Test
    fun withNobodyLoggedInNoOrderIsLoadedAndNothingIsNavigated() =
        runTest {
            val f = checkoutFixture(tmp.root, 1_152)
            val r = startCheckout(f, context, user = null)

            assertFalse(r.state.hasOrder)
            assertFalse(r.state.canTakePayment)
            coVerify(exactly = 0) { f.engine.getOrder(any()) }
            assertEquals(emptyList<CheckoutEffect>(), r.effects)
        }

    @Test
    fun withTheEngineClosedNothingTalksToTheEngine() =
        runTest {
            val f = checkoutFixture(tmp.root, 1_152)
            val r = startCheckout(f, context, open = false, user = null)

            assertFalse(r.state.hasOrder)
            assertEquals(0L, r.state.amounts.restCents)
            coVerify(exactly = 0) { f.engine.getOrder(any()) }
            coVerify(exactly = 0) { f.engine.checkMailStatus() }
        }

    @Test
    fun aRefusedLoadShowsTheErrorAndKeepsTheEmptyState() =
        runTest {
            val f = checkoutFixture(tmp.root, 1_152)
            coEvery { f.engine.getOrder(ORDER_ID) } throws PosException.Coded("order_not_open", emptyMap())
            val r = startCheckout(f, context)

            assertFalse(r.state.hasOrder)
            assertTrue(r.state.loadTried)
            assertEquals(ToastVariant.Error, r.toasts.toasts.value.last().variant)
        }

    @Test
    fun theMailStatusIsReadWhenThePayStepOpens() =
        runTest {
            val mail = MailStatus(configured = true, reachable = true, allowDigitalOnly = true)
            val r = started(1_152, mail = mail)

            assertEquals(mail, r.state.mailStatus)
            coVerify(atLeast = 1) { r.f.engine.checkMailStatus() }
        }

    // ── Amounts come from the engine ──────────────────────────────

    @Test
    fun theRestAndTheRoundedCashDueComeFromTheEnginesSettlePreview() =
        runTest {
            val r = started(1_152)

            assertEquals(1_152L, r.state.amounts.restCents)
            assertEquals(1_152L, r.state.amounts.dueCents)
            assertEquals(1_150L, r.state.amounts.roundedDueCents)
            assertEquals(1_150L, r.state.amounts.restShownCents)
            assertEquals(listOf(1_500L, 2_000L, 5_000L), r.state.amounts.quickAmountsCents)
            assertTrue(r.natives.previews.contains(1_152L to listOf(cash)))
        }

    // ── Single cash ───────────────────────────────────────────────

    @Test
    fun singleCashWithChangeSendsOneCashPartWithoutAmountAndKeepsTheChange() =
        runTest {
            val r = started(1_152)

            r.type("2000")
            assertEquals(2_000L, r.state.entryCents)
            assertTrue(r.state.canPayCash)
            assertEquals(850L, r.state.changeCents)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(cash), null) }
            assertEquals(CheckoutStep.DONE, r.state.step)
            val done = requireNotNull(r.state.done)
            assertEquals(850L, done.changeCents)
            assertEquals(1_152L, done.totalCents)
            assertEquals(99L, done.fiscalEventId)
            assertTrue(done.fdmConnected)
            assertNull(r.order.current.value)
        }

    @Test
    fun cashBelowTheRoundedDueCannotBeValidated() =
        runTest {
            val r = started(1_152)

            r.type("1100")
            assertFalse(r.state.canPayCash)
            assertFalse(r.state.canValidate)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
            assertEquals(CheckoutStep.PAY, r.state.step)
        }

    @Test
    fun theRoundedDueIsEnoughForCashAndGivesNoChange() =
        runTest {
            val r = started(1_152)

            r.model.setExact()
            assertEquals(1_150L, r.state.entryCents)
            assertTrue(r.state.exactActive)
            assertTrue(r.state.canValidate)
            assertEquals(0L, r.state.changeCents)
        }

    @Test
    fun aQuickAmountFillsTheEntryAndTheNextKeyReplacesIt() =
        runTest {
            val r = started(1_152)

            r.model.setQuickAmount(1_500L)
            assertEquals(1_500L, r.state.entryCents)
            assertTrue(r.state.quickActive(1_500L))
            r.type("7")
            assertEquals(7L, r.state.entryCents)
            assertFalse(r.state.quickActive(1_500L))
        }

    @Test
    fun theKeysBuildTheAmountLikeATerminalAndCapAtSixDigits() =
        runTest {
            val r = started(1_152)

            r.type("1234567")
            assertEquals(123_456L, r.state.entryCents)
            r.model.pressKey(NumPadKey.Backspace)
            assertEquals(12_345L, r.state.entryCents)
            r.model.pressKey(NumPadKey.Clear)
            assertEquals(0L, r.state.entryCents)
        }

    // ── Card and meal vouchers ────────────────────────────────────

    @Test
    fun cardIsPrefilledWithTheAmountDueAndSendsItExactly() =
        runTest {
            val r = started(1_152)

            r.model.selectMethod(PaymentMethod.CARD)
            assertEquals(1_152L, r.state.entryCents)
            assertTrue(r.state.fresh)
            assertTrue(r.state.exactActive)
            assertTrue(r.state.canValidate)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_152L)), null) }
            assertEquals(CheckoutStep.DONE, r.state.step)
            assertEquals(0L, r.state.done?.changeCents)
        }

    @Test
    fun aKeyReplacesThePrefilledCardAmount() =
        runTest {
            val r = started(1_152)

            r.model.selectMethod(PaymentMethod.CARD)
            r.type("5")

            assertEquals(5L, r.state.entryCents)
            assertFalse(r.state.fresh)
        }

    @Test
    fun aMealVoucherAboveWhatIsLeftIsRefusedByTheScreen() =
        runTest {
            val r = started(1_152)

            r.model.selectMethod(PaymentMethod.MEAL_VOUCHER)
            r.type("99999")
            assertTrue(r.state.exactTooHigh)
            assertFalse(r.state.canValidate)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
            assertEquals(CheckoutStep.PAY, r.state.step)
            assertEquals(emptyList<TakenPart>(), r.state.taken)
        }

    @Test
    fun aMealVoucherBelowTheTotalIsATakenPartAndTheRestStaysToPay() =
        runTest {
            val r = started(1_152)

            r.model.selectMethod(PaymentMethod.MEAL_VOUCHER)
            r.type("500")
            r.model.validate()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
            assertEquals(listOf(TakenPart(PaymentMethod.MEAL_VOUCHER, 500L, 0L)), r.state.taken)
            assertEquals(652L, r.state.amounts.restCents)
            assertEquals(PaymentMethod.CASH, r.state.method)
            assertEquals(650L, r.state.amounts.restShownCents)
        }

    @Test
    fun cashIsRefusedOnceACashPartIsTaken() =
        runTest {
            val r = started(6_031)
            r.model.selectSplit(2)
            r.type("3016")
            r.model.validate()
            runCurrent()

            assertEquals(PaymentMethod.CARD, r.state.method)
            r.model.selectMethod(PaymentMethod.CASH)
            assertEquals(PaymentMethod.CARD, r.state.method)
            assertTrue(r.state.hasCashPart)
        }

    // ── Split ─────────────────────────────────────────────────────

    @Test
    fun splittingInThreeSharesTheTotalAndSendsTheCashPartLastWithoutAmount() =
        runTest {
            val r = started(6_031)

            r.model.selectSplit(3)
            assertEquals(listOf(2_011L, 2_010L, 2_010L), r.state.amounts.shares)
            assertEquals(
                listOf(
                    PartChip(ChipState.CURRENT, PaymentMethod.CASH, 2_011L),
                    PartChip(ChipState.NEXT, null, 2_010L),
                    PartChip(ChipState.NEXT, null, 2_010L),
                ),
                r.state.amounts.chips,
            )
            assertEquals(2_010L, r.state.amounts.roundedDueCents)

            // Part 1: card for its share.
            r.model.selectMethod(PaymentMethod.CARD)
            assertEquals(2_011L, r.state.entryCents)
            r.model.validate()
            runCurrent()
            assertEquals(1, r.state.taken.size)
            assertEquals(PaymentMethod.CASH, r.state.method)
            assertEquals(4_020L, r.state.amounts.restCents)
            assertEquals(2_010L, r.state.amounts.dueCents)

            // Part 2: cash for its share, taken before the last part.
            r.type("2010")
            r.model.validate()
            runCurrent()
            assertEquals(2, r.state.taken.size)
            assertEquals(PaymentMethod.CARD, r.state.method)
            assertEquals(2_010L, r.state.amounts.restCents)
            assertEquals(2_010L, r.state.entryCents)
            assertEquals(CheckoutStep.PAY, r.state.step)
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }

            // Part 3: card for what is left; the sale goes out with the cash part last and without amount.
            r.model.validate()
            runCurrent()
            coVerify(exactly = 1) {
                r.f.engine.checkout(
                    ORDER_ID,
                    listOf(PaymentPart(PaymentMethod.CARD, 2_011L), PaymentPart(PaymentMethod.CARD, 2_010L), cash),
                    null,
                )
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun theLastPartInCashIsRoundedByTheEngineOnItsOwnRest() =
        runTest {
            val r = started(1_003)
            r.model.selectSplit(2)
            assertEquals(listOf(502L, 501L), r.state.amounts.shares)

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()

            assertEquals(501L, r.state.amounts.restCents)
            assertEquals(501L, r.state.amounts.dueCents)
            assertEquals(500L, r.state.amounts.roundedDueCents)
            assertEquals(500L, r.state.amounts.restShownCents)
            r.type("500")
            assertTrue(r.state.canValidate)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 502L), cash), null)
            }
        }

    @Test
    fun aSplitIsOnlyAllowedBeforeTheFirstPartAndWithAtLeastAsManyCentsAsParts() =
        runTest {
            val r = started(2)

            assertTrue(r.state.splitAllowed(1))
            assertTrue(r.state.splitAllowed(2))
            assertFalse(r.state.splitAllowed(3))
            r.model.selectSplit(3)
            assertEquals(1, r.state.splitCount)
            r.model.selectSplit(2)
            assertEquals(2, r.state.splitCount)
        }

    @Test
    fun noSplitAfterAPartWasTakenAndClearingThePartsStartsOver() =
        runTest {
            val r = started(6_031)
            r.model.selectSplit(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            assertEquals(1, r.state.taken.size)
            assertFalse(r.state.splitAllowed(3))
            r.model.selectSplit(3)
            assertEquals(2, r.state.splitCount)

            r.model.clearParts()
            runCurrent()

            assertEquals(emptyList<TakenPart>(), r.state.taken)
            assertEquals(PaymentMethod.CASH, r.state.method)
            assertEquals(6_031L, r.state.amounts.restCents)
            assertTrue(r.state.splitAllowed(3))
        }

    // ── Refusals ──────────────────────────────────────────────────

    @Test
    fun anEngineRefusalGoesBackToThePayStepWithItsTextAndKeepsTheOrder() =
        runTest {
            val r = started(1_152)
            val refusal = PosException.Coded("payment_exceeds_total", emptyMap())
            coEvery { r.f.engine.checkout(ORDER_ID, any(), any()) } throws refusal

            r.type("2000")
            r.model.validate()
            runCurrent()

            assertEquals(CheckoutStep.PAY, r.state.step)
            assertEquals(r.errorTexts.describe(refusal, AppLanguage.FR), r.state.error)
            assertNotNull(r.order.current.value)
            assertNull(r.state.done)
            assertEquals(2_000L, r.state.entryCents)
            coVerify(atLeast = 2) { r.f.engine.checkMailStatus() }
        }

    @Test
    fun aResultThatIsNotASuccessShowsTheEnginesCodeAndKeepsTheOrder() =
        runTest {
            val r = started(1_152)
            r.f.engine.stubSale(sale(success = false, error = "payment_invalid", fiscalEventId = null))

            r.type("2000")
            r.model.validate()
            runCurrent()

            assertEquals(CheckoutStep.PAY, r.state.step)
            assertEquals(r.errorTexts.describe("payment_invalid", AppLanguage.FR), r.state.error)
            assertNotNull(r.order.current.value)
        }

    @Test
    fun theErrorClearsWhenTheNextAttemptStarts() =
        runTest {
            val r = started(1_152)
            coEvery { r.f.engine.checkout(ORDER_ID, any(), any()) } throws PosException.Coded("payment_invalid", emptyMap())
            r.type("2000")
            r.model.validate()
            runCurrent()
            assertNotNull(r.state.error)

            r.f.engine.stubSale()
            r.model.validate()
            runCurrent()

            assertNull(r.state.error)
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun aWaiterCannotTakeAPayment() =
        runTest {
            val f = checkoutFixture(tmp.root, 1_152)
            f.engine.stubSale()
            val r = startCheckout(f, context, user = waiter)

            r.type("2000")
            assertFalse(r.state.canTakePayment)
            assertFalse(r.state.canValidate)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 0) { f.engine.checkout(any(), any(), any()) }
        }

    // ── Delivery ──────────────────────────────────────────────────

    @Test
    fun theDeliveryRequestOfTheStoreIsPassedToCheckoutAndResetAfterwards() =
        runTest {
            val r = started(1_152)
            r.model.chooseDeliveryEmail(RequestedMedium.PAPER_DIGITAL, "client@example.be")
            assertEquals(TicketDeliveryRequest(RequestedMedium.PAPER_DIGITAL, "client@example.be"), r.state.deliveryRequest)

            r.type("2000")
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(cash), TicketDeliveryRequest(RequestedMedium.PAPER_DIGITAL, "client@example.be"))
            }
            assertEquals(RequestedMedium.PAPER, r.delivery.request.value.medium)
            assertNull(r.delivery.request.value.email)
        }

    @Test
    fun anEmailOnlyChoiceCanGoBackToPaper() =
        runTest {
            val r = started(1_152)
            r.model.chooseDeliveryEmail(RequestedMedium.DIGITAL, "client@example.be")
            r.model.chooseDeliveryPaper()

            assertEquals(RequestedMedium.PAPER, r.state.deliveryRequest?.medium)
            r.type("2000")
            r.model.validate()
            runCurrent()
            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(cash), null) }
        }

    @Test
    fun theMissingOriginalIsPrintedThroughTheDeliveryStore() =
        runTest {
            val r = started(1_152)
            coEvery { r.f.engine.printMissingOriginal(99L) } just Runs

            assertTrue(r.model.printMissingOriginal(99L))
            coVerify(exactly = 1) { r.f.engine.printMissingOriginal(99L) }
        }

    // ── Done ──────────────────────────────────────────────────────

    @Test
    fun theDoneStateKeepsWhatTheSaleReturnedAndWarnsWhenTheSlipFailed() =
        runTest {
            val outcome =
                DeliveryOutcome(
                    fiscalEventId = 99L,
                    deliveryId = 3L,
                    medium = TicketMedium.DIGITAL,
                    email = EmailStatus.QUEUED,
                    recipientHint = "c***@example.be",
                    paperPrinted = false,
                    needsPaper = true,
                )
            val r = started(1_152)
            r.f.engine.stubSale(sale(fdmConnected = false, delivery = outcome, slip = SlipOutcome.FAILED))

            r.type("2000")
            r.model.validate()
            runCurrent()

            val done = requireNotNull(r.state.done)
            assertEquals(outcome, done.delivery)
            assertFalse(done.fdmConnected)
            assertEquals(99L, done.fiscalEventId)
            val toast = r.toasts.toasts.value.last()
            assertEquals(ToastVariant.Warning, toast.variant)
            assertEquals(r.catalog.text("kitchen_slip_failed", AppLanguage.FR), toast.message)
            assertEquals(emptyList<TakenPart>(), r.state.taken)
        }

    @Test
    fun noWarningWhenTheSlipPrintedOrThereWasNothingToPrint() =
        runTest {
            val r = started(1_152)
            r.f.engine.stubSale(sale(slip = SlipOutcome.PRINTED))

            r.type("2000")
            r.model.validate()
            runCurrent()

            assertTrue(r.toasts.toasts.value.none { it.variant == ToastVariant.Warning })
        }

    @Test
    fun aSecondTapWhileTheSaleIsBeingSignedDoesNothing() =
        runTest {
            val r = started(1_152)
            val gate = kotlinx.coroutines.CompletableDeferred<uniffi.pos_ffi.CheckoutResult>()
            coEvery { r.f.engine.checkout(ORDER_ID, any(), any()) } coAnswers { gate.await() }

            r.type("2000")
            r.model.validate()
            runCurrent()
            assertEquals(CheckoutStep.PROCESSING, r.state.step)
            assertFalse(r.state.canValidate)
            r.model.validate()
            runCurrent()
            gate.complete(sale())
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.checkout(any(), any(), any()) }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun backAndFinishAreMovesTheScreenMakes() =
        runTest {
            val r = started(1_152)

            r.model.back()
            runCurrent()
            assertEquals(listOf<CheckoutEffect>(CheckoutEffect.BackToOrder(ORDER_ID)), r.effects)

            r.type("2000")
            r.model.validate()
            runCurrent()
            r.model.finish()
            runCurrent()

            assertEquals(listOf(CheckoutEffect.BackToOrder(ORDER_ID), CheckoutEffect.NewOrder), r.effects)
            assertNull(r.state.done)
        }

    @Test
    fun theRecapListsTheLinesWithTheEnginesVatSplitAmounts() =
        runTest {
            val lines =
                listOf(
                    cartLine(1, quantity = 2, unitPriceCents = 500L),
                    cartLine(2, name = "Free", quantity = 3, unitPriceCents = 100L, vat = uniffi.pos_ffi.VatCode.X),
                )
            val r = started(1_300, lines)

            assertEquals(lines, r.state.summary.lines)
            assertEquals(
                listOf(
                    dev.nuagemagique.pilipos.ui.format.LineAmount.Total(1_000L),
                    dev.nuagemagique.pilipos.ui.format.LineAmount.PerUnit(3, 100L),
                ),
                r.state.summary.amounts,
            )
            assertEquals(1, r.state.summary.vatRows.size)
            assertEquals(5, r.state.itemCount)
        }
}
