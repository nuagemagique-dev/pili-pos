package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class NumPadTest {
    @get:Rule
    val compose = createComposeRule()

    private val keys = mutableListOf<NumPadKey>()

    private fun show(enabled: Boolean = true) {
        compose.setContent {
            PiliTheme {
                NumPad(onKey = { keys += it }, clearLabel = "Effacer", backspaceLabel = "Retour", enabled = enabled)
            }
        }
    }

    @Test
    fun everyDigitKeyEmitsItsDigit() {
        show()
        ('0'..'9').forEach { d -> compose.onNodeWithTag("numpad_key_$d").assertIsDisplayed().performClick() }
        assertEquals(('0'..'9').map { NumPadKey.Digit(it) }, keys)
    }

    @Test
    fun clearAndBackspaceEmitTheirKeys() {
        show()
        compose.onNodeWithTag("numpad_clear").performClick()
        compose.onNodeWithTag("numpad_back").performClick()
        assertEquals(listOf(NumPadKey.Clear, NumPadKey.Backspace), keys)
    }

    @Test
    fun disabledPadEmitsNothing() {
        show(enabled = false)
        ('0'..'9').forEach { d -> compose.onNodeWithTag("numpad_key_$d").performClick() }
        compose.onNodeWithTag("numpad_clear").performClick()
        compose.onNodeWithTag("numpad_back").performClick()
        assertEquals(emptyList<NumPadKey>(), keys)
    }

    @Test
    fun glyphKeysCarryTheirContentDescriptions() {
        show()
        compose.onNodeWithContentDescription("Effacer").performClick()
        compose.onNodeWithContentDescription("Retour").performClick()
        assertEquals(listOf(NumPadKey.Clear, NumPadKey.Backspace), keys)
    }

    @Test
    fun aDigitKeyOnlyAcceptsDigits() {
        assertEquals('7', NumPadKey.Digit('7').digit)
        assertThrows(IllegalArgumentException::class.java) { NumPadKey.Digit('a') }
    }
}
