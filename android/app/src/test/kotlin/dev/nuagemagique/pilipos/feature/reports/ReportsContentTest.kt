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
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

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
        )

    private val base = ReportsState(signedIn = true)

    private fun show(state: ReportsState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) {
                        ReportsContent(state, callbacks)
                        ReportsOverlay(state, callbacks)
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
}
