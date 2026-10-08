package dev.nuagemagique.pilipos.ui.components

import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
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
class PiliButtonTest {
    @get:Rule
    val compose = createComposeRule()

    private var clicks = 0

    @Test
    fun everyVariantClicks() {
        compose.setContent {
            PiliTheme {
                PiliButtonVariant.entries.forEach { v ->
                    PiliButton("Go ${v.name}", { clicks++ }, Modifier.testTag("b_${v.name}"), variant = v)
                }
            }
        }
        PiliButtonVariant.entries.forEach { v ->
            compose.onNodeWithTag("b_${v.name}").assertIsEnabled().performClick()
        }
        assertEquals(PiliButtonVariant.entries.size, clicks)
        compose.onNodeWithText("Go Primary").assertIsEnabled()
    }

    @Test
    fun disabledButtonIsNotClickable() {
        compose.setContent {
            PiliTheme { PiliButton("Go", { clicks++ }, Modifier.testTag("b"), enabled = false) }
        }
        compose.onNodeWithTag("b").assertIsNotEnabled().performClick()
        assertEquals(0, clicks)
    }

    @Test
    fun contentSlotOverloadClicks() {
        compose.setContent {
            PiliTheme {
                PiliButton(onClick = { clicks++ }, modifier = Modifier.testTag("slot"), variant = PiliButtonVariant.Ghost) {
                    Text("A")
                    Text("B")
                }
            }
        }
        compose.onNodeWithTag("slot").performClick()
        assertEquals(1, clicks)
    }
}
