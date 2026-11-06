package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertHasClickAction
import androidx.compose.ui.test.assertHasNoClickAction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
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
            onOpenUserMenu = { calls += "user-menu" },
            onCloseUserMenu = { calls += "close-user-menu" },
            onOpenPin = { calls += "pin" },
            onPickLanguage = { calls += "lang-${it.code}" },
            onEndShift = { calls += "end-shift" },
            onMoveTable = { calls += "move-table" },
            onAddCustomer = { calls += "add-customer" },
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
        compose.onNodeWithTag("header_user_role", useUnmergedTree = true).assertTextEquals("Gérant")
        compose.onNodeWithTag("header_clock").assertTextEquals("14:05")
    }

    @Test
    fun theUserButtonOpensTheUserMenu() {
        show(manager)

        compose.onNodeWithTag("header_user").performClick()

        assertEquals(listOf("user-menu"), calls)
    }

    @Test
    fun noMenuUntilTheUserButtonOpensIt() {
        show(manager)

        compose.onNodeWithTag("user_menu").assertDoesNotExist()
    }

    @Test
    fun theUserMenuListsTheLanguageThePinTheEndOfShiftAndTheLock() {
        show(manager.copy(userMenuOpen = true))

        compose.onNodeWithTag("user_menu").assertIsDisplayed()
        compose.onNodeWithTag("user_menu_lang_fr").assertTextEquals("FR")
        compose.onNodeWithTag("user_menu_lang_zh").assertTextEquals("中文")
        compose.onNodeWithTag("user_menu_lang_en").assertTextEquals("EN")
        compose.onNodeWithTag("user_menu_pin").assertHasClickAction()
        compose.onNodeWithTag("user_menu_end_shift").assertHasClickAction()
        compose.onNodeWithTag("user_menu_lock").assertHasClickAction()
        compose.onNodeWithText("Changer mon PIN").assertIsDisplayed()
        compose.onNodeWithText("Fin de service").assertIsDisplayed()
        // The header lock button and the menu row both say it.
        compose.onAllNodesWithText("Verrouiller").assertCountEquals(2)
    }

    @Test
    fun theMenuRowsAreAtLeast48dpHigh() {
        show(manager.copy(userMenuOpen = true))

        for (tag in listOf("user_menu_pin", "user_menu_end_shift", "user_menu_lock", "user_menu_lang_fr")) {
            assertTrue(tag, compose.onNodeWithTag(tag).getUnclippedBoundsInRoot().height >= 48.dp)
        }
    }

    @Test
    fun theMenuRowsCallTheirActions() {
        show(manager.copy(userMenuOpen = true))

        compose.onNodeWithTag("user_menu_lang_zh").performClick()
        compose.onNodeWithTag("user_menu_pin").performClick()
        compose.onNodeWithTag("user_menu_end_shift").performClick()
        compose.onNodeWithTag("user_menu_lock").performClick()

        assertEquals(listOf("lang-zh", "pin", "end-shift", "lock"), calls)
    }

    @Test
    fun theChosenLanguageIsTheSelectedOne() {
        show(manager.copy(userMenuOpen = true, language = AppLanguage.ZH))

        compose.onNodeWithTag("user_menu_lang_zh").assertIsSelected()
        compose.onNodeWithTag("user_menu_lang_fr").assertIsNotSelected()
    }

    @Test
    fun anArmedEndOfShiftAsksForConfirmation() {
        show(manager.copy(userMenuOpen = true, endShiftArmed = true))

        compose.onNodeWithText("Confirmer la fin de service ?").assertIsDisplayed()
        compose.onNodeWithText("Fin de service").assertDoesNotExist()
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
    fun theTableChipOfAMovableOrderIsA48dpButtonThatOpensThePicker() {
        show(manager.copy(area = HeaderArea.ORDER, tableNumber = 12, dineInOrder = true, canMoveTable = true))

        compose.onNodeWithTag("header_table").assertTextEquals("Table 12")
        compose.onNodeWithTag("header_table").assertIsDisplayed().assertHasClickAction()
        assertTrue(compose.onNodeWithTag("header_table").getUnclippedBoundsInRoot().height >= 48.dp)

        compose.onNodeWithTag("header_table").performClick()

        assertEquals(listOf("move-table"), calls)
    }

    @Test
    fun withZonesTheTableChipWritesTheLetterAndTheNumber() {
        show(manager.copy(area = HeaderArea.ORDER, tableNumber = 12, tableLabel = "B-12", dineInOrder = true, canMoveTable = true))

        compose.onNodeWithTag("header_table").assertTextEquals("Table B-12")
    }

    @Test
    fun theTableChipIsAPlainLabelWhileTheOrderCannotMove() {
        show(manager.copy(area = HeaderArea.ORDER, tableNumber = 12, dineInOrder = true, canMoveTable = false))

        compose.onNodeWithTag("header_table").assertTextEquals("Table 12").assertHasNoClickAction()
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

    @Test
    fun aNumberedTakeawayTitlesTheHeaderWithItsNumberAndCustomerAndOffersAddingAName() {
        show(
            manager.copy(
                area = HeaderArea.ORDER,
                takeawayOrder = true,
                takeawayLabel = "E-61",
                customerEditable = true,
            ),
        )

        compose.onNodeWithTag("header_takeaway_badge").assertTextContains("E-61", substring = true)
        compose.onNodeWithTag("header_takeaway_customer").assertTextContains("Ajouter un nom")
        compose.onNodeWithTag("header_takeaway_customer").assertHasClickAction()
        assertTrue(compose.onNodeWithTag("header_takeaway_customer").getUnclippedBoundsInRoot().height >= 48.dp)
        compose.onNodeWithTag("header_takeaway_customer").performClick()
        assertEquals(listOf("add-customer"), calls)
    }

    @Test
    fun aNamedTakeawayShowsTheNameInTheTitleAndOffersEditingTheCustomer() {
        show(
            manager.copy(
                area = HeaderArea.ORDER,
                takeawayOrder = true,
                takeawayLabel = "E-61",
                customerName = "Dupont",
                customerEditable = true,
            ),
        )

        compose.onNodeWithTag("header_takeaway_badge").assertTextContains("E-61 · Dupont", substring = true)
        compose.onNodeWithTag("header_takeaway_customer").assertTextContains("Modifier le client")
    }

    @Test
    fun anOrderWithoutANumberOffersNoCustomerButton() {
        show(manager.copy(area = HeaderArea.ORDER, takeawayOrder = true))

        compose.onNodeWithTag("header_takeaway_customer").assertDoesNotExist()
    }
}
