package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class ConfirmDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private var confirmed = 0
    private var cancelled = 0

    private fun show(destructive: Boolean) {
        compose.setContent {
            PiliTheme {
                ConfirmDialog(
                    title = "Annuler la vente",
                    message = "Les lignes seront perdues.",
                    confirmText = "Annuler",
                    cancelText = "Retour",
                    onConfirm = { confirmed++ },
                    onCancel = { cancelled++ },
                    destructive = destructive,
                )
            }
        }
    }

    @Test
    fun showsTheTextsAndBothButtons() {
        show(destructive = false)
        compose.onNodeWithText("Annuler la vente").assertIsDisplayed()
        compose.onNodeWithText("Les lignes seront perdues.").assertIsDisplayed()
        compose.onNodeWithTag("confirm_ok").assertTextEquals("Annuler")
        compose.onNodeWithTag("confirm_cancel").assertTextEquals("Retour")
    }

    @Test
    fun confirmButtonConfirms() {
        show(destructive = true)
        compose.onNodeWithTag("confirm_ok").performClick()
        assertEquals(1, confirmed)
        assertEquals(0, cancelled)
    }

    @Test
    fun cancelButtonCancels() {
        show(destructive = false)
        compose.onNodeWithTag("confirm_cancel").performClick()
        assertEquals(0, confirmed)
        assertEquals(1, cancelled)
    }
}
