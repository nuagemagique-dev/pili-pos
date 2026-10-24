package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Golden of the TRAINING lettering over a screen in the training palette. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class TrainingWatermarkScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun watermark() {
        rule.setContent {
            PiliTheme(training = true) {
                Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                    Text("Ecran en dessous", color = PiliTheme.colors.textPrimary)
                    TrainingWatermark()
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/training_watermark.png")
    }
}
