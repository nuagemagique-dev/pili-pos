package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.unit.dp
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Golden of a fixed 21 by 21 matrix (three finder patterns and a diagonal pattern, not a scannable code). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w320dp-h320dp-mdpi")
class QrCodeScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private fun finder(
        row: Int,
        column: Int,
        top: Int,
        left: Int,
    ): Boolean {
        val r = row - top
        val c = column - left
        if (r !in 0..6 || c !in 0..6) return false
        val ring = r == 0 || r == 6 || c == 0 || c == 6
        val core = r in 2..4 && c in 2..4
        return ring || core
    }

    private val matrix: List<List<Boolean>> =
        List(21) { row ->
            List(21) { column ->
                finder(row, column, 0, 0) || finder(row, column, 0, 14) || finder(row, column, 14, 0) ||
                    (row in 8..20 && column in 8..20 && (row * 3 + column * 5) % 7 < 3)
            }
        }

    @Test
    fun sampleMatrix() {
        rule.setContent {
            PiliTheme {
                Box(Modifier.background(PiliTheme.colors.bgPrimary).padding(16.dp)) {
                    QrCanvas(matrix, moduleSize = 8.dp)
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/qr_sample.png")
    }
}
