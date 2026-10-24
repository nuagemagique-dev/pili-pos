package dev.nuagemagique.pilipos.ui.eft

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.EFT_CANCELLED
import dev.nuagemagique.pilipos.ui.state.EFT_DECLINED
import dev.nuagemagique.pilipos.ui.state.EFT_UNKNOWN
import dev.nuagemagique.pilipos.ui.state.EftOverlayState
import dev.nuagemagique.pilipos.ui.state.EftPhase
import dev.nuagemagique.pilipos.ui.state.eftPayment
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Goldens of the blocking overlay of the card terminal: waiting, declined, cancelled, unknown and checking. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class EftOverlayScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private fun capture(
        state: EftOverlayState,
        name: String,
    ) {
        rule.mainClock.autoAdvance = false
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                        Text("Ecran en dessous", color = PiliTheme.colors.textPrimary)
                        EftOverlay(state, EftOverlayCallbacks(onCancel = {}, onCheck = {}, onDismiss = {}))
                    }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/eft/$name.png")
    }

    @Test
    fun waiting() = capture(EftOverlayState(phase = EftPhase.WAITING, amountCents = 1_152L, part = 2, parts = 3), "eft_waiting")

    @Test
    fun checking() =
        capture(
            EftOverlayState(phase = EftPhase.CHECKING, amountCents = 1_152L, payment = eftPayment(5, EFT_UNKNOWN, 1_152L)),
            "eft_checking",
        )

    @Test
    fun declined() =
        capture(
            EftOverlayState(
                phase = EftPhase.RESULT,
                amountCents = 1_152L,
                payment = eftPayment(5, EFT_DECLINED, 1_152L, brand = "Visa", detail = "Carte refusée"),
            ),
            "eft_declined",
        )

    @Test
    fun cancelled() =
        capture(
            EftOverlayState(phase = EftPhase.RESULT, amountCents = 1_152L, payment = eftPayment(5, EFT_CANCELLED, 1_152L)),
            "eft_cancelled",
        )

    @Test
    fun unknown() =
        capture(
            EftOverlayState(phase = EftPhase.RESULT, amountCents = 1_152L, payment = eftPayment(5, EFT_UNKNOWN, 1_152L)),
            "eft_unknown",
        )
}
