package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
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

/** Golden of the numeric pad. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w320dp-h340dp-mdpi")
class NumPadScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun numPadDefault() {
        rule.setContent {
            PiliTheme {
                Box(Modifier.background(PiliTheme.colors.bgSecondary).padding(PiliTheme.dimens.spaceMd)) {
                    NumPad(onKey = {}, clearLabel = "Effacer", backspaceLabel = "Corriger")
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/numpad_default.png")
    }
}
