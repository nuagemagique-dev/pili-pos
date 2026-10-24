package dev.nuagemagique.pilipos.feature.tables

import androidx.compose.ui.test.assertIsDisplayed
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
import java.time.Instant

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TablesContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val tapped = mutableListOf<String>()

    private val callbacks =
        TablesCallbacks(
            onTable = { tapped += "table:${it.tableNumber}" },
            onTakeaway = { tapped += "takeaway:${it.orderId}" },
            onNewTakeaway = { tapped += "new" },
            onToggleMove = { tapped += "toggle" },
            onConfirmMove = { tapped += "validate" },
            onCancelTarget = { tapped += "back" },
        )

    // 10:30 UTC is 12:30 in Brussels in October.
    private val now = Instant.parse("2026-10-09T10:30:00Z")

    private val base =
        TablesUiState(
            tables =
                listOf(
                    table(1),
                    table(2, orderId = 40, lines = 3, totalCents = 2_550, openedAt = "2026-10-09 10:00:00"),
                    table(3, orderId = 41, lines = 1, totalCents = 123_456, openedAt = null, preBill = true),
                ),
            takeaways = listOf(takeaway(50, lines = 2, totalCents = 1_800, createdAt = "2026-10-09 10:05:00")),
            now = now,
            signedIn = true,
            loaded = true,
        )

    private fun show(state: TablesUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { TablesContent(state, callbacks) }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun aFreeTableShowsItsNumberAndTheFreeLabelWithoutATotal() {
        show(base)

        compose.onNodeWithTag("table_1").assertIsDisplayed()
        compose.onNodeWithTag("table_1_total", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithTag("table_1_bill", useUnmergedTree = true).assertDoesNotExist()
    }

    @Test
    fun anOccupiedTableShowsTheEnginesAmountItsLinesAndTheMinutesOpen() {
        show(base)

        compose.onNodeWithTag("table_2_total", useUnmergedTree = true).assertTextEquals("25,50 €")
        compose.onNodeWithTag("table_2_meta", useUnmergedTree = true).assertTextEquals("3 art. · 30 min")
    }

    @Test
    fun anOccupiedTableWithoutAnOpeningTimeShowsNoMinutesAndLargeAmountsAreGrouped() {
        show(base)

        compose.onNodeWithTag("table_3_total", useUnmergedTree = true).assertTextEquals("1 234,56 €")
        compose.onNodeWithTag("table_3_meta", useUnmergedTree = true).assertTextEquals("1 art.")
    }

    @Test
    fun aProvisionalBillIsFlaggedOnItsTable() {
        show(base)

        compose.onNodeWithTag("table_3_bill", useUnmergedTree = true).assertIsDisplayed()
    }

    @Test
    fun anOrderWhoseCardWasChargedIsFlaggedOnItsTableAndItsTakeaway() {
        show(base.copy(cardCharged = setOf(40L, 50L)))

        compose.onNodeWithTag("table_2_card", useUnmergedTree = true).assertTextEquals("Carte encaissée")
        compose.onNodeWithTag("table_3_card", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithTag("takeaway_50_card", useUnmergedTree = true).assertTextEquals("Carte encaissée")
    }

    @Test
    fun withNoChargedCardNothingIsFlagged() {
        show(base)

        compose.onNodeWithTag("table_2_card", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithTag("takeaway_50_card", useUnmergedTree = true).assertDoesNotExist()
    }

    @Test
    fun theTakeawayListShowsTheCountTheStartTimeInBrusselsAndTheAmount() {
        show(base)

        compose.onNodeWithTag("takeaways_count").assertTextEquals("1")
        // 10:05 UTC is 12:05 in Brussels.
        compose.onNodeWithTag("takeaway_50").assertIsDisplayed()
        compose.onNodeWithTag("takeaway_50").assertTextContains("12:05", substring = true)
        compose.onNodeWithTag("takeaway_50").assertTextContains("2 art.", substring = true)
        compose.onNodeWithTag("takeaway_50").assertTextContains("18,00 €", substring = true)
    }

    @Test
    fun tappingATableATakeawayAndTheNewTakeawayButtonCallsBack() {
        show(base)

        compose.onNodeWithTag("table_1").performClick()
        compose.onNodeWithTag("table_2").performClick()
        compose.onNodeWithTag("takeaway_50").performClick()
        compose.onNodeWithTag("takeaway_new").performClick()

        assertEquals(listOf("table:1", "table:2", "takeaway:50", "new"), tapped)
    }

    @Test
    fun withoutAnyTableTheEmptyStateSaysSo() {
        show(base.copy(tables = emptyList(), takeaways = emptyList()))

        compose.onNodeWithTag("tables_none").assertIsDisplayed()
        compose.onNodeWithTag("takeaways_count").assertTextEquals("0")
        compose.onNodeWithTag("takeaway_new").assertIsDisplayed()
    }

    @Test
    fun beforeTheFirstLoadNoEmptyMessageFlashes() {
        show(base.copy(tables = emptyList(), takeaways = emptyList(), loaded = false))

        compose.onNodeWithTag("tables_none").assertDoesNotExist()
    }

    @Test
    fun withNoOperatorTheScreenShowsAnEmptyStateAndNoGrid() {
        show(TablesUiState(signedIn = false))

        compose.onNodeWithTag("tables_empty").assertIsDisplayed()
        compose.onNodeWithTag("tables_grid").assertDoesNotExist()
        compose.onNodeWithTag("takeaway_new").assertDoesNotExist()
    }

    @Test
    fun moreThanTwentyTablesWrapOntoFurtherRowsOfFive() {
        val many = List(23) { table(it + 1) }
        show(base.copy(tables = many))

        compose.onNodeWithTag("table_1").assertIsDisplayed()
        compose.onNodeWithTag("table_5").assertIsDisplayed()
        compose.onNodeWithTag("table_16").assertIsDisplayed()
        compose.onNodeWithTag("table_23").assertExists()
    }

    // ── Moving a table ────────────────────────────────────────────

    @Test
    fun theMoveToggleCallsBackAndTheHintFollowsTheMode() {
        show(base)
        compose.onNodeWithTag("move_toggle").performClick()
        assertEquals(listOf("toggle"), tapped)
        compose.onNodeWithTag("move_hint").assertDoesNotExist()
    }

    @Test
    fun inTheMoveModeTheHintAsksForAnOccupiedTable() {
        show(base.copy(move = MoveState(moving = true)))

        compose.onNodeWithTag("move_hint").assertIsDisplayed()
    }

    @Test
    fun onceTheSourceIsChosenTheHintNamesItAndTheConfirmationAsksForValidation() {
        val source = base.tables[1]
        val target = base.tables[0]
        val moving = base.copy(move = MoveState(moving = true, source = source))
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    androidx.compose.foundation.layout.Box {
                        TablesContent(moving, callbacks)
                        MoveConfirmation(MoveState(true, source, target), callbacks)
                    }
                }
            }
        }
        compose.waitForIdle()
        compose.onNodeWithTag("move_hint").assertTextContains("2", substring = true)
        compose.onNodeWithTag("move_validate").performClick()
        compose.onNodeWithTag("move_cancel").performClick()
        assertEquals(listOf("validate", "back"), tapped)
    }
}
