package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
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

/** The "change table" picker (`TablePickerDialog.vue`) and its entry point in the cart header. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TablePickerDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private fun show(tables: List<Int>) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { TablePickerDialog(4, tables, onConfirm = { seen += "go:$it" }, onCancel = { seen += "cancel" }) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun validateStaysDisabledUntilAFreeTableIsPicked() {
        show(listOf(1, 2, 6))

        compose.onNodeWithTag("pick_table_6").assertIsDisplayed()
        compose.onNodeWithTag("transfer_confirm").assertIsNotEnabled()

        compose.onNodeWithTag("pick_table_6").performClick()
        compose.onNodeWithTag("transfer_confirm").assertIsEnabled()
        compose.onNodeWithTag("transfer_confirm").performClick()

        assertEquals(listOf("go:6"), seen)
    }

    @Test
    fun backCancelsWithoutPicking() {
        show(listOf(1))

        compose.onNodeWithTag("transfer_cancel").performClick()

        assertEquals(listOf("cancel"), seen)
    }

    @Test
    fun withNoFreeTableTheDialogSaysSoAndOffersNone() {
        show(emptyList())

        compose.onNodeWithTag("table_picker").assertIsDisplayed()
        compose.onNodeWithTag("pick_table_1").assertDoesNotExist()
        compose.onNodeWithTag("transfer_confirm").assertIsNotEnabled()
    }

    @Test
    fun theCartHeaderOffersTheChangeOnlyWhenTheOrderCanMove() {
        val calls = mutableListOf<String>()
        val actions = OrderCartActions(onTransfer = { calls += "transfer" })
        val state = OrderCartState(order = order(table = 4, totalCents = 500), canTransfer = true)
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { Box { OrderCartColumn(state, actions) } } }
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

        compose.onNodeWithTag("cart_transfer").assertDoesNotExist()
    }
}
