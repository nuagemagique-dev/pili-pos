package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
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

/** Golden of the four button variants, enabled and disabled. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w560dp-h200dp-mdpi")
class PiliButtonScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun buttonVariants() {
        rule.setContent {
            PiliTheme {
                Column(
                    Modifier.background(PiliTheme.colors.bgSecondary).padding(PiliTheme.dimens.spaceMd),
                    verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
                ) {
                    listOf(true, false).forEach { enabled ->
                        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                            PiliButton("PAYER", {}, variant = PiliButtonVariant.Primary, enabled = enabled)
                            PiliButton("Retour", {}, variant = PiliButtonVariant.Secondary, enabled = enabled)
                            PiliButton("Supprimer", {}, variant = PiliButtonVariant.Danger, enabled = enabled)
                            PiliButton("Plus tard", {}, variant = PiliButtonVariant.Ghost, enabled = enabled)
                        }
                    }
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/button_variants.png")
    }
}
