package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Goldens of the in-app keypads that replace the system keyboard (C13 / R6). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class KeypadsScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private fun show(content: @Composable () -> Unit) {
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) { content() }
                }
            }
        }
    }

    @Test
    fun textKeypadWithChipsAndCounter() {
        show {
            TextKeypad(
                spec =
                    TextKeypadSpec(
                        tag = "note",
                        title = "Note pour la cuisine",
                        placeholder = "Ex. : sans wasabi",
                        maxLen = 60,
                        chips = listOf("Sans wasabi", "Sans sésame", "Bien cuit"),
                        chipAppend = { note, chip -> if (note.isEmpty()) chip else "$note, $chip" },
                        symbols = KeypadSymbols.Name,
                        space = true,
                        caseToggle = true,
                        hint = "Imprimée en cuisine avec la commande",
                        showCounter = true,
                    ),
                onConfirm = {},
                onCancel = {},
            )
        }
        rule.typeOnKeypad("Sans wasabi", "note")
        rule.onNodeWithTag("note_dialog").captureRoboImage("src/test/screenshots/components/text_keypad.png")
    }

    @Test
    fun textKeyboardOnAnInvoiceLine() {
        show {
            TextKeyboardDialog(
                TextKeyboardSpec(tag = "keyboard", title = "Nom du client", placeholder = "", maxLength = 80, caseToggle = true),
                onConfirm = {},
                onCancel = {},
            )
        }
        rule.typeOnKeyboard("ACME SPRL", "keyboard")
        rule.onNodeWithTag("keyboard_dialog").captureRoboImage("src/test/screenshots/components/text_keyboard.png")
    }

    @Test
    fun numberKeypadForAnInsz() {
        show {
            NumberKeypad(
                NumberKeypadSpec(tag = "field_number", title = "INSZ · 11 chiffres", maxDigits = 11, exactDigits = 11),
                onConfirm = {},
                onCancel = {},
            )
        }
        rule.typeOnNumPad("9904040")
        rule.onNodeWithTag("field_number_dialog").captureRoboImage("src/test/screenshots/components/number_keypad.png")
    }
}
