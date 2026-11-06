package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.GiftVoucherPrint
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.TicketMedium
import uniffi.pos_ffi.VatCode

/**
 * The quick checkout of the UX audit (P1, P2, P6) as the operator sees it: what the primary button says, the row of
 * the rest after a part, and a done step that is left with a tap unless it holds something to read.
 */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class CheckoutQuickContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        CheckoutCallbacks(
            onMethod = { calls += "method:$it" },
            onValidate = { calls += "validate" },
            onFinish = { calls += "finish" },
        )

    private val base =
        CheckoutUiState(
            orderId = ORDER_ID,
            hasOrder = true,
            loadTried = true,
            orderType = OrderType.DINE_IN,
            tableNumber = 5,
            totalCents = 4_730L,
            canTakePayment = true,
            entry = AmountEntryState("4730"),
            fresh = true,
            amounts =
                CheckoutAmounts(
                    restCents = 4_730L,
                    dueCents = 4_730L,
                    roundedDueCents = 4_730L,
                    restShownCents = 4_730L,
                    shares = listOf(4_730L),
                    quickAmountsCents = listOf(5_000L),
                ),
            summary =
                CheckoutSummary(
                    lines = listOf(cartLine(1, quantity = 2)),
                    amounts = listOf(LineAmount.Total(1_000L)),
                    vatRows = listOf(vatRow(VatCode.C, 6, 57L)),
                ),
            itemCount = 2,
        )

    private fun show(state: CheckoutUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { CheckoutContent(state, callbacks) }
            }
        }
        compose.waitForIdle()
    }

    private fun text(tag: String) = compose.onNodeWithTag(tag, useUnmergedTree = true)

    private fun done(
        changeCents: Long = 0L,
        fdmConnected: Boolean = true,
        delivery: DeliveryOutcome? = null,
        prints: List<GiftVoucherPrint> = emptyList(),
    ) = CheckoutDone(
        ticketLabel = "Ticket",
        totalCents = 4_730L,
        changeCents = changeCents,
        fdmId = "FDM1",
        fdmConnected = fdmConnected,
        delivery = delivery,
        fiscalEventId = 99L,
        voucherPrints = prints,
    )

    private fun delivery(
        email: EmailStatus = EmailStatus.NOT_REQUESTED,
        paperPrinted: Boolean = true,
        needsPaper: Boolean = false,
    ) = DeliveryOutcome(99L, null, TicketMedium.PAPER, email, null, paperPrinted, needsPaper)

    // ── P1: the button says what it collects ──────────────────────

    @Test
    fun cashStartsEnteredAndTheButtonCollectsItInCash() {
        show(base)

        text("checkout_entry").assertTextEquals("47,30")
        text("checkout_validate_label").assertTextEquals("Encaisser 47,30 € en espèces")
        text("checkout_validate_amount").assertDoesNotExist()
        compose.onNodeWithTag("checkout_validate").assertIsEnabled()
        compose.onNodeWithTag("checkout_validate").performClick()
        assertEquals(listOf("validate"), calls)
    }

    @Test
    fun cashHandedOverAddsTheReceivedLineUnderTheLabel() {
        show(base.copy(entry = AmountEntryState("5000"), fresh = false))

        text("checkout_validate_label").assertTextEquals("Encaisser 47,30 € en espèces")
        text("checkout_validate_amount").assertTextEquals("reçu 50,00 €")
    }

    @Test
    fun theRoundingHintStaysOnTheExactButton() {
        val rounded = base.amounts.copy(dueCents = 4_732L, roundedDueCents = 4_730L)
        show(base.copy(amounts = rounded))

        text("checkout_exact_rounded").assertIsDisplayed()
        text("checkout_validate_label").assertTextEquals("Encaisser 47,30 € en espèces")
    }

    @Test
    fun cardMealVouchersAndGiftVouchersNameTheirMethod() {
        show(base.copy(method = PaymentMethod.CARD))
        text("checkout_validate_label").assertTextEquals("Encaisser 47,30 € par carte")
    }

    @Test
    fun mealVouchersAreNamedInTheLabel() {
        show(base.copy(method = PaymentMethod.MEAL_VOUCHER))
        text("checkout_validate_label").assertTextEquals("Encaisser 47,30 € en chèques-repas")
    }

    @Test
    fun aPartNamesItsNumberAndAmount() {
        val split = base.amounts.copy(dueCents = 1_577L, roundedDueCents = 1_575L)
        val taken = listOf(TakenPart(PaymentMethod.CARD, 1_577L, 0L))
        show(base.copy(splitCount = 3, taken = taken, amounts = split, entry = AmountEntryState("1575")))

        text("checkout_validate_label").assertTextEquals("Encaisser la part 2 · 15,75 €")
    }

    // ── P6: the rest row ──────────────────────────────────────────

    @Test
    fun noRestRowBeforeAPart() {
        show(base)

        text("checkout_rest_label").assertDoesNotExist()
    }

    @Test
    fun afterAPartOneRowNamesTheRestAndOffersTheMethodsStillAllowed() {
        val taken = listOf(TakenPart(PaymentMethod.MEAL_VOUCHER, 3_500L, 0L))
        val rest = base.amounts.copy(restCents = 1_230L, dueCents = 1_230L, roundedDueCents = 1_230L, restShownCents = 1_230L)
        show(base.copy(taken = taken, amounts = rest, entry = AmountEntryState("1230")))

        text("checkout_rest_label").assertTextEquals("Reste 12,30 € :")
        compose.onNodeWithTag("checkout_method_cash").assertIsEnabled()
        compose.onNodeWithTag("checkout_method_card").performClick()
        assertEquals(listOf("method:CARD"), calls)
        text("checkout_validate_label").assertTextEquals("Encaisser 12,30 € en espèces")
    }

    @Test
    fun theRestRowDoesNotOfferCashAfterACashPart() {
        val taken = listOf(TakenPart(PaymentMethod.CASH, 2_000L, 0L))
        show(base.copy(method = PaymentMethod.CARD, taken = taken, amounts = base.amounts.copy(restCents = 2_730L)))

        text("checkout_rest_label").assertIsDisplayed()
        compose.onNodeWithTag("checkout_method_cash").assertDoesNotExist()
    }

    // ── P2: the done step ─────────────────────────────────────────

    @Test
    fun theDoneStepShowsOkAndTheChangeVeryLarge() {
        show(base.copy(step = CheckoutStep.DONE, hasOrder = false, done = done(changeCents = 270L)))

        text("checkout_done_ok").assertTextEquals("OK")
        text("checkout_done_change_banner").assertIsDisplayed()
        text("checkout_done_change").assertTextEquals("2,70 €")
        text("checkout_done_auto_return").assertIsDisplayed()
    }

    @Test
    fun noChangeBannerWithoutChange() {
        show(base.copy(step = CheckoutStep.DONE, hasOrder = false, done = done()))

        text("checkout_done_change_banner").assertDoesNotExist()
    }

    @Test
    fun aTapAnywhereLeavesAPlainDoneStep() {
        show(base.copy(step = CheckoutStep.DONE, hasOrder = false, done = done(changeCents = 270L)))

        compose.onNodeWithTag("checkout_done").performTouchInput { click(topLeft) }
        assertEquals(listOf("finish"), calls)
    }

    private fun assertHeld(state: CheckoutDone) {
        show(base.copy(step = CheckoutStep.DONE, hasOrder = false, done = state))

        text("checkout_done_auto_return").assertDoesNotExist()
        // A tap on the screen does not leave; only "Terminer" does.
        compose.onNodeWithTag("checkout_done").performTouchInput { click(topLeft) }
        assertEquals(emptyList<String>(), calls)
        compose.onNodeWithTag("checkout_finish").performClick()
        assertEquals(listOf("finish"), calls)
    }

    @Test
    fun aTicketThatCouldNotBePrintedHoldsTheDoneStep() = assertHeld(done(delivery = delivery(paperPrinted = false, needsPaper = true)))

    @Test
    fun aDigitalTicketInTheOutboxHoldsTheDoneStep() =
        assertHeld(done(delivery = delivery(email = EmailStatus.QUEUED, paperPrinted = false)))

    @Test
    fun anUnsignedTicketHoldsTheDoneStep() = assertHeld(done(fdmConnected = false))

    @Test
    fun aGiftVoucherSlipNoteHoldsTheDoneStep() = assertHeld(done(prints = listOf(GiftVoucherPrint(3L, "voucher", SlipOutcome.FAILED))))
}
