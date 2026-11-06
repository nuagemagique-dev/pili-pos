package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.state.LineAdd
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/**
 * The dense cart (UX audit C2): one 56 dp row per line, the actions only under the selected line, the newest line
 * selected by default. The rules of the actions themselves are the earlier ones (the other cart tests).
 */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class DenseCartTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()
    private val actions =
        OrderCartActions(
            onIncrement = { seen += "plus:$it" },
            onDecrement = { seen += "minus:$it" },
            onRemove = { seen += "remove:$it" },
            onNote = { seen += "note:$it" },
        )

    private fun row(
        id: Long,
        bowlOf: Long? = null,
        unsent: Long = 0,
        quantity: Int = 1,
    ) = CartRow(
        cartLine(id, id, "Plat $id", quantity = quantity, unsent = unsent, bowlLineId = bowlOf),
        LineAmount.Total(1_200),
        canNote = true,
    )

    private fun stateOf(
        rows: List<CartRow>,
        lastAdd: LineAdd? = null,
    ) = OrderCartState(order = order(table = 4, totalCents = 1_200L * rows.size), rows = rows, lastAdd = lastAdd)

    private var state by mutableStateOf(stateOf(emptyList()))

    private fun show(
        initial: OrderCartState,
        height: Int = 900,
    ) {
        state = initial
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.height(height.dp)) { OrderCartColumn(state, actions) }
                }
            }
        }
        compose.waitForIdle()
    }

    private fun detailTags() = (1..20).filter { compose.onAllNodesWithTag("cart_line_${it}_detail").fetchSemanticsNodes().isNotEmpty() }

    @Test
    fun theNewestLineIsSelectedByDefault() {
        show(stateOf(listOf(row(1), row(2), row(3))))

        assertEquals(listOf(3), detailTags())
        compose.onNodeWithTag("cart_line_3_plus").assertIsDisplayed()
    }

    @Test
    fun aTapOnAnotherLineMovesTheSelectionAndSecondTapFoldsIt() {
        show(stateOf(listOf(row(1), row(2), row(3))))

        compose.onNodeWithTag("cart_line_1_row").performClick()
        assertEquals(listOf(1), detailTags())

        compose.onNodeWithTag("cart_line_1_row").performClick()
        assertEquals(emptyList<Int>(), detailTags())
    }

    @Test
    fun aFoldedCartStaysFoldedWhenTheLinesChangeQuantity() {
        show(stateOf(listOf(row(1), row(2))))
        compose.onNodeWithTag("cart_line_2_row").performClick()
        assertEquals(emptyList<Int>(), detailTags())

        state = stateOf(listOf(row(1), row(2, quantity = 3)))
        compose.waitForIdle()

        assertEquals(emptyList<Int>(), detailTags())
    }

    @Test
    fun anAddSelectsTheLineItWentTo() {
        show(stateOf(listOf(row(1), row(2), row(3))))
        compose.onNodeWithTag("cart_line_1_row").performClick()
        assertEquals(listOf(1), detailTags())

        state = stateOf(listOf(row(1), row(2), row(3)), lastAdd = LineAdd(seq = 1, lineId = 2, itemId = 2, units = 1))
        compose.waitForIdle()

        assertEquals(listOf(2), detailTags())
    }

    @Test
    fun aNewLineIsSelectedWhenItIsAdded() {
        show(stateOf(listOf(row(1), row(2))))
        compose.onNodeWithTag("cart_line_1_row").performClick()

        state = stateOf(listOf(row(1), row(2), row(3)), lastAdd = LineAdd(seq = 1, lineId = 3, itemId = 3, units = 1))
        compose.waitForIdle()

        assertEquals(listOf(3), detailTags())
    }

    @Test
    fun whenTheSelectedLineLeavesTheCartTheNewestOneIsSelected() {
        show(stateOf(listOf(row(1), row(2), row(3))))
        compose.onNodeWithTag("cart_line_2_row").performClick()
        assertEquals(listOf(2), detailTags())

        state = stateOf(listOf(row(1), row(3)))
        compose.waitForIdle()

        assertEquals(listOf(3), detailTags())
    }

    @Test
    fun aBowlsOptionIsNotSelectableAndTheDefaultSkipsIt() {
        show(stateOf(listOf(row(1), row(2), row(3, bowlOf = 2))))

        assertEquals(listOf(2), detailTags())
        compose.onNodeWithTag("cart_line_3_row").performClick()
        assertEquals(listOf(2), detailTags())
        compose.onAllNodesWithTag("cart_line_3_plus").assertCountEquals(0)
    }

    @Test
    fun theActionsOfTheSelectedLineCallBackWithItsId() {
        show(stateOf(listOf(row(1, quantity = 2), row(2))))
        compose.onNodeWithTag("cart_line_1_row").performClick()

        compose.onNodeWithTag("cart_line_1_plus").performClick()
        compose.onNodeWithTag("cart_line_1_minus").performClick()
        compose.onNodeWithTag("cart_line_1_note").performClick()
        compose.onNodeWithTag("cart_line_1_remove").performClick()

        assertEquals(listOf("plus:1", "minus:1", "note:1", "remove:1"), seen)
    }

    @Test
    fun everyActionIsATouchTargetOfAtLeast48Dp() {
        show(stateOf(listOf(row(1))))

        for (tag in listOf("plus", "minus", "note", "remove")) {
            compose.onNodeWithTag("cart_line_1_$tag").assertHeightIsAtLeast(48.dp)
        }
    }

    @Test
    fun theRowStaysOnOneLineWhenALongNoteIsOnIt() {
        val noted = cartLine(1, 1, "Plat 1", kitchenNote = "sans wasabi, bien cuit, sauce à part, sans sésame, sans oignon")
        show(stateOf(listOf(CartRow(noted, LineAmount.Total(1_200), canNote = true), row(2))))
        compose.onNodeWithTag("cart_line_2_row").performClick()

        compose.onNodeWithTag("cart_line_1_kitchen_note").assertIsDisplayed()
        compose.onNodeWithTag("cart_line_1_amount").assertTextEquals("12,00 €")
        val rowHeight = compose.onNodeWithTag("cart_line_1_row").fetchSemanticsNode().size.height
        // Mdpi: one pixel is one dp. A 56 dp row, not a taller one with a second line for the note.
        assertTrue("row height $rowHeight", rowHeight in 56..60)
    }

    @Test
    fun aRowIs56DpHighSoAboutTwelveLinesFitTheCart() {
        // The cart of the till at 1080 p is 670 dp tall: with every line folded, 11 rows show whole.
        state = stateOf((1L..14L).map { row(it) })
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { CartLines(state, actions, dineIn = true, modifier = Modifier.height(670.dp)) }
            }
        }
        compose.waitForIdle()
        compose.onNodeWithTag("cart_line_14_row").performClick() // the newest line was selected: fold it
        compose.onNodeWithTag("cart_line_1_row").performScrollTo()

        val heights = (1..11).map { compose.onNodeWithTag("cart_line_${it}_row").fetchSemanticsNode().size.height }
        assertTrue("heights $heights", heights.all { it in 56..60 })
        for (id in 1..11) compose.onNodeWithTag("cart_line_${id}_row").assertIsDisplayed()
    }
}
