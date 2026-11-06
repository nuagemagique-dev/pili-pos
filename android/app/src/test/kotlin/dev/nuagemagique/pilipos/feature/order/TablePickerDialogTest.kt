package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.feature.tables.table
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
import uniffi.pos_ffi.TableInfo

/** The "Déplacer" picker (`TablePickerDialog.vue`) and its entry point in the cart header. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TablePickerDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val busyTable = table(4, orderId = 44L, lines = 2, totalCents = 2_400)

    private fun show(
        tables: List<TableInfo>,
        from: String = "1",
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { TablePickerDialog(from, tables, onPick = { seen += "go:$it" }, onCancel = { seen += "cancel" }) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun aFreeTableMovesTheOrderAtOnceWithNoValidateStep() {
        show(listOf(table(2), table(6)))

        compose.onNodeWithTag("pick_table_6").assertIsDisplayed().assertIsEnabled()
        compose.onNodeWithTag("transfer_confirm").assertDoesNotExist()

        compose.onNodeWithTag("pick_table_6").performClick()

        assertEquals(listOf("go:6"), seen)
    }

    @Test
    fun aBusyTableIsDisabledAndShowsItsTotal() {
        show(listOf(table(2), busyTable))

        compose.onNodeWithTag("pick_table_4").assertIsNotEnabled()
        compose.onNodeWithTag("pick_total_4", useUnmergedTree = true).assertTextContains("24,00", substring = true)
        compose.onNodeWithTag("pick_total_2", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithTag("pick_table_4").performClick()

        assertEquals(emptyList<String>(), seen)
    }

    @Test
    fun theTitleSaysDeplacerAndTheTableTheOrderIsOn() {
        show(listOf(table(2)))

        compose.onNodeWithText("Déplacer · Table 1").assertIsDisplayed()
    }

    @Test
    fun withZonesTheTitleAndTheTilesNameTheTablesWithTheirLetter() {
        show(listOf(table(2, zone = "A"), table(21, zone = "B")), from = "A-1")

        compose.onNodeWithText("Déplacer · Table A-1").assertIsDisplayed()
        compose.onNodeWithTag("pick_table_21").assertTextContains("B-21", substring = true)
        // The move still goes by the number: the tap hands back 21.
        compose.onNodeWithTag("pick_table_21").performClick()
        assertEquals(listOf("go:21"), seen)
    }

    @Test
    fun backCancelsWithoutPicking() {
        show(listOf(table(1)))

        compose.onNodeWithTag("transfer_cancel").performClick()

        assertEquals(listOf("cancel"), seen)
    }

    @Test
    fun withOnlyBusyTablesTheDialogSaysNoTableIsFreeAndOffersNoTap() {
        show(listOf(busyTable))

        compose.onNodeWithTag("table_picker").assertIsDisplayed()
        compose.onNodeWithText("Aucune table libre").assertIsDisplayed()
        compose.onNodeWithTag("pick_table_4").assertIsNotEnabled()
    }

    @Test
    fun withNoOtherTableAtAllTheDialogSaysSo() {
        show(emptyList())

        compose.onNodeWithText("Aucune table libre").assertIsDisplayed()
        compose.onNodeWithTag("pick_table_1").assertDoesNotExist()
    }

    @Test
    fun theCartHeaderOffersTheChangeOnlyWhenTheOrderCanMove() {
        val calls = mutableListOf<String>()
        val actions = OrderCartActions(onMore = { calls += "more" })
        val state = OrderCartState(order = order(table = 4, totalCents = 500), canTransfer = true)
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { Box { OrderCartColumn(state, actions) } } }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("cart_transfer").assertDoesNotExist()
        compose.onNodeWithTag("cart_more").performClick()
        assertEquals(listOf("more"), calls)
    }

    @Test
    fun theMenuRowOpensTheTablePickerThroughItsCallback() {
        val calls = mutableListOf<String>()
        val menu = OrderDialog.CartMore(canTransfer = true, canDiscount = false, orderDiscounted = false, canCloseUnpaid = false)
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { CartMoreDialog(menu, OrderCallbacks(onMoreTransfer = { calls += "transfer" })) }
                }
            }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("cart_transfer").performClick()

        assertEquals(listOf("transfer"), calls)
    }

    @Test
    fun theCartHeaderHidesTheChangeWhenTheOrderCannotMove() {
        val state = OrderCartState(order = order(table = null, totalCents = 500), canTransfer = false)
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { Box { OrderCartColumn(state, OrderCartActions()) } } }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("cart_more").assertDoesNotExist()
        compose.onNodeWithTag("cart_transfer").assertDoesNotExist()
    }
}
