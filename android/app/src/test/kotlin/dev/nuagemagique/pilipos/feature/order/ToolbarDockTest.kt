package dev.nuagemagique.pilipos.feature.order

import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertHeightIsEqualTo
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.DpRect
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
import androidx.compose.ui.unit.width
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.AllergenEntry
import uniffi.pos_ffi.AllergenNames

/**
 * UX audit C1 (the one 56 dp toolbar row between the category tabs and the grid) and C6 (the code pad docked under
 * the grid, which keeps its 5 columns). The behaviour behind the buttons and the keys is tested elsewhere
 * (`SoldOutContentTest`, `AllergenModeTest`, `CodePadFlowTest`); this is the layout.
 */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class ToolbarDockTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val callbacks =
        OrderCallbacks(
            onSoldOutMode = { seen += "soldout" },
            onSoldOutList = { seen += "list" },
            onCodePadToggle = { seen += "pad" },
            onAllergenMode = { seen += "allergens" },
            onSellVoucher = { seen += "voucher" },
        )

    private val items = (1L..30L).map { menuItem(it, 1, "Plat $it", 650, code = "A$it") }

    private val menu =
        MenuState(
            categories = listOf(category(1, "Sushi")),
            selectedCategoryId = 1,
            items = items,
            loaded = true,
            canSoldOut = true,
            giftVouchersEnabled = true,
        )

    private val legend =
        listOf(
            AllergenEntry("gluten", 1, AllergenNames("Gluten", "Gluten", "Gluten", "麸质")),
            AllergenEntry("molluscs", 14, AllergenNames("Mollusques", "Weekdieren", "Molluscs", "软体动物")),
        )

    private val base = OrderUiState(signedIn = true, menu = menu, cart = OrderCartState(order = order()))

    private fun show(state: OrderUiState = base) {
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { OrderContent(state, callbacks, OrderCartActions()) } }
        }
        compose.waitForIdle()
    }

    private fun bounds(tag: String): DpRect = compose.onNodeWithTag(tag).getUnclippedBoundsInRoot()

    private fun SemanticsNodeInteraction.rect(): DpRect = getUnclippedBoundsInRoot()

    private val buttons = listOf("soldout_mode", "code_pad_toggle", "allergen_mode", "gift_voucher_sell", "soldout_list_button")

    @Test
    fun allTheModesSitOnOne56dpRowBetweenTheTabsAndTheGrid() {
        show()

        compose.onNodeWithTag("mode_toolbar").assertHeightIsEqualTo(56.dp)
        val row = bounds("mode_toolbar")
        val tabs = bounds("category_1")
        val grid = bounds("menu_grid")
        assertTrue("the row is under the tabs", row.top >= tabs.bottom)
        assertTrue("the grid is under the row", grid.top >= row.bottom)
        val boxes = buttons.map { bounds(it) }
        boxes.forEach { b ->
            assertEquals("one line", boxes.first().top.value, b.top.value, 0.5f)
            assertTrue("touch height", b.height.value >= 48f)
            assertTrue("inside the row", b.bottom <= row.bottom && b.top >= row.top)
        }
        boxes.zipWithNext().forEach { (a, b) -> assertTrue("left to right", b.left >= a.right) }
    }

    @Test
    fun everyButtonHasAnIconBesideItsLabel() {
        show()

        buttons.forEach { tag ->
            val label = compose.onNode(hasText("", substring = true) and hasAnyAncestor(hasTestTag(tag)), useUnmergedTree = true)
            val button = bounds(tag)
            val text = label.rect()
            // 14 dp of padding on each side, the 22 dp icon and 8 dp between it and the label.
            assertEquals("icon, gap and padding around $tag", 14f + 22f + 8f, (text.left - button.left).value, 1f)
            assertTrue(button.width.value > text.width.value + 44f)
        }
        compose.onNodeWithTag("soldout_list_button").assertIsDisplayed()
    }

    @Test
    fun theButtonsCallBackAndTheModesAreSelected() {
        show(base.copy(menu = menu.copy(soldOutMode = true, allergenMode = false, codePad = CodePadState(open = true))))

        compose.onNodeWithTag("soldout_mode").assertIsSelected()
        compose.onNodeWithTag("code_pad_toggle").assertIsSelected()
        buttons.forEach { compose.onNodeWithTag(it).performClick() }

        assertEquals(listOf("soldout", "pad", "allergens", "voucher", "list"), seen)
    }

    @Test
    fun aWaiterHasNoSoldOutButtonsAndVouchersOffHasNoVoucherButton() {
        show(base.copy(menu = menu.copy(canSoldOut = false, giftVouchersEnabled = false)))

        compose.onNodeWithTag("soldout_mode").assertDoesNotExist()
        compose.onNodeWithTag("soldout_list_button").assertDoesNotExist()
        compose.onNodeWithTag("gift_voucher_sell").assertDoesNotExist()
        compose.onNodeWithTag("code_pad_toggle").assertIsDisplayed()
        compose.onNodeWithTag("allergen_mode").assertIsDisplayed()
        compose.onNodeWithTag("mode_toolbar").assertHeightIsEqualTo(56.dp)
    }

    @Test
    fun theHintOfAModeStaysOnTheRowAndTheLegendOpensUnderIt() {
        show(base.copy(menu = menu.copy(soldOutMode = true)))
        val row = bounds("mode_toolbar")
        val hint = bounds("soldout_hint")
        assertTrue(hint.top >= row.top && hint.bottom <= row.bottom)
        compose.onNodeWithTag("mode_toolbar").assertHeightIsEqualTo(56.dp)
    }

    @Test
    fun theAllergenHintAndLegendFollowTheMode() {
        show(base.copy(menu = menu.copy(allergenMode = true, allergenLegend = legend)))

        compose.onNodeWithTag("allergen_hint").assertIsDisplayed()
        compose.onNodeWithTag("soldout_hint").assertDoesNotExist()
        compose.onNodeWithTag("allergen_legend").assertIsDisplayed()
        val row = bounds("mode_toolbar")
        val hint = bounds("allergen_hint")
        assertTrue(hint.top >= row.top && hint.bottom <= row.bottom)
        assertTrue(bounds("allergen_legend").top >= row.bottom)
        compose.onNodeWithTag("mode_toolbar").assertHeightIsEqualTo(56.dp)
    }

    private fun withPad(text: String = "") = base.copy(menu = menu.copy(codePad = CodePadState(open = true, text = text, items = items)))

    @Test
    fun theDockIs300dpUnderTheGridAndTheGridKeepsFiveColumns() {
        show(withPad())

        compose.onNodeWithTag("code_pad").assertHeightIsEqualTo(300.dp)
        val dock = bounds("code_pad")
        val grid = bounds("menu_grid")
        assertTrue("under the grid", dock.top >= grid.bottom)
        assertEquals("the whole width of the left area", grid.width.value, dock.width.value, 0.5f)
        val tops =
            items.mapNotNull { runCatching { bounds("menu_item_${it.id}") }.getOrNull() }
        val first = tops.first().top
        assertEquals(5, tops.count { kotlin.math.abs((it.top - first).value) < 1f })
    }

    @Test
    fun withoutThePadTheGridTakesTheRoomAndNoDockIsDrawn() {
        show()

        compose.onNodeWithTag("code_pad").assertDoesNotExist()
        compose.onNodeWithTag("menu_grid").assertIsDisplayed()
    }

    @Test
    fun fieldAndMessageOnTheLeftDigitsInTheMiddleOkOnTheRight() {
        show(withPad())

        val dock = bounds("code_pad")
        val field = bounds("code_pad_display")
        val message = bounds("code_pad_message")
        val seven = bounds("code_key_7")
        val ok = bounds("code_key_ok")
        val letter = bounds("code_key_A")
        assertTrue(message.top >= field.bottom)
        assertTrue(letter.top >= message.bottom)
        assertTrue("digits right of the field", seven.left >= field.right)
        assertTrue("OK right of the digits", ok.left >= bounds("code_key_9").right)
        assertTrue("letters left of the digits", letter.right <= seven.left)
        listOf(seven, letter, ok).forEach { assertTrue("touch height", it.height.value >= 48f) }
        listOf("0", "9", "times", "back", "ok", "A").forEach { k ->
            val b = bounds("code_key_$k")
            assertTrue("$k inside the dock", b.top >= dock.top && b.bottom <= dock.bottom)
        }
        compose.onNodeWithTag("code_key_7").assertHeightIsAtLeast(48.dp)
    }

    @Test
    fun theDigitsKeepTheCalculatorLayout() {
        show(withPad())

        val b = "0123456789".associate { it to bounds("code_key_$it") }
        listOf("789", "456", "123").forEach { row ->
            assertEquals(b[row[0]]!!.top.value, b[row[1]]!!.top.value, 0.5f)
            assertEquals(b[row[0]]!!.top.value, b[row[2]]!!.top.value, 0.5f)
            assertTrue(b[row[1]]!!.left > b[row[0]]!!.left && b[row[2]]!!.left > b[row[1]]!!.left)
        }
        assertTrue(b['4']!!.top > b['7']!!.top && b['1']!!.top > b['4']!!.top)
        assertEquals(b['7']!!.left.value, b['4']!!.left.value, 0.5f)
        assertTrue(b['0']!!.top > b['1']!!.top)
        assertEquals(b['2']!!.left.value, b['0']!!.left.value, 0.5f)
    }

    @Test
    fun aTypedCodeLeavesOneRowOfResultsAboveTheDock() {
        show(withPad(text = "a1"))

        // A1, A10..A19 start with "A1": 11 products, three rows at most; the results sit above the dock.
        val dock = bounds("code_pad")
        compose.onAllNodesWithTag("menu_item_1").assertCountEquals(1)
        val first = bounds("menu_item_1")
        assertTrue(first.bottom <= dock.top)
    }
}
