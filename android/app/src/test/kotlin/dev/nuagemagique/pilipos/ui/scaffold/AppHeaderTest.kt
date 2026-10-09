package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
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

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class AppHeaderTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()
    private var state by mutableStateOf(HeaderUiState())

    private val actions =
        HeaderActions(
            onTables = { calls += "tables" },
            onNewTakeaway = { calls += "takeaway" },
            onOnlineOrders = { calls += "online" },
            onReports = { calls += "reports" },
            onSettings = { calls += "settings" },
            onLock = { calls += "lock" },
            onOpenPin = { calls += "pin" },
        )

    private fun show(initial: HeaderUiState) {
        state = initial
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { AppHeader(state, actions, clock = "14:05") }
            }
        }
        compose.waitForIdle()
    }

    private val manager = HeaderUiState(area = HeaderArea.TABLES, userName = "Alice", roleKey = "role_admin", isManager = true)

    @Test
    fun theHeaderShowsTheLogoTheOperatorTheRoleWithPinAndTheClock() {
        show(manager)

        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("header_logo").assertIsDisplayed()
        compose.onNodeWithTag("header_user_name", useUnmergedTree = true).assertTextEquals("Alice")
        compose.onNodeWithTag("header_user_role", useUnmergedTree = true).assertTextEquals("Gérant · PIN")
        compose.onNodeWithTag("header_clock").assertTextEquals("14:05")
    }

    @Test
    fun theUserButtonOpensTheOwnPinDialog() {
        show(manager)

        compose.onNodeWithTag("header_user").performClick()

        assertEquals(listOf("pin"), calls)
    }

    @Test
    fun theButtonsCallTheirActions() {
        show(manager.copy(hubriseVisible = true))

        compose.onNodeWithTag("header_nav_tables").performClick()
        compose.onNodeWithTag("header_nav_takeaway").performClick()
        compose.onNodeWithTag("header_nav_online").performClick()
        compose.onNodeWithTag("header_nav_reports").performClick()
        compose.onNodeWithTag("header_nav_settings").performClick()
        compose.onNodeWithTag("header_lock").performClick()

        assertEquals(listOf("tables", "takeaway", "online", "reports", "settings", "lock"), calls)
    }

    @Test
    fun reportsAndSettingsShowForManagersOnly() {
        show(manager.copy(isManager = false, roleKey = "role_cashier"))

        compose.onNodeWithTag("header_nav_reports").assertDoesNotExist()
        compose.onNodeWithTag("header_nav_settings").assertDoesNotExist()
        compose.onNodeWithTag("header_lock").assertIsDisplayed()
    }

    @Test
    fun theOnlineOrdersButtonShowsOnlyWhileHubriseIsVisible() {
        show(manager)
        compose.onNodeWithTag("header_nav_online").assertDoesNotExist()

        state = state.copy(hubriseVisible = true)
        compose.waitForIdle()
        compose.onNodeWithTag("header_nav_online").assertIsDisplayed()
    }

    @Test
    fun theBadgeCountsTheWaitingOnlineOrdersAndDisappearsAtZero() {
        show(manager.copy(hubriseVisible = true))
        compose.onNodeWithTag("header_online_badge", useUnmergedTree = true).assertDoesNotExist()

        state = state.copy(pendingOrders = 3)
        compose.waitForIdle()
        compose.onNodeWithTag("header_online_badge", useUnmergedTree = true).assertTextEquals("3")

        state = state.copy(pendingOrders = 4)
        compose.waitForIdle()
        compose.mainClock.advanceTimeBy(400)
        compose.onNodeWithTag("header_online_badge", useUnmergedTree = true).assertTextEquals("4")

        state = state.copy(pendingOrders = 0)
        compose.waitForIdle()
        compose.onNodeWithTag("header_online_badge", useUnmergedTree = true).assertDoesNotExist()
    }

    @Test
    fun aDineInOrderShowsTheTableAndMarksTablesAsTheCurrentPlace() {
        show(manager.copy(area = HeaderArea.ORDER, tableNumber = 12, dineInOrder = true))

        compose.onNodeWithTag("header_table").assertTextEquals("Table 12")
        compose.onNodeWithTag("header_takeaway_badge").assertDoesNotExist()
        compose.onNodeWithTag("header_nav_tables").assertIsSelected()
        compose.onNodeWithTag("header_nav_takeaway").assertIsNotSelected()
    }

    @Test
    fun aTakeawayOrderShowsTheAmberTagAndMarksTheTakeawayButton() {
        show(manager.copy(area = HeaderArea.ORDER, takeawayOrder = true))

        compose.onNodeWithTag("header_takeaway_badge").assertTextContains("emporter", substring = true, ignoreCase = true)
        compose.onNodeWithTag("header_table").assertDoesNotExist()
        compose.onNodeWithTag("header_nav_takeaway").assertIsSelected()
        compose.onNodeWithTag("header_nav_tables").assertIsNotSelected()
    }

    @Test
    fun theCurrentWorkspaceIsTheSelectedButton() {
        show(manager.copy(area = HeaderArea.REPORTS))

        compose.onNodeWithTag("header_nav_reports").assertIsSelected()
        compose.onNodeWithTag("header_nav_settings").assertIsNotSelected()
        compose.onNodeWithTag("header_nav_tables").assertIsNotSelected()
    }

    @Test
    fun theTablesButtonIsSelectedOnTheTablesScreen() {
        show(manager)

        compose.onNodeWithTag("header_nav_tables").assertIsSelected()
    }

    @Test
    fun noOperatorGivesAnEmptyNameAndStillRenders() {
        show(HeaderUiState())

        compose.onNodeWithTag("header_user_name", useUnmergedTree = true).assertTextEquals("")
        compose.onNodeWithTag("app_header").assertIsDisplayed()
    }
}
