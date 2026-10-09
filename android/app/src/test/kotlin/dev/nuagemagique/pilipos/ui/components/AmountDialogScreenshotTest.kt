package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
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

/** Golden of the amount dialog with 12,50 EUR typed. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w800dp-h640dp-mdpi")
class AmountDialogScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun amountTyped() {
        val catalog = I18nTestSupport.catalog()
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                        AmountDialog(
                            title = "Prix libre",
                            itemName = "Menu du jour",
                            texts = defaultAmountDialogTexts(),
                            onConfirm = {},
                            onCancel = {},
                        )
                    }
                }
            }
        }
        listOf('1', '2', '5', '0').forEach { rule.onNodeWithTag("numpad_key_$it").performClick() }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/amount_dialog.png")
    }
}
