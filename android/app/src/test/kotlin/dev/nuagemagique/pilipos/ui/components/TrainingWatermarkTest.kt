package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class TrainingWatermarkTest {
    @get:Rule
    val compose = createComposeRule()

    private val training = MutableStateFlow(false)
    private val settings = mockk<PosSettingsStore> { every { trainingMode } returns training }

    @Test
    fun theOrderIsFifty() {
        assertEquals(50, TrainingWatermarkOverlay(settings).order)
    }

    @Test
    fun theOverlayDrawsNothingOutsideTrainingMode() {
        val overlay = TrainingWatermarkOverlay(settings)
        compose.setContent { PiliTheme { overlay.Content() } }
        compose.onNodeWithTag("training_watermark").assertDoesNotExist()
    }

    @Test
    fun theOverlayAppearsWhenTrainingTurnsOnAndGoesWhenItTurnsOff() {
        val overlay = TrainingWatermarkOverlay(settings)
        compose.setContent { PiliTheme { overlay.Content() } }

        training.value = true
        compose.waitForIdle()
        compose.onNodeWithTag("training_watermark").assertIsDisplayed()

        training.value = false
        compose.waitForIdle()
        compose.onNodeWithTag("training_watermark").assertDoesNotExist()
    }

    @Test
    fun aButtonUnderTheWatermarkStaysClickable() {
        var clicks by mutableIntStateOf(0)
        training.value = true
        val overlay = TrainingWatermarkOverlay(settings)
        compose.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize()) {
                    Button(onClick = { clicks++ }, modifier = Modifier.testTag("under")) { Text("under") }
                    overlay.Content()
                }
            }
        }
        compose.onNodeWithTag("training_watermark").assertIsDisplayed()
        compose.onNodeWithTag("under").performClick()
        assertEquals(1, clicks)
    }
}
