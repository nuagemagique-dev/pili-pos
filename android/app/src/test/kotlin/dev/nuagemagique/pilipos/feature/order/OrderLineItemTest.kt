package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Column
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
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

/** One cart line (`OrderLineItem.vue`): quantity steps, removal, names, notes and the engine amount. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class OrderLineItemTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val actions =
        OrderCartActions(
            onIncrement = { seen += "plus:$it" },
            onDecrement = { seen += "minus:$it" },
            onRemove = { seen += "remove:$it" },
        )

    private fun show(
        row: CartRow,
        pending: Boolean = false,
        language: AppLanguage = AppLanguage.FR,
        mode: ProductLabelMode = ProductLabelMode.NAME,
    ) {
        val state = OrderCartState(rows = listOf(row), language = language, labelMode = mode)
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) { Column { OrderLineItem(row, state, actions, pending) } }
            }
        }
        compose.waitForIdle()
    }

    private fun row(
        quantity: Int,
        amount: LineAmount? = LineAmount.Total(1_300),
        notes: String? = null,
        nameZh: String? = "三文鱼卷",
        code: String? = null,
    ) = CartRow(cartLine(1, 10, "Maki saumon", quantity = quantity, nameZh = nameZh, modifiersJson = notes, code = code), amount)

    @Test
    fun aWeighedLineShowsItsWeightAndPricePerKiloAndCannotBeStepped() {
        val line = cartLine(1, 10, "Bol malatang", unitPriceCents = 1_990).copy(weightGrams = 612)
        show(CartRow(line, LineAmount.Total(1_218)))

        compose.onNodeWithTag("cart_line_1_weight").assertTextEquals("0,612 kg")
        compose.onNodeWithTag("cart_line_1_per_kilo").assertTextEquals("19,90 €/kg")
        compose.onAllNodesWithTag("cart_line_1_plus").assertCountEquals(0)
        compose.onAllNodesWithTag("cart_line_1_minus").assertCountEquals(0)
    }

    @Test
    fun thePlusAddsOneThroughTheCallbackWithTheLineId() {
        show(row(2))

        compose.onNodeWithTag("cart_line_1_plus").performClick()

        assertEquals(listOf("plus:1"), seen)
    }

    @Test
    fun theMinusRemovesOneAboveOne() {
        show(row(2))

        compose.onNodeWithTag("cart_line_1_minus").assertIsEnabled()
        compose.onNodeWithTag("cart_line_1_minus").performClick()

        assertEquals(listOf("minus:1"), seen)
    }

    @Test
    fun theMinusStopsAtOneAndNothingIsSent() {
        show(row(1))

        compose.onNodeWithTag("cart_line_1_minus").assertIsNotEnabled()
        compose.onNodeWithTag("cart_line_1_minus").performClick()

        assertEquals(emptyList<String>(), seen)
        compose.onNodeWithTag("cart_line_1_qty").assertTextEquals("1")
    }

    @Test
    fun theDeleteControlRemovesTheLineAtAnyQuantity() {
        show(row(1))
        compose.onNodeWithTag("cart_line_1_remove").performClick()

        assertEquals(listOf("remove:1"), seen)
    }

    @Test
    fun theDeleteControlAlsoRemovesALineWithSeveralUnits() {
        show(row(5))
        compose.onNodeWithTag("cart_line_1_remove").performClick()

        assertEquals(listOf("remove:1"), seen)
    }

    @Test
    fun theLineShowsTheNameTheQuantityAndTheEngineAmount() {
        show(row(2))

        compose.onNodeWithTag("cart_line_1_name").assertTextEquals("Maki saumon")
        compose.onNodeWithTag("cart_line_1_sub").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_1_qty").assertTextEquals("2")
        compose.onNodeWithTag("cart_line_1_amount").assertTextEquals("13,00 €")
    }

    @Test
    fun aLineTheVatSplitLeavesOutShowsQuantityAndUnitPriceAsText() {
        show(row(3, amount = LineAmount.PerUnit(3, 250)))

        compose.onNodeWithTag("cart_line_1_amount").assertTextEquals("3 × 2,50 €")
    }

    @Test
    fun aLineWithoutAnAmountYetShowsNoAmount() {
        show(row(1, amount = null))

        compose.onNodeWithTag("cart_line_1_amount").assertTextEquals("")
    }

    @Test
    fun inChineseTheChineseNameLeads() {
        show(row(1), language = AppLanguage.ZH)

        compose.onNodeWithTag("cart_line_1_name").assertTextEquals("三文鱼卷")
    }

    @Test
    fun inCodeModeTheMenuCodeLeadsAndTheNameFollows() {
        show(row(1, code = "M10"), mode = ProductLabelMode.CODE)

        compose.onNodeWithTag("cart_line_1_name").assertTextEquals("M10")
        compose.onNodeWithTag("cart_line_1_sub").assertTextEquals("Maki saumon")
    }

    @Test
    fun theNotesOfTheOptionsAreShownJoinedByADot() {
        show(row(1, notes = """["Sans sésame","Sauce soja"]"""))

        compose.onNodeWithTag("cart_line_1_notes").assertTextEquals("Sans sésame · Sauce soja")
    }

    @Test
    fun aLineWithoutNotesShowsNoNotesLine() {
        show(row(1, notes = "[]"))

        compose.onNodeWithTag("cart_line_1").assertIsDisplayed()
        compose.onNodeWithTag("cart_line_1_notes").assertDoesNotExist()
    }

    @Test
    fun aPendingLineIsStillAFullLine() {
        show(row(2), pending = true)

        compose.onNodeWithTag("cart_line_1_name").assertTextContains("Maki saumon")
    }

    @Test
    fun notesAreAStringAListOfStringsOrNothing() {
        assertEquals("", lineNotes(null))
        assertEquals("", lineNotes(""))
        assertEquals("", lineNotes("[]"))
        assertEquals("Sans sésame", lineNotes("\"Sans sésame\""))
        assertEquals("a · c", lineNotes("[\"a\", 3, \"c\"]"))
        assertEquals("", lineNotes("{\"a\":1}"))
        assertEquals("", lineNotes("42"))
        assertEquals("pas du json {", lineNotes("pas du json {"))
    }
}
