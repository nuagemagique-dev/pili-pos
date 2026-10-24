package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
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
import uniffi.pos_ffi.CartPriceChange
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

/** The cart with a till discount: the buttons, the change rows under a line, the net amounts. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class DiscountCartTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val actions =
        OrderCartActions(
            onDiscount = { seen += "discount" },
            onRemoveOrderDiscount = { seen += "remove-order" },
            onOffer = { seen += "offer:$it" },
            onRemoveLineDiscount = { seen += "remove-line:$it" },
        )

    private fun show(state: OrderCartState) {
        compose.setContent { PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { Box { OrderCartColumn(state, actions) } } } }
        compose.waitForIdle()
    }

    private val plain =
        CartRow(cartLine(1, 10, "Maki saumon", quantity = 2, unitPriceCents = 1_200), LineAmount.Total(2_400), canOffer = true)

    private val halfOff =
        CartRow(
            cartLine(
                1,
                10,
                "Maki saumon",
                quantity = 2,
                unitPriceCents = 1_200,
                discountCents = -1_200,
                priceChanges = listOf(CartPriceChange("TILL-DISCOUNT", "Remise 50 %", "EVENT", -1_200)),
            ),
            LineAmount.Total(1_200),
        )

    private val offered =
        CartRow(
            cartLine(
                2,
                11,
                "Cola",
                unitPriceCents = 300,
                discountCents = -300,
                priceChanges = listOf(CartPriceChange("TILL-FREE-LINE", "Offert", "LINE", -300)),
            ),
            LineAmount.Total(0),
            canRemoveDiscount = true,
        )

    @Test
    fun aManagerSeesTheDiscountButtonAndTapsIt() {
        show(OrderCartState(order = order(table = 4, totalCents = 2_400), rows = listOf(plain), canDiscount = true))

        compose.onNodeWithTag("cart_discount").assertTextEquals("Remise")
        compose.onNodeWithTag("cart_discount").performClick()

        assertEquals(listOf("discount"), seen)
    }

    @Test
    fun withoutTheRightThereIsNoDiscountButton() {
        val noOffer = plain.copy(canOffer = false)
        show(OrderCartState(order = order(table = 4, totalCents = 2_400), rows = listOf(noOffer)))

        compose.onNodeWithTag("cart_discount").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_1_offer").assertDoesNotExist()
    }

    @Test
    fun anOrderDiscountSwitchesTheButtonToRemoval() {
        show(
            OrderCartState(
                order = order(table = 4, totalCents = 1_200),
                rows = listOf(halfOff),
                canDiscount = true,
                orderDiscounted = true,
            ),
        )

        compose.onNodeWithTag("cart_discount").assertDoesNotExist()
        compose.onNodeWithTag("cart_discount_remove").assertTextEquals("Retirer la remise")
        compose.onNodeWithTag("cart_discount_remove").performClick()

        assertEquals(listOf("remove-order"), seen)
    }

    @Test
    fun aDiscountedLineShowsItsChangeRowAndItsNetAmount() {
        show(
            OrderCartState(
                order = order(table = 4, totalCents = 1_200),
                rows = listOf(halfOff),
                canDiscount = true,
                orderDiscounted = true,
            ),
        )

        compose.onNodeWithTag("cart_line_1_amount").assertTextContains("12,00", substring = true)
        compose.onNodeWithTag("cart_line_1_change_0").assertIsDisplayed()
        compose.onNodeWithText("Remise 50 %").assertIsDisplayed()
        compose.onNodeWithText("-12,00").assertIsDisplayed()
        compose.onNodeWithTag("cart_total").assertTextContains("12,00", substring = true)
    }

    @Test
    fun aPlainLineOffersOffrirAndAnOfferedLineOffersRemoval() {
        show(OrderCartState(order = order(table = 4, totalCents = 2_400), rows = listOf(plain, offered), canDiscount = true))

        compose.onNodeWithTag("cart_line_1_offer").performClick()
        compose.onNodeWithTag("cart_line_2_remove_discount").performClick()
        compose.onNodeWithTag("cart_line_2_offer").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_2_amount").assertTextContains("0,00", substring = true)

        assertEquals(listOf("offer:1", "remove-line:2"), seen)
    }

    @Test
    fun theVatSummaryAndThePayButtonShowTheNetTotal() {
        show(
            OrderCartState(
                order = order(table = 4, totalCents = 1_200),
                rows = listOf(halfOff),
                canPay = true,
                canDiscount = true,
                orderDiscounted = true,
                vatRows = listOf(VatSplitRow(VatCode.C, 6, 1_200, 68)),
            ),
        )

        compose.onNodeWithTag("cart_pay_amount", useUnmergedTree = true).assertTextContains("12,00", substring = true)
        compose.onNodeWithTag("vat_summary").assertTextContains("6% 0,68", substring = true)
    }
}
