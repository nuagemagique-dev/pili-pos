package dev.nuagemagique.pilipos.feature.tables

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.longClick
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
import java.time.Instant

/** The long press on a table card and the sheet it opens (UX audit T7): the Compose side. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TablesSheetContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val callbacks =
        TablesCallbacks(
            onTable = { seen += "table:${it.tableNumber}" },
            onTableLongPress = { seen += "long:${it.tableNumber}" },
            onSheetOpen = { seen += "open" },
            onSheetBill = { seen += "bill" },
            onSheetMove = { seen += "move" },
            onSheetPay = { seen += "pay" },
            onSheetClose = { seen += "close" },
        )

    private val state =
        TablesUiState(
            tables = listOf(table(1), table(2, orderId = 40, lines = 3, totalCents = 2_550, zone = "A")),
            now = Instant.parse("2026-10-09T10:30:00Z"),
            signedIn = true,
            loaded = true,
        )

    private fun show(
        ui: TablesUiState = state,
        sheet: TableSheet? = null,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    TablesContent(ui, callbacks)
                    sheet?.let { TableActionSheet(it, callbacks) }
                }
            }
        }
        compose.waitForIdle()
    }

    // ── The gesture ───────────────────────────────────────────────

    @Test
    fun aLongPressOnAnOccupiedTableAsksForItsSheetAndIsNotATap() {
        show()

        compose.onNodeWithTag("table_2").performTouchInput { longClick() }
        compose.waitForIdle()

        assertEquals(listOf("long:2"), seen)
    }

    @Test
    fun aShortTapStillOpensTheTable() {
        show()

        compose.onNodeWithTag("table_2").performClick()

        assertEquals(listOf("table:2"), seen)
    }

    @Test
    fun aPressHeldUnder500MsIsStillATap() {
        show()

        compose.onNodeWithTag("table_2").performTouchInput {
            down(center)
            advanceEventTime(450)
            up()
        }
        compose.waitForIdle()

        assertEquals(listOf("table:2"), seen)
    }

    @Test
    fun aPressHeldPast500MsAsksForTheSheet() {
        show()

        compose.onNodeWithTag("table_2").performTouchInput {
            down(center)
            advanceEventTime(600)
            up()
        }
        compose.waitForIdle()

        assertEquals(listOf("long:2"), seen)
    }

    @Test
    fun aLongPressOnAFreeTableIsATapLikeBefore() {
        show()

        compose.onNodeWithTag("table_1").performTouchInput { longClick() }
        compose.waitForIdle()

        assertEquals(listOf("table:1"), seen)
    }

    @Test
    fun inTheMoveModeALongPressIsNotASheet() {
        show(state.copy(move = MoveState(moving = true)))

        compose.onNodeWithTag("table_2").performTouchInput { longClick() }
        compose.waitForIdle()

        assertEquals(listOf("table:2"), seen)
    }

    // ── The sheet ─────────────────────────────────────────────────

    @Test
    fun theSheetOffersTheFourActionsToACashier() {
        show(sheet = TableSheet(state.tables[1], canTakePayment = true))

        compose.onNodeWithTag("table_sheet").assertIsDisplayed()
        compose.onNodeWithTag("sheet_open").assertTextEquals("Ouvrir")
        compose.onNodeWithTag("sheet_bill").assertTextEquals("Addition")
        compose.onNodeWithTag("sheet_move").assertTextEquals("Déplacer")
        compose.onNodeWithTag("sheet_pay").assertTextEquals("Payer")
        compose.onNodeWithTag("sheet_close").assertTextEquals("Fermer")
        compose.onNodeWithTag("sheet_total").assertTextContains("25,50", substring = true)
        compose.onNodeWithText("Table A-2").assertIsDisplayed()
    }

    @Test
    fun aWaiterSeesOuvrirAndDeplacerOnly() {
        show(sheet = TableSheet(state.tables[1], canTakePayment = false))

        compose.onNodeWithTag("sheet_open").assertIsDisplayed()
        compose.onNodeWithTag("sheet_move").assertIsDisplayed()
        compose.onNodeWithTag("sheet_bill").assertDoesNotExist()
        compose.onNodeWithTag("sheet_pay").assertDoesNotExist()
    }

    @Test
    fun eachActionCallsItsCallback() {
        show(sheet = TableSheet(state.tables[1], canTakePayment = true))

        listOf("sheet_open", "sheet_bill", "sheet_move", "sheet_pay", "sheet_close").forEach {
            compose.onNodeWithTag(it).performClick()
        }

        assertEquals(listOf("open", "bill", "move", "pay", "close"), seen)
    }

    @Test
    fun everyActionIsAtLeast48DpHigh() {
        show(sheet = TableSheet(state.tables[1], canTakePayment = true))

        listOf("sheet_open", "sheet_bill", "sheet_move", "sheet_pay", "sheet_close").forEach {
            val height = compose.onNodeWithTag(it).fetchSemanticsNode().size.height
            assert(height >= 48) { "$it is $height px high" }
        }
    }
}
