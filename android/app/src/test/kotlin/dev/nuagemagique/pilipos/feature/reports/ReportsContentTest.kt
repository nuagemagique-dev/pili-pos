package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.CopyResult
import uniffi.pos_ffi.CopyableTicket
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.RefundableEvent
import uniffi.pos_ffi.TicketMedium
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

@Suppress("TooManyFunctions", "LargeClass")
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class ReportsContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        ReportsCallbacks(
            onTab = { calls += "tab:$it" },
            day =
                DayCallbacks(
                    onX = { calls += "x" },
                    onUserX = { calls += "userx" },
                    onZ = { calls += "z" },
                    onOpenZList = { calls += "zlist" },
                    onCloseZList = { calls += "zlist-close" },
                    onReprintZ = { calls += "reprint:$it" },
                ),
            cash =
                CashCallbacks(
                    onDirection = { calls += "dir:$it" },
                    onReason = { calls += "reason:$it" },
                    onAmount = { calls += "amount:$it" },
                    onOpenOtherAmount = { calls += "other-open" },
                    onCloseOtherAmount = { calls += "other-close" },
                    onPickOtherAmount = { calls += "other:$it" },
                    onOpenDeclaration = { calls += "declaration-open" },
                    onCloseDeclaration = { calls += "declaration-close" },
                    onDeclare = { calls += "declare:$it" },
                    onOpenDrawer = { calls += "drawer" },
                    onSubmit = { calls += "submit" },
                ),
            tickets =
                TicketsCallbacks(
                    onCopy = { calls += "copy:$it" },
                    onCopyConfirm = { calls += "copy-confirm" },
                    onCopyClose = { calls += "copy-close" },
                    onRefund = { calls += "refund:${it.fiscalEventId}" },
                    onToggleLine = { calls += "toggle:$it" },
                    onQuantity = { id, delta -> calls += "qty:$id:$delta" },
                    onRefundNext = { calls += "refund-next" },
                    onRefundBack = { calls += "refund-back" },
                    onRefundConfirm = { calls += "refund-confirm" },
                    onRefundClose = { calls += "refund-close" },
                    describeError = { "Refus: $it" },
                ),
            delivery = DeliveryCallbacks(onPaper = { calls += "paper" }),
        )

    private val base = ReportsState(signedIn = true)

    private fun show(
        state: ReportsState,
        delivery: ReportsDelivery = ReportsDelivery(),
        refund: RefundUi = RefundUi(),
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) {
                        ReportsContent(state, callbacks)
                        ReportsOverlay(state, delivery, refund, callbacks)
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    private fun day(change: DayState.() -> DayState) = base.copy(day = DayState().change())

    private fun cash(change: CashState.() -> CashState) = base.copy(tab = ReportsTab.CASH, cash = CashState().change())

    // ── Frame and tabs ────────────────────────────────────────────

    @Test
    fun withNoOperatorTheScreenShowsAnEmptyStateAndNoTab() {
        show(ReportsState(signedIn = false))

        compose.onNodeWithTag("reports_empty").assertIsDisplayed()
        compose.onNodeWithTag("reports_tab_day").assertDoesNotExist()
    }

    @Test
    fun theThreeTabsCarryTheVueCaptionsAndCallBack() {
        show(base)

        compose.onNodeWithTag("reports_tab_day").assertTextEquals("Journée")
        compose.onNodeWithTag("reports_tab_cash").assertTextEquals("Caisse")
        compose.onNodeWithTag("reports_tab_tickets").assertTextEquals("Tickets")
        compose.onNodeWithTag("reports_tab_cash").performClick()
        compose.onNodeWithTag("reports_tab_tickets").performClick()
        compose.onNodeWithTag("reports_tab_day").performClick()

        assertEquals(listOf("tab:CASH", "tab:TICKETS", "tab:DAY"), calls)
    }

    // ── Day tab ───────────────────────────────────────────────────

    @Test
    fun theDayTabShowsTheThreeReportCardsAndTheCloseButtonWithoutTheReminder() {
        show(day { this })

        compose.onNodeWithTag("report_x").assertIsDisplayed()
        compose.onNodeWithTag("report_user_x").assertIsDisplayed()
        compose.onNodeWithTag("report_z_history").assertIsDisplayed()
        compose.onNodeWithTag("report_z").assertTextContains("Clôturer la journée (Z)", substring = true)
        compose.onNodeWithTag("reports_z_notice").assertDoesNotExist()
        compose.onNodeWithTag("report_result").assertDoesNotExist()
    }

    @Test
    fun theReminderAppearsWhenAZIsDue() {
        show(day { copy(zPending = true) })

        compose.onNodeWithTag("reports_z_notice").assertTextEquals("Rapport Z pas encore fait aujourd'hui")
    }

    @Test
    fun theUnsettledCardPaymentsAreListedAboveTheCloseButton() {
        show(day { copy(unsettled = listOf(unsettled(1L, table = 4), unsettled(2L, amountCents = 950L, brand = null))) })

        compose.onNodeWithTag("report_z_unsettled").assertIsDisplayed()
        compose.onNodeWithText("Paiements par carte encaissés mais pas encore enregistrés dans une vente :").assertIsDisplayed()
        compose.onNodeWithTag("report_z_unsettled_1", useUnmergedTree = true).assertTextEquals("- Table 4 · 17,00 € · Bancontact")
        compose.onNodeWithTag("report_z_unsettled_2", useUnmergedTree = true).assertTextEquals("- À emporter · 9,50 €")
        // Only a warning: the Z button stays usable.
        compose.onNodeWithTag("report_z").assertIsEnabled()
    }

    @Test
    fun withEveryCardSettledThereIsNoWarning() {
        show(day { this })

        compose.onNodeWithTag("report_z_unsettled").assertDoesNotExist()
    }

    @Test
    fun theReprintCardNamesTheNewestZInBrusselsTime() {
        // 22:00 UTC on the 9th is midnight on the 10th in Brussels.
        show(day { copy(zHistory = listOf(zSummary(5L), zSummary(4L))) })

        compose.onNodeWithTag("report_z_history").assertTextContains("Dernier : Z #5 · 10/10/2026", substring = true)
    }

    @Test
    fun withoutAnyZTheReprintCardSaysThereIsNoHistory() {
        show(day { this })

        compose.onNodeWithTag("report_z_history").assertTextContains("Aucun", substring = true)
    }

    @Test
    fun theCardsAndTheCloseButtonCallBack() {
        show(day { this })

        compose.onNodeWithTag("report_x").performClick()
        compose.onNodeWithTag("report_user_x").performClick()
        compose.onNodeWithTag("report_z_history").performClick()
        compose.onNodeWithTag("report_z").performClick()

        assertEquals(listOf("x", "userx", "zlist", "z"), calls)
    }

    @Test
    fun whileAReportRunsItsCardSpinsAndBothReportButtonsAreDisabled() {
        show(day { copy(loadingKind = ReportKind.X) })

        compose.onNodeWithTag("report_spinner", useUnmergedTree = true).assertIsDisplayed()
        compose.onNodeWithTag("report_x").assertIsNotEnabled()
        compose.onNodeWithTag("report_user_x").assertIsNotEnabled()
        compose.onNodeWithTag("report_z").assertIsNotEnabled()
        compose.onNodeWithTag("report_z_history").assertIsEnabled()
    }

    @Test
    fun theFirstTapOnTheCloseButtonArmsItAndTheLabelAsksForConfirmation() {
        show(day { copy(zArmed = true) })

        compose.onNodeWithTag("report_z").assertTextContains("Confirmer la clôture ?", substring = true)
    }

    @Test
    fun aGeneratedReportIsAnnouncedWithItsLabel() {
        val outcome = ReportOutcome(ReportKind.Z, reportResult(reportNo = 7L, type = "Z"))
        show(day { copy(lastReport = outcome) })

        compose.onNodeWithTag("report_result").assertTextEquals("Rapport généré (Z #7)")
    }

    @Test
    fun aRefusedReportShowsTheTextOfTheRefusal() {
        val outcome = ReportOutcome(ReportKind.X, reportResult(success = false, error = "boom"), failure = "Refusé par le moteur")
        show(day { copy(lastReport = outcome) })

        compose.onNodeWithTag("report_result_failure").assertTextEquals("Refusé par le moteur")
        compose.onNodeWithTag("report_result").assertDoesNotExist()
    }

    @Test
    fun theZHistoryListsEveryZAndOnlyOffersReprintWhenThereIsAFiscalEvent() {
        show(day { copy(zListOpen = true, zHistory = listOf(zSummary(5L), zSummary(4L, fiscalEventId = null))) })

        compose.onNodeWithTag("z_label_5").assertTextEquals("Z #5 · 10/10/2026")
        compose.onNodeWithTag("z_reprint_5").assertIsDisplayed()
        compose.onNodeWithTag("z_row_4").assertIsDisplayed()
        compose.onNodeWithTag("z_reprint_4").assertDoesNotExist()

        compose.onNodeWithTag("z_reprint_5").performClick()
        compose.onNodeWithTag("z_list_close").performClick()

        assertEquals(listOf("reprint:5", "zlist-close"), calls)
    }

    @Test
    fun anEmptyZHistoryDialogSaysSo() {
        show(day { copy(zListOpen = true) })

        compose.onNodeWithTag("z_list_empty").assertIsDisplayed()
    }

    // ── Cash tab ──────────────────────────────────────────────────

    @Test
    fun theCashFormOffersTheReasonsOfTheChosenDirection() {
        show(cash { this })
        compose.onNodeWithTag("cash_reason_fond_de_caisse").assertTextEquals("Fond de caisse")
        compose.onNodeWithTag("cash_reason_pourboire").assertIsDisplayed()
        compose.onNodeWithTag("cash_reason_retrait").assertDoesNotExist()
    }

    @Test
    fun cashOutOffersWithdrawalInsteadOfTheFloat() {
        show(cash { copy(direction = CashDirection.OUT, reason = "retrait") })

        compose.onNodeWithTag("cash_reason_retrait").assertIsDisplayed()
        compose.onNodeWithTag("cash_reason_fond_de_caisse").assertDoesNotExist()
        compose.onNodeWithTag("cash_submit").assertTextContains("Enregistrer la sortie", substring = true)
    }

    @Test
    fun theDirectionReasonAndPresetButtonsCallBack() {
        show(cash { this })

        compose.onNodeWithTag("cash_dir_out").performClick()
        compose.onNodeWithTag("cash_dir_in").performClick()
        compose.onNodeWithTag("cash_reason_pourboire").performClick()
        compose.onNodeWithTag("cash_preset_2000").performClick()
        compose.onNodeWithTag("cash_other").performClick()

        assertEquals(listOf("dir:OUT", "dir:IN", "reason:pourboire", "amount:2000", "other-open"), calls)
    }

    @Test
    fun thePresetsAreShownInWholeEuros() {
        show(cash { this })

        compose.onNodeWithTag("cash_preset_1000").assertTextEquals("10 €")
        compose.onNodeWithTag("cash_preset_10000").assertTextEquals("100 €")
    }

    @Test
    fun theSummaryShowsDirectionReasonAndTheChosenAmount() {
        show(cash { copy(amountCents = 5_000L) })

        compose.onNodeWithTag("cash_summary_line").assertTextEquals("Entrée · Fond de caisse")
        compose.onNodeWithTag("cash_summary_amount").assertTextEquals("50,00 €")
    }

    @Test
    fun theRecordButtonIsDisabledUntilAnAmountIsChosen() {
        show(cash { this })

        compose.onNodeWithTag("cash_summary_amount").assertTextEquals("0,00 €")
        compose.onNodeWithTag("cash_submit").assertIsNotEnabled()
    }

    @Test
    fun theRecordButtonCallsBackOnceAnAmountIsChosen() {
        show(cash { copy(amountCents = 1_000L) })

        compose.onNodeWithTag("cash_submit").assertIsEnabled()
        compose.onNodeWithTag("cash_submit").performClick()

        assertEquals(listOf("submit"), calls)
    }

    @Test
    fun theDrawerButtonsCallBackAndTheCountIsDisabledWhileAMovementIsRecorded() {
        show(cash { this })

        compose.onNodeWithTag("cash_drawer").performClick()
        compose.onNodeWithTag("cash_declare").performClick()

        assertEquals(listOf("drawer", "declaration-open"), calls)
    }

    @Test
    fun whileAMovementIsRecordedTheDrawerCountAndTheRecordButtonAreDisabled() {
        show(cash { copy(amountCents = 1_000L, loading = true) })

        compose.onNodeWithTag("cash_declare").assertIsNotEnabled()
        compose.onNodeWithTag("cash_submit").assertIsNotEnabled()
    }

    @Test
    fun aRefusedMovementShowsTheTextOfTheRefusal() {
        show(cash { copy(lastOutcome = CashOutcome(false, "Tiroir indisponible")) })

        compose.onNodeWithTag("cash_failure").assertTextEquals("Tiroir indisponible")
    }

    @Test
    fun theOtherAmountKeypadTakesDigitsAsCentsAndConfirms() {
        show(cash { copy(otherAmountOpen = true) })

        compose.onNodeWithTag("amount_dialog").assertIsDisplayed()
        listOf("1", "2", "5", "0").forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.onNodeWithTag("amount_value").assertTextEquals("12,50 €")
        compose.onNodeWithTag("amount_confirm").performClick()

        assertEquals(listOf("other:1250"), calls)
    }

    @Test
    fun theOtherAmountKeypadRefusesZero() {
        show(cash { copy(otherAmountOpen = true) })

        compose.onNodeWithTag("amount_confirm").assertIsNotEnabled()
        compose.onNodeWithTag("amount_cancel").performClick()

        assertEquals(listOf("other-close"), calls)
    }

    @Test
    fun theDrawerCountKeypadAcceptsZeroForAnEmptyDrawer() {
        show(cash { copy(declarationOpen = true) })

        compose.onNodeWithTag("amount_confirm").assertIsEnabled()
        compose.onNodeWithTag("amount_confirm").performClick()

        assertEquals(listOf("declare:0"), calls)
    }

    @Test
    fun theDrawerCountKeypadCancelsWithoutRecording() {
        show(cash { copy(declarationOpen = true) })

        compose.onNodeWithTag("amount_cancel").performClick()

        assertEquals(listOf("declaration-close"), calls)
    }

    // ── Tickets tab ───────────────────────────────────────────────

    private fun tickets(
        copyables: List<CopyableTicket> = emptyList(),
        refundables: List<RefundableEvent> = emptyList(),
        loaded: Boolean = true,
    ) = base.copy(
        tab = ReportsTab.TICKETS,
        tickets =
            TicketsState(
                loaded = loaded,
                copyable = copyables,
                refundable = refundables,
                rows = mergeRows(copyables, refundables),
            ),
    )

    @Test
    fun theTicketsTabListsEveryTicketWithItsKindTableTimeAndAmount() {
        show(tickets(listOf(copyable(3L), copyable(2L, label = "T"), copyable(1L, label = "P")), listOf(refundable(3L))))

        compose.onNodeWithTag("tickets_title").assertTextEquals("Tickets récents")
        compose.onNodeWithTag("ticket_number_3").assertTextEquals("#3")
        compose.onNodeWithTag("ticket_kind_3").assertTextEquals("Vente")
        compose.onNodeWithTag("ticket_kind_2").assertTextEquals("Formation")
        compose.onNodeWithTag("ticket_kind_1").assertTextEquals("Addition")
        compose.onNodeWithTag("ticket_table_3").assertTextEquals("T4")
        // 12:00 UTC is 14:00 in Brussels.
        compose.onNodeWithTag("ticket_time_3").assertTextEquals("14:00")
        compose.onNodeWithTag("ticket_total_3").assertTextEquals("10,00 €")
    }

    @Test
    fun aRefundTicketIsNamedAndShownWithItsNegativeAmount() {
        show(tickets(listOf(copyable(5L, totalCents = -500L))))

        compose.onNodeWithTag("ticket_kind_5").assertTextEquals("Remboursement")
        compose.onNodeWithTag("ticket_total_5").assertTextEquals("-5,00 €")
    }

    @Test
    fun theCopyButtonIsOfferedForACopyableTicketAndTheRefundButtonForARefundableOne() {
        show(tickets(listOf(copyable(3L), copyable(2L)), listOf(refundable(3L), refundable(1L))))

        compose.onNodeWithTag("ticket_copy_3").assertIsDisplayed()
        compose.onNodeWithTag("ticket_refund_3").assertIsDisplayed()
        compose.onNodeWithTag("ticket_copy_2").assertIsDisplayed()
        compose.onNodeWithTag("ticket_refund_2").assertDoesNotExist()
        compose.onNodeWithTag("ticket_copy_1").assertDoesNotExist()
        compose.onNodeWithTag("ticket_refund_1").assertIsDisplayed()
    }

    @Test
    fun theCopyAndRefundButtonsCallBackWithTheirTicket() {
        show(tickets(listOf(copyable(3L)), listOf(refundable(3L))))

        compose.onNodeWithTag("ticket_copy_3").performClick()
        compose.onNodeWithTag("ticket_refund_3").performClick()

        assertEquals(listOf("copy:3", "refund:3"), calls)
    }

    @Test
    fun anEmptyListAfterTheLoadSaysThereIsNoTicket() {
        show(tickets())

        compose.onNodeWithTag("tickets_empty").assertTextEquals("Aucun ticket récent")
    }

    @Test
    fun beforeTheFirstLoadNoEmptyMessageFlashes() {
        show(tickets(loaded = false))

        compose.onNodeWithTag("tickets_empty").assertDoesNotExist()
    }

    // ── Copy flow ─────────────────────────────────────────────────

    private val mail = MailStatus(configured = true, reachable = true, allowDigitalOnly = true)

    private fun copying(
        step: CopyStep,
        kind: CopyTargetKind = CopyTargetKind.TICKET,
        result: CopyResult? = null,
    ) = base.copy(tab = ReportsTab.TICKETS, copyFlow = CopyFlow(7L, kind, step, result))

    private fun outcome(
        email: EmailStatus = EmailStatus.NOT_REQUESTED,
        needsPaper: Boolean = false,
        printed: Boolean = true,
    ) = DeliveryOutcome(8L, 1L, TicketMedium.PAPER, email, "a***@x.be", printed, needsPaper)

    @Test
    fun theCopyPickerOpensBeforeAnythingIsSignedAndOffersTheMediums() {
        show(copying(CopyStep.PICKER), ReportsDelivery(mail))

        compose.onNodeWithText("Copie de ticket").assertIsDisplayed()
        compose.onNodeWithTag("ticket_medium_picker").assertIsDisplayed()
        compose.onNodeWithTag("medium_paper").assertIsDisplayed()
        compose.onNodeWithTag("copy_confirm").performClick()
        compose.onNodeWithTag("copy_back").performClick()

        assertEquals(listOf("copy-confirm", "copy-close"), calls)
    }

    @Test
    fun theCopyOfAZIsNamedReprintAndPaperStaysTheSilentDefaultWithoutMail() {
        show(copying(CopyStep.PICKER, CopyTargetKind.Z), ReportsDelivery(mailStatus = null))

        compose.onNodeWithText("Réimprimer").assertIsDisplayed()
        compose.onNodeWithTag("ticket_medium_picker").assertDoesNotExist()
    }

    @Test
    fun theMediumButtonsCallTheDeliveryStore() {
        show(copying(CopyStep.PICKER), ReportsDelivery(mail))

        compose.onNodeWithTag("medium_paper").performClick()

        assertEquals(listOf("paper"), calls)
    }

    @Test
    fun whileTheCopyIsSignedOnlyTheProgressIsShown() {
        show(copying(CopyStep.PROCESSING), ReportsDelivery(mail))

        compose.onNodeWithTag("copy_processing").assertIsDisplayed()
        compose.onNodeWithTag("copy_confirm").assertDoesNotExist()
        compose.onNodeWithTag("copy_done").assertDoesNotExist()
    }

    @Test
    fun aSignedCopyShowsWhatHappenedToTheTicketAndFinishes() {
        val result = copyResult().also { it.delivery = outcome(EmailStatus.SENT) }
        show(copying(CopyStep.RESULT, result = result), ReportsDelivery(mail))

        compose.onNodeWithTag("delivery_message").assertTextContains("a***@x.be", substring = true)
        compose.onNodeWithTag("copy_error").assertDoesNotExist()
        compose.onNodeWithTag("copy_done").performClick()

        assertEquals(listOf("copy-close"), calls)
    }

    @Test
    fun aRefusedCopyShowsTheTextOfTheRefusal() {
        show(copying(CopyStep.RESULT, result = copyResult(success = false, error = "boom")), ReportsDelivery(mail))

        compose.onNodeWithTag("copy_error").assertTextEquals("Refus: boom")
    }

    @Test
    fun theOnePrintActionIsOfferedWhenThePaperOriginalIsMissingAndTheOperatorMayTakePayments() {
        val result = copyResult().also { it.delivery = outcome(needsPaper = true, printed = false) }
        show(copying(CopyStep.RESULT, result = result), ReportsDelivery(mail, canPrintMissing = true))

        compose.onNodeWithTag("delivery_print_missing").assertIsDisplayed()
    }

    @Test
    fun theOnePrintActionIsHiddenFromAnOperatorWhoMayNotTakePayments() {
        val result = copyResult().also { it.delivery = outcome(needsPaper = true, printed = false) }
        show(copying(CopyStep.RESULT, result = result), ReportsDelivery(mail, canPrintMissing = false))

        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
    }

    // ── Refund flow ───────────────────────────────────────────────

    private val lineOne = orderLine(1L, quantity = 3)
    private val lineTwo = orderLine(2L, quantity = 1)

    private fun refunding(
        step: RefundStep,
        checked: Map<Long, Int> = emptyMap(),
        result: CheckoutResult? = null,
    ) = base.copy(
        tab = ReportsTab.TICKETS,
        tickets = TicketsState(loaded = true, refundable = listOf(refundable(7L))),
        refundFlow = RefundFlow(step, refundable(7L), listOf(lineOne, lineTwo), checked, result),
    )

    private val amounts = mapOf<Long, LineAmount>(1L to LineAmount.Total(1_500L), 2L to LineAmount.Total(500L))

    private fun previewOf(
        quantity: Int = 2,
        amount: LineAmount = LineAmount.Total(1_000L),
        cash: Long? = 995L,
    ) = RefundPreview(
        lines = listOf(PreviewLine(RefundSelection(lineOne, quantity), amount)),
        vatRows = if (amount is LineAmount.Total) listOf(VatSplitRow(VatCode.B, 12L, 1_000L, 107L)) else emptyList(),
        totalCents = if (cash != null) 1_000L else null,
        cashCents = cash,
    )

    @Test
    fun theRefundLinesStepShowsEveryLineWithTheEnginesAmountAndTheOriginalQuantity() {
        show(refunding(RefundStep.CHOOSE_LINES), refund = RefundUi(amounts))

        compose.onNodeWithText("Articles à rembourser · #7").assertIsDisplayed()
        compose.onNodeWithTag("refund_orig_1", useUnmergedTree = true).assertTextEquals("3x")
        compose.onNodeWithTag("refund_amount_1", useUnmergedTree = true).assertTextEquals("15,00 €")
        compose.onNodeWithTag("refund_amount_2", useUnmergedTree = true).assertTextEquals("5,00 €")
        compose.onNodeWithTag("refund_next").assertIsNotEnabled()
        compose.onNodeWithTag("refund_qty_1", useUnmergedTree = true).assertDoesNotExist()
    }

    @Test
    fun tappingALineTicksItAndASelectedLineShowsItsStepper() {
        show(refunding(RefundStep.CHOOSE_LINES, checked = mapOf(1L to 2)), refund = RefundUi(amounts, previewOf()))

        compose.onNodeWithTag("refund_qty_1", useUnmergedTree = true).assertTextEquals("2")
        compose.onNodeWithTag("refund_minus_1").performClick()
        compose.onNodeWithTag("refund_plus_1").performClick()
        compose.onNodeWithTag("refund_line_2").performClick()

        assertEquals(listOf("qty:1:-1", "qty:1:1", "toggle:2"), calls)
    }

    @Test
    fun theNextButtonCarriesTheCashAmountTheEngineRoundedAndOpensTheDelivery() {
        show(refunding(RefundStep.CHOOSE_LINES, checked = mapOf(1L to 2)), refund = RefundUi(amounts, previewOf()))

        compose.onNodeWithTag("refund_next").assertTextContains("Rembourser 9,95 €", substring = true)
        compose.onNodeWithTag("refund_next").assertIsEnabled()
        compose.onNodeWithTag("refund_next").performClick()
        compose.onNodeWithTag("refund_close").performClick()

        assertEquals(listOf("refund-next", "refund-close"), calls)
    }

    @Test
    fun theVatSummaryOfTheSelectionIsShownUnderTheLines() {
        show(refunding(RefundStep.CHOOSE_LINES, checked = mapOf(1L to 2)), refund = RefundUi(amounts, previewOf()))

        compose.onNodeWithTag("refund_vat").assertTextContains("12%", substring = true)
    }

    @Test
    fun theConfirmationShowsTheSelectedLinesTheVatSummaryAndThePreviewTotal() {
        show(refunding(RefundStep.DELIVERY, mapOf(1L to 2)), ReportsDelivery(mail), RefundUi(amounts, previewOf()))

        compose.onNodeWithTag("refund_recap_1").assertTextContains("2 × item1", substring = true)
        compose.onNodeWithTag("refund_recap_1").assertTextContains("10,00 €", substring = true)
        compose.onNodeWithTag("refund_vat").assertTextContains("12%", substring = true)
        compose.onNodeWithTag("refund_total").assertTextEquals("9,95 €")
        compose.onNodeWithTag("refund_confirm").assertTextContains("Rembourser 9,95 €", substring = true)
        compose.onNodeWithTag("ticket_medium_picker").assertIsDisplayed()
        compose.onNodeWithTag("refund_total_unavailable").assertDoesNotExist()
    }

    @Test
    fun aSelectedLineWithVatCodeXHidesThePreviewTotal() {
        val preview = previewOf(amount = LineAmount.PerUnit(2, 500L), cash = null)
        show(refunding(RefundStep.DELIVERY, mapOf(1L to 2)), ReportsDelivery(mail), RefundUi(amounts, preview))

        compose.onNodeWithTag("refund_recap_1").assertTextContains("2 × 5,00 €", substring = true)
        compose.onNodeWithTag("refund_total").assertDoesNotExist()
        compose.onNodeWithTag("refund_total_unavailable").assertIsDisplayed()
        compose.onNodeWithTag("refund_confirm").assertTextEquals("Rembourser")
    }

    @Test
    fun theLinesStepWithAnXLineShowsNoTotalOnTheNextButtonEither() {
        val preview = previewOf(amount = LineAmount.PerUnit(2, 500L), cash = null)
        show(refunding(RefundStep.CHOOSE_LINES, mapOf(1L to 2)), refund = RefundUi(amounts, preview))

        compose.onNodeWithTag("refund_next").assertTextEquals("Rembourser")
    }

    @Test
    fun theConfirmButtonSignsAndTheBackButtonReturnsToTheLines() {
        show(refunding(RefundStep.DELIVERY, mapOf(1L to 2)), ReportsDelivery(mail), RefundUi(amounts, previewOf()))

        compose.onNodeWithTag("refund_confirm").performClick()
        compose.onNodeWithTag("refund_back").performClick()

        assertEquals(listOf("refund-confirm", "refund-back"), calls)
    }

    @Test
    fun whileTheRefundIsSignedOnlyTheProgressIsShown() {
        show(refunding(RefundStep.PROCESSING, mapOf(1L to 2)))

        compose.onNodeWithTag("refund_processing").assertIsDisplayed()
        compose.onNodeWithTag("refund_confirm").assertDoesNotExist()
    }

    @Test
    fun aSignedRefundShowsWhatHappenedToTheTicketAndFinishes() {
        val result = checkoutResult().also { it.delivery = outcome(EmailStatus.QUEUED) }
        show(refunding(RefundStep.RESULT, result = result), ReportsDelivery(mail))

        compose.onNodeWithTag("delivery_message").assertIsDisplayed()
        compose.onNodeWithTag("refund_error").assertDoesNotExist()
        compose.onNodeWithTag("refund_done").performClick()

        assertEquals(listOf("refund-close"), calls)
    }

    @Test
    fun aRefusedRefundShowsTheTextOfTheRefusal() {
        show(refunding(RefundStep.RESULT, result = checkoutResult(success = false, error = "boom")), ReportsDelivery(mail))

        compose.onNodeWithTag("refund_error").assertTextEquals("Refus: boom")
    }

    @Test
    fun theRefundCanStartFromTheListOfRefundableSales() {
        val state = refunding(RefundStep.CHOOSE_EVENT).copy(refundFlow = RefundFlow(RefundStep.CHOOSE_EVENT))
        show(state)

        compose.onNodeWithTag("refund_event_7").performClick()

        assertEquals(listOf("refund:7"), calls)
    }
}
