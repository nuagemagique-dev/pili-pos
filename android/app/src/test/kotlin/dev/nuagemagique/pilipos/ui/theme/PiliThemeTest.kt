package dev.nuagemagique.pilipos.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.junit4.v2.createComposeRule
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
class PiliThemeTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun standardThemeMapsTheMaterialScheme() {
        rule.setContent {
            PiliTheme {
                val s = MaterialTheme.colorScheme
                val c = PiliTheme.colors
                assertEquals(c.bgPrimary, s.background)
                assertEquals(c.textPrimary, s.onBackground)
                assertEquals(c.bgSecondary, s.surface)
                assertEquals(c.textPrimary, s.onSurface)
                assertEquals(c.bgSurface, s.surfaceVariant)
                assertEquals(c.textSecondary, s.onSurfaceVariant)
                assertEquals(c.accentPrimary, s.primary)
                assertEquals(c.textOnAccent, s.onPrimary)
                assertEquals(c.selectedBg, s.secondary)
                assertEquals(c.selectedFg, s.onSecondary)
                assertEquals(c.accentInfo, s.tertiary)
                assertEquals(c.accentDanger, s.error)
                assertEquals(c.textOnAccent, s.onError)
                assertEquals(c.border, s.outline)
                assertEquals(PiliColors.Standard, c)
            }
        }
        rule.waitForIdle()
    }

    @Test
    fun trainingSwapsTheSurfaces() {
        rule.setContent {
            PiliTheme(training = true) {
                val s = MaterialTheme.colorScheme
                assertEquals(PiliColors.Training, PiliTheme.colors)
                assertEquals(PiliColors.Training.bgPrimary, s.background)
                assertEquals(PiliColors.Training.bgSecondary, s.surface)
                assertEquals(PiliColors.Training.bgSurface, s.surfaceVariant)
                assertEquals(PiliColors.Training.textSecondary, s.onSurfaceVariant)
                assertEquals(PiliColors.Standard.accentPrimary, s.primary)
            }
        }
        rule.waitForIdle()
    }

    @Test
    fun themeProvidesDimensAndTypography() {
        rule.setContent {
            PiliTheme {
                assertEquals(PiliDimens(), PiliTheme.dimens)
                assertEquals(PiliTypography(), PiliTheme.typography)
                assertEquals(670f, PiliTheme.dimens.cartWidth.value, 0f)
            }
        }
        rule.waitForIdle()
    }

    private fun densityAt(widthPx: Int): Pair<Float, Float> {
        var result = 0f to 0f
        rule.setContent {
            CompositionLocalProvider(LocalDensity provides androidx.compose.ui.unit.Density(2f, 1.3f)) {
                PiliDensity(containerWidthPx = widthPx) {
                    result = LocalDensity.current.density to LocalDensity.current.fontScale
                }
            }
        }
        rule.waitForIdle()
        return result
    }

    @Test
    fun densityIsOneAtDesignWidth() {
        val (density, fontScale) = densityAt(1920)
        assertEquals(1f, density, 0f)
        assertEquals(1f, fontScale, 0f)
    }

    @Test
    fun densityScalesDownAt1280() {
        val (density, fontScale) = densityAt(1280)
        assertEquals(0.6667f, density, 0.0001f)
        assertEquals(1f, fontScale, 0f)
    }

    @Test
    fun unknownWidthKeepsTheParentDensity() {
        val (density, fontScale) = densityAt(0)
        assertEquals(2f, density, 0f)
        assertEquals(1.3f, fontScale, 0f)
    }

    @Test
    @Config(qualifiers = "w1280dp-h720dp-mdpi")
    fun windowWidthDrivesTheDensity() {
        var density = 0f
        var fontScale = 0f

        @Composable
        fun Probe() {
            density = LocalDensity.current.density
            fontScale = LocalDensity.current.fontScale
        }
        rule.setContent { PiliDensity { Probe() } }
        rule.waitForIdle()
        assertEquals(1280f / 1920f, density, 0.001f)
        assertEquals(1f, fontScale, 0f)
    }
}
