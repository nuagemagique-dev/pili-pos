package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
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
@Suppress("TooManyFunctions")
class SoldOutContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val callbacks =
        OrderCallbacks(
            onDismiss = { seen += "dismiss" },
            onSoldOutMode = { seen += "mode" },
            onSoldOutList = { seen += "list" },
            onAvailability = { id, choice -> seen += "availability:$id:$choice" },
        )

    private val endOfDay = "2026-10-18T04:00:00+02:00"
    private val maki = menuItem(10, 1, "Maki saumon", 650)
    private val sold = menuItem(11, 1, "Nigiri thon", 480, soldOut = true, soldOutUntil = endOfDay)

    private val menu =
        MenuState(
            categories = listOf(category(1, "Sushi")),
            selectedCategoryId = 1,
            items = listOf(maki, sold),
            loaded = true,
            canSoldOut = true,
        )

    private val base = OrderUiState(signedIn = true, menu = menu, cart = OrderCartState(order = order()))

    private fun show(
        state: OrderUiState,
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) {
                    Box {
                        OrderContent(state, callbacks, OrderCartActions())
                        // The dialogs sit in the scaffold overlay in the app.
                        OrderDialogs(state.dialog, callbacks, state.menu.soldOut)
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theBarIsDrawnForACashierAndTheModeAndListButtonsCallBack() {
        show(base.copy(menu = menu.copy(soldOut = listOf(soldOutItem(11, "Nigiri thon", endOfDay)))))

        compose.onNodeWithTag("soldout_mode").assertTextContains("Épuisé")
        compose.onNodeWithTag("soldout_hint").assertDoesNotExist()
        compose.onNodeWithTag("soldout_list_button").assertTextContains("Produits épuisés (1)")
        compose.onNodeWithTag("soldout_mode").performClick()
        compose.onNodeWithTag("soldout_list_button").performClick()

        assertEquals(listOf("mode", "list"), seen)
    }

    @Test
    fun theModeShowsItsHintAndIsSelected() {
        show(base.copy(menu = menu.copy(soldOutMode = true)))

        compose.onNodeWithTag("soldout_mode").assertIsSelected()
        compose.onNodeWithTag("soldout_hint").assertTextContains("Touchez un produit", substring = true)
    }

    @Test
    fun noBarWithoutTheRight() {
        show(base.copy(menu = menu.copy(canSoldOut = false)))

        compose.onNodeWithTag("soldout_mode").assertDoesNotExist()
        compose.onNodeWithTag("soldout_list_button").assertDoesNotExist()
    }

    @Test
    fun aSoldOutProductHasItsBadgeAndTheOtherDoesNot() {
        show(base)

        compose.onNodeWithTag("menu_item_11_soldout", useUnmergedTree = true).assertTextEquals("Épuisé")
        compose.onNodeWithTag("menu_item_10_soldout", useUnmergedTree = true).assertDoesNotExist()
        // A sold-out product is still tappable: the model shows the refusal.
        compose.onNodeWithTag("menu_item_11").performClick()
        assertEquals(listOf<String>(), seen)
    }

    @Test
    fun theDialogOutlinesTheCurrentStateAndCallsBackWithTheChoice() {
        show(base.copy(dialog = OrderDialog.SoldOut(sold)))

        compose.onNodeWithTag("soldout_dialog").assertIsDisplayed()
        compose.onNodeWithTag("soldout_status").assertTextContains("Jusqu'à la fin de la journée (18/10 04:00)")
        compose.onNodeWithTag("soldout_choice_day").assertIsSelected()
        compose.onNodeWithTag("soldout_choice_until").performClick()
        compose.onNodeWithTag("soldout_choice_available").performClick()
        // The current state just closes.
        compose.onNodeWithTag("soldout_choice_day").performClick()
        compose.onNodeWithTag("soldout_cancel").performClick()

        assertEquals(
            listOf("availability:11:UNTIL", "availability:11:AVAILABLE", "dismiss", "dismiss"),
            seen,
        )
    }

    @Test
    fun theDialogOfAnAvailableProductHasNoStatusLine() {
        show(base.copy(dialog = OrderDialog.SoldOut(maki)))

        compose.onNodeWithTag("soldout_status").assertDoesNotExist()
        compose.onNodeWithTag("soldout_choice_available").assertIsSelected()
    }

    @Test
    fun theListShowsEachProductWithItsStatusAndReEnablesOne() {
        val list = listOf(soldOutItem(11, "Nigiri thon", endOfDay), soldOutItem(12, "Edamame"))
        show(base.copy(menu = menu.copy(soldOut = list), dialog = OrderDialog.SoldOutList))

        compose.onNodeWithTag("soldout_row_11").assertIsDisplayed()
        compose.onNode(hasText("Nigiri thon") and hasAnyAncestor(hasTestTag("soldout_row_11"))).assertIsDisplayed()
        compose.onNodeWithText("Jusqu'à la fin de la journée (18/10 04:00)").assertIsDisplayed()
        compose.onNodeWithTag("soldout_row_12").assertIsDisplayed()
        compose.onNodeWithText("Jusqu'à réactivation").assertIsDisplayed()
        compose.onNodeWithTag("soldout_reenable_11").performClick()
        compose.onNodeWithTag("soldout_list_close").performClick()

        assertEquals(listOf("availability:11:AVAILABLE", "dismiss"), seen)
    }

    @Test
    fun anEmptyListSaysSo() {
        show(base.copy(dialog = OrderDialog.SoldOutList))

        compose.onNodeWithTag("soldout_list_empty").assertTextEquals("Aucun produit épuisé.")
    }

    @Test
    fun theTextsFollowTheLanguage() {
        show(base.copy(dialog = OrderDialog.SoldOut(maki)), AppLanguage.ZH)

        compose.onNodeWithTag("soldout_choice_day").assertTextEquals("售罄至营业日结束")
        compose.onNodeWithTag("soldout_choice_available").assertTextEquals("有货")
    }
}
