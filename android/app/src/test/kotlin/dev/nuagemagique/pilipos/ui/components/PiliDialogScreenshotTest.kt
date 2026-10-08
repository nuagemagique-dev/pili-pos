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

/** Golden of a confirmation dialog over a screen. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w800dp-h450dp-mdpi")
class PiliDialogScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun confirmDialog() {
        rule.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgSecondary)) {
                        Text("Ecran en dessous", color = PiliTheme.colors.textPrimary)
                    }
                    ConfirmDialog(
                        title = "Annuler la vente",
                        message = "Les lignes de la commande seront perdues. Cette action est definitive.",
                        confirmText = "Annuler la vente",
                        cancelText = "Retour",
                        onConfirm = {},
                        onCancel = {},
                        destructive = true,
                    )
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/dialog_confirm.png")
    }
}
