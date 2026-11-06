package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
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

/** The "Plus" menu of the cart header (`CartMoreMenu.vue`, UX audit C3): which rows a role and an order get. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CartMoreMenuTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val callbacks =
        OrderCallbacks(
            onMoreTransfer = { seen += "transfer" },
            onMoreDiscount = { seen += "discount" },
            onMoreDiscountRemove = { seen += "discount-remove" },
            onMoreUnpaid = { seen += "unpaid" },
            onDismiss = { seen += "close" },
        )

    private fun show(
        menu: OrderDialog.CartMore,
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent { PiliTheme { ProvideUiText(language, catalog) { Box { CartMoreDialog(menu, callbacks) } } } }
        compose.waitForIdle()
    }

    private val manager = OrderDialog.CartMore(canTransfer = true, canDiscount = true, orderDiscounted = false, canCloseUnpaid = true)

    @Test
    fun aManagerHasChangeTableDiscountAndCustomerLeft() {
        show(manager)

        compose.onNodeWithTag("cart_transfer").assertTextEquals("Déplacer")
        compose.onNodeWithTag("cart_discount").assertTextEquals("Remise")
        compose.onNodeWithTag("cart_unpaid").assertTextEquals("Client parti sans payer")
        compose.onNodeWithTag("cart_discount_remove").assertDoesNotExist()
    }

    @Test
    fun everyRowIsAtLeast48DpHigh() {
        show(manager)

        listOf("cart_transfer", "cart_discount", "cart_unpaid", "cart_more_close").forEach {
            compose.onNodeWithTag(it).assertHeightIsAtLeast(48.dp)
        }
    }

    @Test
    fun aServerOnlyGetsChangeTable() {
        show(OrderDialog.CartMore(canTransfer = true, canDiscount = false, orderDiscounted = false, canCloseUnpaid = false))

        compose.onNodeWithTag("cart_transfer").assertIsDisplayed()
        compose.onNodeWithTag("cart_discount").assertDoesNotExist()
        compose.onNodeWithTag("cart_unpaid").assertDoesNotExist()
    }

    @Test
    fun anOrderDiscountSwitchesTheRowToRemoval() {
        show(manager.copy(orderDiscounted = true))

        compose.onNodeWithTag("cart_discount").assertDoesNotExist()
        compose.onNodeWithTag("cart_discount_remove").assertTextEquals("Retirer la remise")
        compose.onNodeWithTag("cart_discount_remove").performClick()

        assertEquals(listOf("discount-remove"), seen)
    }

    @Test
    fun eachRowCallsItsCallback() {
        show(manager)

        compose.onNodeWithTag("cart_transfer").performClick()
        compose.onNodeWithTag("cart_discount").performClick()
        compose.onNodeWithTag("cart_unpaid").performClick()

        assertEquals(listOf("transfer", "discount", "unpaid"), seen)
    }

    @Test
    fun closeLeavesWithoutDoingAnything() {
        show(manager)

        compose.onNodeWithTag("cart_more_close").performClick()

        assertEquals(listOf("close"), seen)
    }

    @Test
    fun theMenuSpeaksChineseInTheChineseUi() {
        show(manager, AppLanguage.ZH)

        compose.onNodeWithTag("cart_transfer").assertTextEquals("移动")
    }

    // ── State rules that open the menu and pick the primary (pure) ──

    @Test
    fun theStateRulesFollowTheVueUtilities() {
        val dineIn = order(table = 4, totalCents = 500)
        val unsent = CartRow(cartLine(1, 10, "A", unsent = 1), null)
        val sent = CartRow(cartLine(2, 11, "B"), null)
        val held = CartRow(cartLine(3, 12, "C", unsent = 1, kitchenState = "held"), null)

        assertEquals(1, OrderCartState(order = dineIn, rows = listOf(unsent, sent, held)).sendableLineCount)
        assertEquals(0, OrderCartState(order = order(table = null), rows = listOf(unsent)).sendableLineCount)
        assertEquals(true, OrderCartState(order = dineIn, rows = listOf(unsent), canPay = true).kitchenPrimary)
        assertEquals(false, OrderCartState(order = dineIn, rows = listOf(sent, held), canPay = true).kitchenPrimary)
        assertEquals(true, OrderCartState(order = dineIn, rows = listOf(sent), canPay = false).kitchenPrimary)
        assertEquals(false, OrderCartState(order = order(table = null), rows = listOf(unsent), canPay = false).kitchenPrimary)
        assertEquals(false, OrderCartState(order = dineIn).hasMoreActions)
        assertEquals(true, OrderCartState(order = dineIn, canCloseUnpaid = true).hasMoreActions)
    }
}
