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

/** Goldens of the PIN pad: entering a PIN, and the error line after a refusal. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w420dp-h700dp-mdpi")
class PinPadScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val texts =
        PinPadTexts(
            title = "Changer le PIN",
            hint = "Confirmez avec votre PIN de gerant",
            enterPrompt = "Saisir le PIN",
            confirmPrompt = "Confirmer le PIN",
            mismatch = "Les PIN ne correspondent pas",
            cancel = "Retour",
            clearLabel = "Effacer",
            backspaceLabel = "Corriger",
        )

    private fun capture(
        state: PinPadState,
        name: String,
    ) {
        rule.setContent {
            PiliTheme {
                Box(Modifier.background(PiliTheme.colors.bgSecondary).padding(PiliTheme.dimens.spaceLg)) {
                    PinPad(state = state, texts = texts, onKey = {}, onCancel = {})
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/$name.png")
    }

    @Test
    fun pinPadEnter() {
        val state = PinPadState(PinMode.Verify)
        listOf('4', '2').forEach { state.apply(NumPadKey.Digit(it)) }
        capture(state, "pinpad_enter")
    }

    @Test
    fun pinPadError() {
        val state = PinPadState(PinMode.Choose, texts.mismatch)
        "12341235".forEach { state.apply(NumPadKey.Digit(it)) }
        state.apply(NumPadKey.Digit('9'))
        capture(state, "pinpad_error")
    }
}
