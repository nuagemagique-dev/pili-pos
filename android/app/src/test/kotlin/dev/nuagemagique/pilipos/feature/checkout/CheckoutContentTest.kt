package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.TicketMedium
import uniffi.pos_ffi.VatCode

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("LargeClass", "TooManyFunctions")
class CheckoutContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        CheckoutCallbacks(
            onMethod = { calls += "method:$it" },
            onSplit = { calls += "split:$it" },
            onKey = { calls += "key:$it" },
            onExact = { calls += "exact" },
            onQuick = { calls += "quick:$it" },
            onClearParts = { calls += "clear" },
            onBack = { calls += "back" },
            onValidate = { calls += "validate" },
            onFinish = { calls += "finish" },
            onPaper = { calls += "paper" },
            onPrintMissing = {
                calls += "print:$it"
                true
            },
        )

    private val lines = listOf(cartLine(1, name = "Sushi", quantity = 2, unitPriceCents = 500L), cartLine(2, name = "Free", quantity = 3, unitPriceCents = 100L, vat = VatCode.X))

    private val base =
        CheckoutUiState(
            orderId = ORDER_ID,
            hasOrder = true,
            loadTried = true,
            orderType = OrderType.DINE_IN,
            tableNumber = 5,
            totalCents = 1_152L,
            canTakePayment = true,
            amounts =
                CheckoutAmounts(
                    restCents = 1_152L,
                    dueCents = 1_152L,
                    roundedDueCents = 1_150L,
                    restShownCents = 1_150L,
                    shares = listOf(1_152L),
                    quickAmountsCents = listOf(1_500L, 2_000L, 5_000L),
                ),
            summary =
                CheckoutSummary(
                    lines = lines,
                    amounts = listOf(LineAmount.Total(1_000L), LineAmount.PerUnit(3, 100L)),
                    vatRows = listOf(vatRow(VatCode.C, 6, 57L)),
                ),
            itemCount = 5,
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

    // ── Pay step ──────────────────────────────────────────────────

    @Test
    fun theCashPanelShowsTheEntryTheRoundedDueTheChangeAndWhatIsLeft() {
        show(base.copy(entry = AmountEntryState("2000")))

        text("checkout_entry").assertTextEquals("20,00")
        text("checkout_exact_amount").assertTextEquals("11,50 €")
        text("checkout_exact_rounded").assertTextContains("11,52", substring = true)
        text("checkout_change_value").assertTextEquals("8,50 €")
        text("checkout_total").assertTextEquals("11,52 €")
        text("checkout_rest").assertTextEquals("11,50 €")
        text("checkout_validate_amount").assertTextEquals(catalog.text("received_amount", AppLanguage.FR, mapOf("amount" to "20,00")))
        compose.onNodeWithTag("checkout_validate").assertIsEnabled()
    }

    @Test
    fun cashBelowTheRoundedDueShowsADashAndDisablesValidate() {
        show(base.copy(entry = AmountEntryState("1100")))

        text("checkout_change_value").assertTextEquals("-")
        compose.onNodeWithTag("checkout_validate").assertIsNotEnabled()
    }

    @Test
    fun theRoundedHintIsHiddenWhenTheDueNeedsNoRounding() {
        show(base.copy(amounts = base.amounts.copy(dueCents = 1_150L, roundedDueCents = 1_150L)))

        text("checkout_exact_rounded").assertDoesNotExist()
    }

    @Test
    fun theMethodButtonsReportTheTapAndCashIsDisabledOnceACashPartIsTaken() {
        show(base)
        compose.onNodeWithTag("checkout_method_card").performClick()
        compose.onNodeWithTag("checkout_method_meal_voucher").performClick()

        assertEquals(listOf("method:CARD", "method:MEAL_VOUCHER"), calls)
    }

    @Test
    fun cashCannotBePickedAgainAfterACashPart() {
        show(base.copy(method = PaymentMethod.CARD, taken = listOf(TakenPart(PaymentMethod.CASH, 500L, 0L))))

        compose.onNodeWithTag("checkout_method_cash").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_method_card").assertIsEnabled()
        text("checkout_cash_taken").assertIsDisplayed()
    }

    @Test
    fun theSplitButtonsAreDisabledWhenThePartsCouldNotHaveACent() {
        show(base.copy(totalCents = 2L))

        compose.onNodeWithTag("checkout_split_1").assertIsEnabled()
        compose.onNodeWithTag("checkout_split_2").assertIsEnabled()
        compose.onNodeWithTag("checkout_split_3").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_split_2").performClick()
        assertEquals(listOf("split:2"), calls)
    }

    @Test
    fun theSplitIsLockedOnceAPartIsTaken() {
        show(base.copy(splitCount = 2, taken = listOf(TakenPart(PaymentMethod.CARD, 500L, 0L))))

        compose.onNodeWithTag("checkout_split_2").assertIsEnabled()
        compose.onNodeWithTag("checkout_split_1").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_split_3").assertIsNotEnabled()
    }

    @Test
    fun thePartRowShowsPaidCurrentAndNextParts() {
        val chips =
            listOf(
                PartChip(ChipState.PAID, PaymentMethod.CARD, 2_011L),
                PartChip(ChipState.CURRENT, PaymentMethod.CASH, 2_010L),
                PartChip(ChipState.NEXT, null, 2_010L),
            )
        show(base.copy(splitCount = 3, taken = listOf(TakenPart(PaymentMethod.CARD, 2_011L, 0L)), amounts = base.amounts.copy(chips = chips)))

        compose.onNodeWithTag("checkout_chip_0").assertTextContains("Carte · 20,11 € ✓", substring = true)
        compose.onNodeWithTag("checkout_chip_1").assertTextContains("Espèces · 20,10 €", substring = true)
        compose.onNodeWithTag("checkout_chip_2").assertTextContains("20,10 €", substring = true)
        text("checkout_part_0").assertTextEquals("− 20,11 €")
    }

    @Test
    fun noPartRowForASinglePart() {
        show(base)

        compose.onNodeWithTag("checkout_chip_0").assertDoesNotExist()
    }

    @Test
    fun thePadAndTheShortcutsReportTheirTaps() {
        show(base)
        compose.onNodeWithTag("numpad_key_5").performClick()
        compose.onNodeWithTag("numpad_clear").performClick()
        compose.onNodeWithTag("numpad_back").performClick()
        compose.onNodeWithTag("checkout_exact").performClick()
        compose.onNodeWithTag("checkout_quick_2000").performClick()

        assertEquals(
            listOf("key:${NumPadKey.Digit('5')}", "key:${NumPadKey.Clear}", "key:${NumPadKey.Backspace}", "exact", "quick:2000"),
            calls,
        )
    }

    @Test
    fun theQuickAmountsAreOnlyOfferedForCash() {
        show(base)
        compose.onNodeWithTag("checkout_quick_1500").assertTextEquals("15,00 €")
        compose.onNodeWithTag("checkout_quick_5000").assertTextEquals("50,00 €")
    }

    @Test
    fun theActiveShortcutIsHighlightedByTheState() {
        show(base.copy(entry = AmountEntryState("1500"), fresh = true))

        assertEquals(true, base.copy(entry = AmountEntryState("1500"), fresh = true).quickActive(1_500L))
        compose.onNodeWithTag("checkout_quick_1500").assertIsDisplayed()
    }

    @Test
    fun cardShowsTheTerminalHintAndNoChangeBox() {
        show(base.copy(method = PaymentMethod.CARD, entry = AmountEntryState("1152"), fresh = true))

        text("checkout_exact_amount").assertTextEquals("11,52 €")
        text("checkout_change").assertDoesNotExist()
        text("checkout_entry").assertTextEquals("11,52")
        text("checkout_validate_amount").assertTextEquals("11,52 €")
        text("checkout_too_high").assertDoesNotExist()
    }

    @Test
    fun anAmountAboveWhatIsLeftIsFlaggedAndNotValidatable() {
        show(base.copy(method = PaymentMethod.MEAL_VOUCHER, entry = AmountEntryState("2000")))

        text("checkout_too_high").assertIsDisplayed()
        compose.onNodeWithTag("checkout_validate").assertIsNotEnabled()
    }

    @Test
    fun theTicketMediumRowShowsWhenEmailIsConfiguredAndNoPartIsTaken() {
        val mail = MailStatus(configured = true, reachable = true, allowDigitalOnly = true)
        show(base.copy(mailStatus = mail))

        compose.onNodeWithTag("checkout_ticket_row").assertIsDisplayed()
        compose.onNodeWithTag("ticket_medium_picker").assertIsDisplayed()
        compose.onNodeWithTag("medium_paper").performClick()
        assertEquals(listOf("paper"), calls)
    }

    @Test
    fun theTicketMediumRowIsHiddenWithoutEmailOrAfterAPart() {
        val mail = MailStatus(configured = true, reachable = true, allowDigitalOnly = true)
        show(base.copy(mailStatus = mail, taken = listOf(TakenPart(PaymentMethod.CARD, 500L, 0L)), method = PaymentMethod.CASH))
        compose.onNodeWithTag("checkout_ticket_row").assertDoesNotExist()
    }

    @Test
    fun theTicketMediumRowIsHiddenWhenEmailIsNotConfigured() {
        show(base.copy(mailStatus = mailOff))

        compose.onNodeWithTag("checkout_ticket_row").assertDoesNotExist()
    }

    // ── Recap ─────────────────────────────────────────────────────

    @Test
    fun theRecapShowsTheTitleTheCountTheLinesAndTheVat() {
        show(base)

        text("checkout_title").assertTextEquals("Commande · Table 5")
        text("checkout_item_count").assertTextEquals("5 art.")
        compose.onNodeWithTag("checkout_line_1").assertTextContains("10,00", substring = true)
        compose.onNodeWithTag("checkout_line_2").assertTextContains("3 × 1,00 €", substring = true)
    }

    @Test
    fun aTakeawayTitleNamesTheTakeaway() {
        show(base.copy(orderType = OrderType.TAKEAWAY, tableNumber = null))

        text("checkout_title").assertTextEquals("Commande · À emporter")
    }

    @Test
    fun theBackButtonLeavesWhenNoPartIsTakenAndTheClearButtonReplacesItAfterwards() {
        show(base)
        compose.onNodeWithTag("checkout_back").performClick()
        compose.onNodeWithTag("checkout_clear_parts").assertDoesNotExist()
        assertEquals(listOf("back"), calls)
    }

    @Test
    fun afterAPartTheClearButtonReplacesTheBackButton() {
        show(base.copy(taken = listOf(TakenPart(PaymentMethod.CARD, 500L, 0L))))

        compose.onNodeWithTag("checkout_back").assertDoesNotExist()
        compose.onNodeWithTag("checkout_clear_parts").performClick()
        assertEquals(listOf("clear"), calls)
    }

    @Test
    fun validateReportsTheTapAndNamesThePartInASplit() {
        show(base.copy(entry = AmountEntryState("2000"), splitCount = 2, taken = listOf(TakenPart(PaymentMethod.CARD, 500L, 0L))))

        compose.onNodeWithTag("checkout_validate").assertTextContains("Valider part 2", substring = true)
        compose.onNodeWithTag("checkout_validate").performClick()
        assertEquals(listOf("validate"), calls)
    }

    @Test
    fun theErrorOfTheLastAttemptIsShown() {
        show(base.copy(error = "Il reste un montant à payer."))

        text("checkout_error").assertTextEquals("Il reste un montant à payer.")
    }

    @Test
    fun aRoleThatMayNotTakePaymentsSeesAnExplanationAndADisabledValidate() {
        show(base.copy(canTakePayment = false, entry = AmountEntryState("2000")))

        text("checkout_no_rights").assertIsDisplayed()
        compose.onNodeWithTag("checkout_validate").assertIsNotEnabled()
    }

    // ── Other steps ───────────────────────────────────────────────

    @Test
    fun withoutAnOrderTheScreenShowsTheEmptyState() {
        show(base.copy(hasOrder = false))

        compose.onNodeWithTag("checkout_empty").assertIsDisplayed()
        compose.onNodeWithTag("checkout_pay").assertDoesNotExist()
    }

    @Test
    fun theProcessingStepShowsTheFdmMessage() {
        show(base.copy(step = CheckoutStep.PROCESSING))

        compose.onNodeWithTag("checkout_processing").assertIsDisplayed()
        compose.onNodeWithTag("checkout_pay").assertDoesNotExist()
    }

    @Test
    fun theDoneStepShowsTheTotalTheChangeTheFdmReferenceAndFinishes() {
        val done =
            CheckoutDone(
                ticketLabel = "Ticket",
                totalCents = 1_152L,
                changeCents = 850L,
                fdmId = "FDM123",
                fdmConnected = true,
                delivery = null,
                fiscalEventId = 99L,
            )
        show(base.copy(step = CheckoutStep.DONE, hasOrder = false, done = done))

        compose.onNodeWithTag("checkout_done").assertIsDisplayed()
        text("checkout_done_total").assertTextEquals("11,52 €")
        text("checkout_done_change").assertTextEquals("8,50 €")
        text("checkout_done_fdm").assertTextEquals("FDM : FDM123")
        text("checkout_receipt_label").assertTextEquals("Ticket")
        text("checkout_fdm_warning").assertDoesNotExist()
        compose.onNodeWithTag("checkout_finish").performClick()
        assertEquals(listOf("finish"), calls)
    }

    @Test
    fun theDoneStepWithoutChangeOrFdmWarnsAndOffersThePrintOfTheMissingOriginal() {
        val outcome =
            DeliveryOutcome(
                fiscalEventId = 99L,
                deliveryId = 3L,
                medium = TicketMedium.DIGITAL,
                email = EmailStatus.SENT,
                recipientHint = "c***@example.be",
                paperPrinted = false,
                needsPaper = true,
            )
        val done = CheckoutDone(null, 500L, 0L, null, fdmConnected = false, delivery = outcome, fiscalEventId = 99L)
        show(base.copy(step = CheckoutStep.DONE, hasOrder = false, done = done))

        text("checkout_done_change").assertDoesNotExist()
        text("checkout_done_fdm").assertDoesNotExist()
        text("checkout_fdm_warning").assertIsDisplayed()
        compose.onNodeWithTag("delivery_outcome").assertIsDisplayed()
        compose.onNodeWithTag("delivery_print_missing").performClick()
        compose.waitForIdle()
        assertEquals(listOf("print:99"), calls)
    }

    @Test
    fun aWaiterDoesNotSeeThePrintOfTheMissingOriginal() {
        val outcome = DeliveryOutcome(99L, 3L, TicketMedium.DIGITAL, EmailStatus.SENT, "c***", paperPrinted = false, needsPaper = true)
        val done = CheckoutDone(null, 500L, 0L, null, true, outcome, 99L)
        show(base.copy(step = CheckoutStep.DONE, hasOrder = false, canTakePayment = false, done = done))

        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
    }

    // ── Line notes ────────────────────────────────────────────────

    @Test
    fun lineNotesReadAJsonListOrAPlainStringAndShowOtherTextAsIs() {
        assertNull(lineNotes(null))
        assertNull(lineNotes(""))
        assertEquals("sans piment · bien cuit", lineNotes("[\"sans piment\",\"bien cuit\"]"))
        assertNull(lineNotes("[]"))
        assertEquals("extra", lineNotes("\"extra\""))
        assertNull(lineNotes("\"\""))
        assertEquals("not json", lineNotes("not json"))
        assertEquals("{\"a\":1}", lineNotes("{\"a\":1}"))
    }

    @Test
    fun aLineWithNotesShowsThem() {
        val noted = cartLine(1, name = "Sushi", quantity = 1, modifiersJson = "[\"sans piment\"]")
        show(base.copy(summary = base.summary.copy(lines = listOf(noted), amounts = listOf(LineAmount.Total(500L)))))

        compose.onNodeWithTag("checkout_line_1").assertTextContains("sans piment", substring = true)
    }
}
