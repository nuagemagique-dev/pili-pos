package dev.nuagemagique.pilipos.feature.webshop

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
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
import uniffi.pos_ffi.Attention

@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class WebshopContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val tapped = mutableListOf<String>()

    private val callbacks =
        WebshopCallbacks(
            onPrimary = { tapped += "primary:${it.id}" },
            onReject = { tapped += "reject:${it.id}" },
            onResync = { tapped += "resync:${it.id}" },
            onCollectCard = { tapped += "card:${it.id}" },
            onDelay = { tapped += "delay:${it.id}" },
            onToggleReprint = { tapped += "reprint:${it.id}" },
            onPrintKitchen = { tapped += "kitchen:${it.id}" },
            onPrintCustomer = { tapped += "customer:${it.id}" },
            onToggleDone = { tapped += "done" },
            onConfirmReject = { tapped += "confirm-reject" },
            onConfirmRefund = { tapped += "confirm-refund" },
            onDelayMinutes = { tapped += "minutes:$it" },
            onDismissDialog = { tapped += "dismiss" },
        )

    private val review = webshopState(1, "pending", resolution = "review", attention = listOf(Attention.NoItems))

    private val orders =
        listOf(
            webshopOrder(1, createdAt = "2026-10-09T10:22:00Z", state = review),
            webshopOrder(
                2,
                hubrise = "new",
                source = "Webshop",
                service = "delivery",
                createdAt = "2026-10-09T10:28:00Z",
                customer = "Marie Dupont",
                notes = "Sonnez deux fois",
                address = listOf("Rue Neuve 5", null, "4000", "Liège"),
                items = listOf(item("Maki thon", "金枪鱼卷", 3, options = listOf("sans wasabi"), notes = "bien froid")),
                totalCents = 123_456,
            ),
            webshopOrder(3, hubrise = "accepted", fiscal = "signed_n", confirmed = "2026-10-09T10:45:00Z"),
            webshopOrder(
                4,
                hubrise = "awaiting_collection",
                fiscal = "signed_p",
                confirmed = "2026-10-09T10:40:00Z",
                totalCents = 1_850,
                state = webshopState(4, "signed_p", paymentMode = "unpaid"),
            ),
            webshopOrder(5, hubrise = "in_delivery", fiscal = "signed_n", source = "Webshop", service = "delivery"),
            webshopOrder(6, hubrise = "completed", fiscal = "signed_n"),
            webshopOrder(7, hubrise = "cancelled", fiscal = "refund_pending", totalCents = 2_450),
        )

    private val base =
        WebshopUiState(
            orders = orders,
            signedIn = true,
            isManager = true,
            hubriseVisible = true,
            operator = "Alice",
            now = NOW,
        )

    private fun show(
        state: WebshopUiState,
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) {
                    Box {
                        WebshopContent(state, callbacks)
                        WebshopDialogs(state, callbacks)
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    private fun text(tag: String) = compose.onNodeWithTag(tag)

    // ── Empty states ──────────────────────────────────────────────

    @Test
    fun withoutAUserTheScreenIsEmptyAndHasNoBoard() {
        show(WebshopUiState(signedIn = false, now = NOW))

        compose.onNodeWithTag("webshop_empty").assertIsDisplayed()
        compose.onNodeWithTag("webshop_board").assertDoesNotExist()
    }

    @Test
    fun whileHubriseIsOffTheScreenSaysSo() {
        show(base.copy(hubriseVisible = false))

        compose.onNodeWithTag("webshop_off").assertIsDisplayed()
        compose.onNodeWithTag("webshop_board").assertDoesNotExist()
    }

    @Test
    fun anEmptyBoardShowsTheEmptyLineInEachColumn() {
        show(base.copy(orders = emptyList()))

        listOf("new", "kitchen", "ready").forEach {
            compose.onNodeWithTag("webshop_col_${it}_empty").assertIsDisplayed()
            text("webshop_col_${it}_count").assertTextEquals("0")
        }
    }

    @Test
    fun loadingAndErrorLinesAreShown() {
        show(base.copy(orders = emptyList(), loading = true))
        text("webshop_message").assertTextEquals("Chargement…")
    }

    @Test
    fun aFailureIsShownAboveTheBoardAndKeepsTheOrders() {
        show(base.copy(error = "HubRise n'est pas connecté."))

        text("webshop_message").assertTextEquals("HubRise n'est pas connecté.")
        compose.onNodeWithTag("webshop_card_2").assertIsDisplayed()
    }

    // ── Columns ───────────────────────────────────────────────────

    @Test
    fun theCardsSitInTheirColumnsWithTheirCounts() {
        show(base)

        text("webshop_col_new_count").assertTextEquals("3")
        text("webshop_col_kitchen_count").assertTextEquals("1")
        text("webshop_col_ready_count").assertTextEquals("2")
        compose.onNodeWithTag("webshop_card_1").assertIsDisplayed()
        compose.onNodeWithTag("webshop_card_3").assertIsDisplayed()
        compose.onNodeWithTag("webshop_card_4").assertIsDisplayed()
        compose.onNodeWithTag("webshop_card_6").assertDoesNotExist()
    }

    @Test
    fun theDoneButtonCountsFinishedOrdersAndSwitchesTheThirdColumn() {
        show(base)
        text("webshop_done_toggle").assertTextContains("Terminées · 1", substring = true)
        compose.onNodeWithTag("webshop_done_toggle").performClick()
        assertEquals(listOf("done"), tapped)
    }

    @Test
    fun withTheDoneViewTheThirdColumnListsFinishedOrders() {
        show(base.copy(showDone = true))

        text("webshop_col_done_count").assertTextEquals("1")
        compose.onNodeWithTag("webshop_card_6").assertIsDisplayed()
        compose.onNodeWithTag("webshop_card_4").assertDoesNotExist()
        text("webshop_card_6_time").assertTextEquals("Terminée")
        compose.onNodeWithTag("webshop_card_6_primary").assertDoesNotExist()
    }

    // ── Card content ──────────────────────────────────────────────

    @Test
    fun theChannelIsATextChipAndTheOwnWebshopHasItsLabel() {
        show(base)

        text("webshop_card_1_channel").assertTextEquals("deliveroo")
        text("webshop_card_2_channel").assertTextEquals("Webshop")
    }

    @Test
    fun theTotalIsTheEnginesAmountThroughTheFormatter() {
        show(base)

        text("webshop_card_2_total").assertTextEquals("1 234,56 €")
        text("webshop_card_4_total").assertTextEquals("18,50 €")
    }

    @Test
    fun theKeyTimesFollowTheColumn() {
        show(base)

        text("webshop_card_1_time").assertTextEquals("il y a 8 min")
        text("webshop_card_3_time").assertTextEquals("prêt à 12:45")
        text("webshop_card_4_time").assertTextEquals("prête 12:40")
        text("webshop_card_5_time").assertTextEquals("En livraison")
    }

    @Test
    fun theMetaLineHasTheServiceTheCustomerAndThePaymentBadge() {
        show(base)

        text("webshop_card_2_meta").assertTextEquals("Livraison · Marie Dupont")
        text("webshop_card_4_pay").assertTextEquals("À encaisser")
        text("webshop_card_3_pay").assertTextEquals("Payée")
        compose.onNodeWithTag("webshop_card_1_pay").assertDoesNotExist()
    }

    @Test
    fun itemsShowTheirNameInTheUiLanguageWithOptionsAndNotes() {
        show(base)

        text("webshop_card_2_item_0").assertTextContains("3×", substring = true)
        text("webshop_card_2_item_0").assertTextContains("Maki thon", substring = true)
        text("webshop_card_2_item_0").assert(!hasText("金枪鱼卷", substring = true))
        text("webshop_card_2_item_0").assertTextContains("sans wasabi", substring = true)
        text("webshop_card_2_item_0").assertTextContains("“bien froid”", substring = true)
    }

    @Test
    fun theChineseInterfaceLeadsWithTheChineseName() {
        show(base, language = AppLanguage.ZH)

        text("webshop_card_2_item_0").assertTextContains("金枪鱼卷", substring = true)
        text("webshop_card_2_item_0").assert(!hasText("Maki thon", substring = true))
    }

    @Test
    fun theAddressIsShownForAnOwnDeliveryOnly() {
        show(base)

        text("webshop_card_2_address").assertTextEquals("Rue Neuve 5, 4000 Liège")
        compose.onNodeWithTag("webshop_card_5_address").assertDoesNotExist()
        compose.onNodeWithTag("webshop_card_1_address").assertDoesNotExist()
    }

    @Test
    fun anOrderToCheckListsItsReasonsAndOffersNoAcceptButton() {
        show(base)

        text("webshop_card_1_review").assertTextContains("À vérifier", substring = true)
        text("webshop_card_1_review").assertTextContains("Aucun article.", substring = true)
        compose.onNodeWithTag("webshop_card_1_primary").assertDoesNotExist()
        compose.onNodeWithTag("webshop_card_1_resync").assertIsDisplayed()
        compose.onNodeWithTag("webshop_card_1_reject").assertIsDisplayed()
    }

    @Test
    fun aCancelledSignedOrderShowsItsBannerAndTheRefundButtonToAManager() {
        show(base.copy(orders = listOf(orders.last())))

        text("webshop_card_7_banner").assertTextContains("remboursement à confirmer par un manager", substring = true)
        text("webshop_card_7_primary").assertTextEquals("Rembourser")
    }

    @Test
    fun withoutAManagerTheBannerAsksForOneAndTheRefundButtonIsGone() {
        show(base.copy(isManager = false, orders = listOf(orders.last())))

        text("webshop_card_7_banner").assertTextContains("Un manager doit se connecter", substring = true)
        compose.onNodeWithTag("webshop_card_7_primary").assertDoesNotExist()
    }

    @Test
    fun aLateOrderRaisesTheAlarmWithItsCount() {
        show(base)

        text("webshop_alarm").assertTextEquals("1 commande(s) en ligne attendent depuis plus de 5 minutes")
    }

    @Test
    fun noAlarmWhenNothingWaitsForFiveMinutes() {
        show(base.copy(orders = listOf(webshopOrder(2, createdAt = "2026-10-09T10:28:00Z"))))

        compose.onNodeWithTag("webshop_alarm").assertDoesNotExist()
    }

    // ── Buttons ───────────────────────────────────────────────────

    @Test
    fun thePrimaryCaptionFollowsTheStatus() {
        show(base)

        text("webshop_card_2_primary").assertTextEquals("Accepter")
        text("webshop_card_3_primary").assertTextEquals("Préparer")
        text("webshop_card_4_primary").assertTextEquals("Encaisser espèces")
        text("webshop_card_5_primary").assertTextEquals("Terminer")
    }

    @Test
    fun theButtonsCallTheirCallbacks() {
        show(base)

        compose.onNodeWithTag("webshop_card_2_primary").performClick()
        compose.onNodeWithTag("webshop_card_2_reject").performClick()
        compose.onNodeWithTag("webshop_card_1_resync").performClick()
        compose.onNodeWithTag("webshop_card_4_collect_card").performClick()
        compose.onNodeWithTag("webshop_card_3_delay").performClick()
        compose.onNodeWithTag("webshop_card_3_reprint").performClick()

        assertEquals(listOf("primary:2", "reject:2", "resync:1", "card:4", "delay:3", "reprint:3"), tapped)
    }

    @Test
    fun theReprintRowOffersTheTwoTicketsOnlyOnTheOpenCard() {
        show(base.copy(reprintOpenId = 3))

        compose.onNodeWithTag("webshop_card_4_print_kitchen").assertDoesNotExist()
        compose.onNodeWithTag("webshop_card_3_print_kitchen").performClick()
        compose.onNodeWithTag("webshop_card_3_print_customer").performClick()

        assertEquals(listOf("kitchen:3", "customer:3"), tapped)
    }

    @Test
    fun aBusyCardCannotBeTappedAgain() {
        show(base.copy(busy = setOf(2L)))

        compose.onNodeWithTag("webshop_card_2_primary").assertIsNotEnabled()
        compose.onNodeWithTag("webshop_card_2_reject").assertIsNotEnabled()
        compose.onNodeWithTag("webshop_card_3_primary").assertIsDisplayed()
    }

    // ── Dialogs ───────────────────────────────────────────────────

    @Test
    fun theRejectDialogAsksBeforeRefusing() {
        show(base.copy(dialog = WebshopDialog.RejectConfirm(2L)))

        compose.onNodeWithTag("pili_dialog").assertIsDisplayed()
        compose.onNodeWithTag("confirm_ok").performClick()
        compose.onNodeWithTag("confirm_cancel").performClick()

        assertEquals(listOf("confirm-reject", "dismiss"), tapped)
    }

    @Test
    fun theRefundDialogNamesTheAmountAndTheManager() {
        show(base.copy(dialog = WebshopDialog.RefundConfirm(7L)))

        compose.onNodeWithText("Rembourser 24,50 € ?", substring = true).assertIsDisplayed()
        compose.onNodeWithText("Alice", substring = true).assertIsDisplayed()
        compose.onNodeWithTag("confirm_ok").performClick()

        assertEquals(listOf("confirm-refund"), tapped)
    }

    @Test
    fun theDelayDialogOffersFifteenThirtyAndFortyFiveMinutes() {
        show(base.copy(dialog = WebshopDialog.Delay(3L)))

        text("webshop_delay_15").assertTextEquals("+15 min")
        compose.onNodeWithTag("webshop_delay_15").performClick()
        compose.onNodeWithTag("webshop_delay_30").performClick()
        compose.onNodeWithTag("webshop_delay_45").performClick()
        compose.onNodeWithTag("webshop_delay_cancel").performClick()

        assertEquals(listOf("minutes:15", "minutes:30", "minutes:45", "dismiss"), tapped)
    }

    @Test
    fun aTapOutsideTheDelayCardDismissesIt() {
        show(base.copy(dialog = WebshopDialog.Delay(3L)))

        compose.onNodeWithTag("pili_dialog_scrim").performTouchInput { click(Offset(4f, 4f)) }

        assertEquals(listOf("dismiss"), tapped)
    }

    @Test
    fun aDialogWhoseOrderIsGoneIsNotDrawn() {
        show(base.copy(dialog = WebshopDialog.RejectConfirm(99L)))

        compose.onNodeWithTag("pili_dialog").assertDoesNotExist()
    }
}
