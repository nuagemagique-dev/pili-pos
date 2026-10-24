package dev.nuagemagique.pilipos.platform.export

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.ui.theme.PiliDensity
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Goldens of the export screen at the design width (960 px window, density 0.5 = the 1920 px till). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class ExportScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val sd = RemovableVolume("BFD1-0A11", "SD card")

    @Test
    fun exportReady() {
        rule.setContent {
            PiliTheme { PiliDensity { ExportContent(ExportUiState.Ready(listOf(sd), sd, "PILI-EXPORT"), ExportActions()) } }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/platform/export_ready.png")
    }

    @Test
    fun exportRunning() {
        val progress = ExportProgress(7, 21, 40_000_000, 120_000_000, "backups/pos-backup-05.db")
        rule.setContent { PiliTheme { PiliDensity { ExportContent(ExportUiState.Running(progress), ExportActions()) } } }
        rule.onRoot().captureRoboImage("src/test/screenshots/platform/export_running.png")
    }
}
