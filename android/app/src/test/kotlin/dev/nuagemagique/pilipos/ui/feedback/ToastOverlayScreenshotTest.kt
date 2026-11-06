package dev.nuagemagique.pilipos.ui.feedback

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
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

/** Golden of the toast variants stacked at the top centre, under the header (the newest three). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class ToastOverlayScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun toastVariants() {
        val toasts =
            listOf(
                Toast(1L, "Ticket imprime", ToastVariant.Success),
                Toast(2L, "Imprimante non configuree", ToastVariant.Warning),
                Toast(3L, "Nouvelle commande en ligne", ToastVariant.Info),
                Toast(4L, "Erreur inattendue. Contactez votre technicien. (order_no_lines)", ToastVariant.Error),
            )
        rule.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                    ToastStack(toasts, onDismiss = {}, tapToCloseText = "Touchez pour fermer")
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/toast_variants.png")
    }

    /** The toast after a table move: the message and its "Annuler" button. */
    @Test
    fun toastWithAction() {
        val toasts = listOf(Toast(1L, "Table 4 déplacée en 9", ToastVariant.Success, ToastAction("Annuler") {}))
        rule.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                    ToastStack(toasts, onDismiss = {}, tapToCloseText = "Touchez pour fermer")
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/toast_action.png")
    }
}
