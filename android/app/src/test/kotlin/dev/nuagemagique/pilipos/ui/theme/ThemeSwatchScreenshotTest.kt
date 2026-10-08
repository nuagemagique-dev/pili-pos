package dev.nuagemagique.pilipos.ui.theme

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.unit.dp
import com.github.takahirom.roborazzi.captureRoboImage
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Goldens of the colour swatches and the type scale, Standard and Training. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class ThemeSwatchScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun standardSwatch() {
        rule.setContent { PiliTheme { Swatch() } }
        rule.onRoot().captureRoboImage("src/test/screenshots/theme/theme_standard.png")
    }

    @Test
    fun trainingSwatch() {
        rule.setContent { PiliTheme(training = true) { Swatch() } }
        rule.onRoot().captureRoboImage("src/test/screenshots/theme/theme_training.png")
    }
}

@Composable
private fun Swatch() {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val named: List<Pair<String, Color>> =
        listOf(
            "bgPrimary" to c.bgPrimary,
            "bgSecondary" to c.bgSecondary,
            "bgSurface" to c.bgSurface,
            "bgPressed" to c.bgPressed,
            "border" to c.border,
            "textPrimary" to c.textPrimary,
            "textSecondary" to c.textSecondary,
            "textOnAccent" to c.textOnAccent,
            "selectedBg" to c.selectedBg,
            "selectedFg" to c.selectedFg,
            "accentPrimary" to c.accentPrimary,
            "accentSuccess" to c.accentSuccess,
            "accentWarning" to c.accentWarning,
            "accentDanger" to c.accentDanger,
            "accentInfo" to c.accentInfo,
            "accentTraining" to c.accentTraining,
        )
    Column(
        Modifier.fillMaxSize().background(c.bgPrimary).padding(d.spaceMd),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        named.chunked(8).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
                row.forEach { (name, color) ->
                    Column(Modifier.width(104.dp)) {
                        Box(
                            Modifier
                                .width(104.dp)
                                .height(48.dp)
                                .background(color, RoundedCornerShape(d.radiusMd)),
                        )
                        Text(name, style = t.body.copy(fontSize = t.sizeXs), color = c.textSecondary)
                    }
                }
            }
        }
        TypeScale()
        ButtonRow()
    }
}

@Composable
private fun TypeScale() {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Column {
        Row(
            Modifier.background(c.bgSecondary, RoundedCornerShape(d.radiusLg)).padding(d.spaceMd),
            horizontalArrangement = Arrangement.spacedBy(d.spaceLg),
            verticalAlignment = Alignment.Bottom,
        ) {
            Text("Titre 28", style = t.title, color = c.textPrimary)
            Text("Texte 16", style = t.body, color = c.textPrimary)
            Text("Libelle 14", style = t.label, color = c.textSecondary)
            Text("12,50 EUR", style = t.price, color = c.accentSuccess)
            Text("0123456789", style = t.figure, color = c.accentInfo)
        }
    }
}

@Composable
private fun ButtonRow() {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Column {
        Row(horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            Box(Modifier.background(c.accentPrimary, RoundedCornerShape(d.radiusMd)).padding(d.spaceMd)) {
                Text("PAYER", style = t.label, color = c.textOnAccent)
            }
            Box(Modifier.background(c.selectedBg, RoundedCornerShape(d.radiusMd)).padding(d.spaceMd)) {
                Text("Selection", style = t.label, color = c.selectedFg)
            }
            Box(Modifier.background(c.bgSurface, RoundedCornerShape(d.radiusMd)).padding(d.spaceMd)) {
                Text("Secondaire", style = t.label, color = c.textPrimary)
            }
        }
    }
}
