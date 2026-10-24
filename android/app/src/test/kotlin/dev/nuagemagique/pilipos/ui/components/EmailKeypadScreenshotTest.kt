package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
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

/** Golden of the e-mail keypad with a wrong address typed (red border and error line). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class EmailKeypadScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun addressBeingTyped() {
        val catalog = I18nTestSupport.catalog()
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                        EmailKeypad(texts = defaultEmailKeypadTexts(), onConfirm = {}, onCancel = {})
                    }
                }
            }
        }
        "marie.dupont".forEach { rule.onNodeWithTag("email_key_$it").performClick() }
        rule.onNodeWithTag("email_keypad").captureRoboImage("src/test/screenshots/components/email_keypad.png")
    }
}
