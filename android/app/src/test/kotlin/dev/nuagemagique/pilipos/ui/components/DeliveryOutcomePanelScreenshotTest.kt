package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.dp
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
import uniffi.pos_ffi.EmailStatus

/** Golden of the delivery outcome: e-mail queued, and the paper original still to print. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class DeliveryOutcomePanelScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun queuedWithThePrintAction() {
        val catalog = I18nTestSupport.catalog()
        val outcome = deliveryOutcome(email = EmailStatus.QUEUED, needsPaper = true)
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.testTag("golden_frame").background(PiliTheme.colors.bgSecondary).padding(16.dp).width(560.dp)) {
                        DeliveryOutcomePanel(
                            outcome = outcome,
                            texts = defaultDeliveryOutcomeTexts(outcome),
                            canPrintMissing = true,
                            onPrintMissing = { true },
                        )
                    }
                }
            }
        }
        rule.onNodeWithTag("golden_frame").captureRoboImage("src/test/screenshots/components/delivery_outcome.png")
    }
}
