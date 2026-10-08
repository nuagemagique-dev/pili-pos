package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTouchInput
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class PiliDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private var dismissed = 0
    private var behindTaps = 0

    private fun show(
        dismissible: Boolean,
        withActions: Boolean = true,
    ) {
        compose.setContent {
            PiliTheme {
                PiliDialog(
                    title = "Titre",
                    onDismissRequest = if (dismissible) ({ dismissed++ }) else null,
                    actions = if (withActions) ({ Text("Action") }) else ({}),
                ) {
                    Text("Contenu")
                }
            }
        }
    }

    private fun tapScrimCorner() {
        compose.onNodeWithTag("pili_dialog_scrim").performTouchInput { click(Offset(4f, 4f)) }
    }

    @Test
    fun showsTitleContentAndActions() {
        show(dismissible = true)
        compose.onNodeWithTag("pili_dialog").assertIsDisplayed()
        compose.onNodeWithText("Titre").assertIsDisplayed()
        compose.onNodeWithText("Contenu").assertIsDisplayed()
        compose.onNodeWithText("Action").assertIsDisplayed()
    }

    @Test
    fun scrimTapDismissesWhenThereIsACallback() {
        show(dismissible = true)
        tapScrimCorner()
        assertEquals(1, dismissed)
    }

    @Test
    fun scrimTapIsConsumedWithoutACallback() {
        show(dismissible = false)
        tapScrimCorner()
        assertEquals(0, dismissed)
        compose.onNodeWithTag("pili_dialog").assertIsDisplayed()
    }

    @Test
    fun cardTapDoesNotDismiss() {
        show(dismissible = true)
        compose.onNodeWithTag("pili_dialog_card").performTouchInput { click() }
        assertEquals(0, dismissed)
    }

    @Test
    fun noActionsRendersNoActionRow() {
        show(dismissible = true, withActions = false)
        compose.onNodeWithText("Action").assertDoesNotExist()
    }

    @Test
    fun scrimStopsTapsFromReachingTheContentBehind() {
        compose.setContent {
            PiliTheme {
                Box {
                    Box(Modifier.fillMaxSize().clickable { behindTaps++ })
                    PiliDialog(title = "T", onDismissRequest = null) { Text("C") }
                }
            }
        }
        tapScrimCorner()
        assertEquals(0, behindTaps)
    }
}
