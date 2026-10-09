package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
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
class OrderContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val callbacks =
        OrderCallbacks(
            onCategory = { seen += "category:$it" },
            onItem = { seen += "item:${it.id}" },
            onPrice = { seen += "price:$it" },
            onPicks = { seen += "picks:$it" },
            onDismiss = { seen += "dismiss" },
        )
    private val actions = OrderCartActions(onPay = { seen += "pay" })

    private val maki = menuItem(10, 1, "Maki saumon", 650, nameZh = "三文鱼卷", code = "M10")
    private val freePrice = menuItem(30, 1, "Supplément", 0, openPrice = true)

    private val menu =
        MenuState(
            categories = listOf(category(1, "Sushi", nameZh = "寿司", color = "#E53935"), category(2, "Boissons")),
            selectedCategoryId = 1,
            items = listOf(maki, freePrice),
            cartQuantities = mapOf(10L to 2),
            loaded = true,
        )

    private val cart =
        OrderCartState(
            order = order(totalCents = 1_300),
            rows =
                listOf(
                    CartRow(cartLine(1, 10, "Maki saumon", quantity = 2, nameZh = "三文鱼卷"), LineAmount.Total(1_300)),
                    CartRow(cartLine(2, 40, "Offert", quantity = 3, unitPriceCents = 0), LineAmount.PerUnit(3, 0)),
                ),
            canPay = true,
        )

    private val base = OrderUiState(signedIn = true, menu = menu, cart = cart)

    private fun show(
        state: OrderUiState,
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) { OrderContent(state, callbacks, actions) }
            }
        }
        compose.waitForIdle()
    }

    // ── Menu ──────────────────────────────────────────────────────

    @Test
    fun theCategoriesAndTheProductsOfTheSelectedCategoryAreShown() {
        show(base)

        compose.onNodeWithTag("order_categories").assertIsDisplayed()
        compose.onNodeWithTag("category_1").assertIsSelected()
        compose.onNodeWithTag("category_2").assertIsNotSelected()
        compose.onNodeWithTag("menu_item_10").assertTextContains("Maki saumon", substring = true)
        compose.onNodeWithTag("menu_item_10").assertTextContains("6,50", substring = true)
    }

    @Test
    fun theOtherLanguageIsShownUnderTheNameAndAFreePriceProductShowsItsBadge() {
        show(base)

        compose.onNodeWithTag("menu_item_10").assertTextContains("三文鱼卷", substring = true)
        compose.onNodeWithTag("category_1").assertTextContains("寿司", substring = true)
        compose.onNodeWithTag("menu_item_30").assertTextContains("Prix libre", substring = true)
    }

    @Test
    fun inChineseTheChineseNameLeads() {
        show(base.copy(menu = menu.copy(language = AppLanguage.ZH)), AppLanguage.ZH)

        compose.onNodeWithTag("menu_item_10").assertTextContains("三文鱼卷", substring = true)
    }

    @Test
    fun inCodeModeTheMenuCodeLeads() {
        show(base.copy(menu = menu.copy(labelMode = ProductLabelMode.CODE)))

        compose.onNodeWithTag("menu_item_10").assertTextContains("M10", substring = true)
    }

    @Test
    fun theQuantityAlreadyInTheCartIsABadgeOnTheProduct() {
        show(base)

        compose.onNodeWithTag("menu_item_10_qty", useUnmergedTree = true).assertTextEquals("2")
        compose.onNodeWithTag("menu_item_30_qty", useUnmergedTree = true).assertDoesNotExist()
    }

    @Test
    fun tappingACategoryAndAProductCallsBack() {
        show(base)

        compose.onNodeWithTag("category_2").performClick()
        compose.onNodeWithTag("menu_item_10").performClick()
        compose.onNodeWithTag("menu_item_30").performClick()

        assertEquals(listOf("category:2", "item:10", "item:30"), seen)
    }

    @Test
    fun aCategoryWithoutProductsSaysSoOnceTheProductsWereRead() {
        show(base.copy(menu = menu.copy(items = emptyList())))

        compose.onNodeWithTag("menu_empty").assertIsDisplayed()
    }

    @Test
    fun beforeTheFirstReadNoEmptyMessageFlashes() {
        show(base.copy(menu = menu.copy(items = emptyList(), loaded = false)))

        compose.onNodeWithTag("menu_empty").assertDoesNotExist()
    }

    @Test
    fun moreThanNineCategoriesWrapToASecondRow() {
        val many = (1L..12L).map { category(it, "Cat $it") }
        show(base.copy(menu = menu.copy(categories = many, selectedCategoryId = 1)))

        compose.onNodeWithTag("category_12").assertIsDisplayed()
    }

    // ── Z reminder and empty state ────────────────────────────────

    @Test
    fun theZReminderBannerIsShownOnlyWhenTheStateAsksForIt() {
        show(base.copy(zWarning = true))

        compose.onNodeWithTag("order_z_warning").assertTextContains("Rapport Z", substring = true)
    }

    @Test
    fun noBannerWithoutTheReminder() {
        show(base)

        compose.onNodeWithTag("order_z_warning").assertDoesNotExist()
    }

    @Test
    fun withoutAnOrderTheScreenShowsAnEmptyStateAndNoMenu() {
        show(base.copy(cart = OrderCartState()))

        compose.onNodeWithTag("order_empty").assertIsDisplayed()
        compose.onNodeWithTag("menu_grid").assertDoesNotExist()
        compose.onNodeWithTag("cart_column").assertDoesNotExist()
    }

    @Test
    fun withNobodyLoggedInTheScreenShowsTheEmptyState() {
        show(OrderUiState(signedIn = false))

        compose.onNodeWithTag("order_empty").assertIsDisplayed()
    }

    // ── Cart column ───────────────────────────────────────────────

    @Test
    fun theCartShowsTheLinesTheirEngineAmountsAndTheEngineTotal() {
        show(base)

        compose.onNodeWithTag("cart_title").assertTextEquals("Commande · Table 4")
        compose.onNodeWithTag("cart_line_1").assertTextContains("2×", substring = true)
        compose.onNodeWithTag("cart_line_1").assertTextContains("Maki saumon", substring = true)
        compose.onNodeWithTag("cart_line_1_amount", useUnmergedTree = true).assertTextEquals("13,00 €")
        compose.onNodeWithTag("cart_line_2_amount", useUnmergedTree = true).assertTextEquals("3 × 0,00 €")
        compose.onNodeWithTag("cart_total").assertTextEquals("13,00 €")
    }

    @Test
    fun aTakeawayTitleNamesTheTakeaway() {
        show(base.copy(cart = cart.copy(order = order(table = null, totalCents = 1_300))))

        compose.onNodeWithTag("cart_title").assertTextEquals("Commande · À emporter")
    }

    @Test
    fun anEmptyCartSaysSoAndItsPayButtonIsDisabled() {
        show(base.copy(cart = cart.copy(rows = emptyList(), order = order(totalCents = 0))))

        compose.onNodeWithTag("cart_empty").assertIsDisplayed()
        compose.onNodeWithTag("cart_total").assertTextEquals("0,00 €")
        compose.onNodeWithTag("cart_pay").assertIsNotEnabled()
    }

    @Test
    fun thePayButtonOpensTheCheckoutCallback() {
        show(base)

        compose.onNodeWithTag("cart_pay").assertIsEnabled()
        compose.onNodeWithTag("cart_pay").performClick()

        assertEquals(listOf("pay"), seen)
    }

    @Test
    fun withoutThePaymentRightThereIsNoPayButton() {
        show(base.copy(cart = cart.copy(canPay = false)))

        compose.onNodeWithTag("cart_pay").assertDoesNotExist()
        compose.onNodeWithTag("cart_total").assertIsDisplayed()
    }

    // ── Dialogs ───────────────────────────────────────────────────

    private fun showDialog(dialog: OrderDialog) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { Box { OrderDialogs(dialog, callbacks) } }
            }
        }
        compose.waitForIdle()
    }

    private val sauces =
        group(1, "Sauce", min = 1, max = 1, modifier(100, "Soja"), modifier(101, "Spicy", 50))
    private val extras =
        group(2, "Extras", min = 0, max = 2, modifier(102, "Gingembre", 30), modifier(103, "Wasabi"), modifier(104, "Sésame"))

    @Test
    fun aFreePriceProductOpensTheAmountDialogAndTheTypedPriceIsReturnedAsCents() {
        showDialog(OrderDialog.Price(freePrice))

        compose.onNodeWithTag("amount_dialog").assertIsDisplayed()
        compose.onNodeWithTag("amount_confirm").assertIsNotEnabled()
        compose.onNodeWithText("Supplément").assertIsDisplayed()
        compose.onNodeWithTag("numpad_key_1", useUnmergedTree = true).performClick()
        compose.onNodeWithTag("numpad_key_2", useUnmergedTree = true).performClick()
        compose.onNodeWithTag("numpad_key_5", useUnmergedTree = true).performClick()
        compose.onNodeWithTag("amount_value").assertTextEquals("1,25 €")
        compose.onNodeWithTag("amount_confirm").performClick()

        assertEquals(listOf("price:125"), seen)
    }

    @Test
    fun cancellingTheAmountDialogDismisses() {
        showDialog(OrderDialog.Price(freePrice))

        compose.onNodeWithTag("amount_cancel").performClick()

        assertEquals(listOf("dismiss"), seen)
    }

    @Test
    fun theOptionsDialogShowsTheItemPriceAndTheOptionPricesButNoTotal() {
        showDialog(OrderDialog.Modifiers(maki, listOf(sauces, extras)))

        compose.onNodeWithTag("modifier_dialog").assertIsDisplayed()
        compose.onNodeWithText("6,50 €").assertIsDisplayed()
        compose.onNodeWithTag("modifier_option_101").assertTextContains("+0,50 €", substring = true)
        compose.onNodeWithTag("modifier_option_100").assertTextEquals("Soja")
        compose.onNodeWithTag("modifier_confirm").assertTextEquals("Ajouter")
    }

    @Test
    fun theRuleOfEachGroupIsShown() {
        showDialog(OrderDialog.Modifiers(maki, listOf(sauces, extras)))

        compose.onNodeWithTag("modifier_group_1_rule").assertTextEquals("Choisir 1")
        compose.onNodeWithTag("modifier_group_2_rule").assertTextEquals("Jusqu'à 2")
    }

    @Test
    fun aRequiredGroupKeepsConfirmDisabledUntilAnOptionIsPicked() {
        showDialog(OrderDialog.Modifiers(maki, listOf(sauces, extras)))

        compose.onNodeWithTag("modifier_confirm").assertIsNotEnabled()
        compose.onNodeWithTag("modifier_option_100").performClick()

        compose.onNodeWithTag("modifier_option_100").assertIsSelected()
        compose.onNodeWithTag("modifier_confirm").assertIsEnabled()
    }

    @Test
    fun aSingleChoiceReplacesThePreviousPick() {
        showDialog(OrderDialog.Modifiers(maki, listOf(sauces)))

        compose.onNodeWithTag("modifier_option_100").performClick()
        compose.onNodeWithTag("modifier_option_101").performClick()

        compose.onNodeWithTag("modifier_option_100").assertIsNotSelected()
        compose.onNodeWithTag("modifier_option_101").assertIsSelected()
    }

    @Test
    fun aGroupStopsAtItsMaximum() {
        showDialog(OrderDialog.Modifiers(maki, listOf(extras)))

        compose.onNodeWithTag("modifier_option_102").performClick()
        compose.onNodeWithTag("modifier_option_103").performClick()
        compose.onNodeWithTag("modifier_option_104").performClick()

        compose.onNodeWithTag("modifier_option_104").assertIsNotSelected()
        compose.onNodeWithTag("modifier_option_102").performClick()
        compose.onNodeWithTag("modifier_option_102").assertIsNotSelected()
    }

    @Test
    fun confirmReturnsThePickedOptionIds() {
        showDialog(OrderDialog.Modifiers(maki, listOf(sauces, extras)))

        compose.onNodeWithTag("modifier_option_101").performClick()
        compose.onNodeWithTag("modifier_option_103").performClick()
        compose.onNodeWithTag("modifier_confirm").performClick()

        assertEquals(1, seen.size)
        assertEquals(setOf(101L, 103L), seen.single().removePrefix("picks:[").removeSuffix("]").split(", ").map { it.toLong() }.toSet())
    }

    @Test
    fun backDismissesWithoutPicks() {
        showDialog(OrderDialog.Modifiers(maki, listOf(sauces)))

        compose.onNodeWithTag("modifier_cancel").performClick()

        assertEquals(listOf("dismiss"), seen)
    }

    @Test
    fun noDialogDrawsNothing() {
        showDialog(OrderDialog.None)

        compose.onNodeWithTag("modifier_dialog").assertDoesNotExist()
        compose.onNodeWithTag("amount_dialog").assertDoesNotExist()
    }
}
