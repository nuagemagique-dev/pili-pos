package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
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
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

/** Golden of the VAT summary line under a cart. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w480dp-h96dp-mdpi")
class VatSummaryScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun threeRates() {
        val catalog = I18nTestSupport.catalog()
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.background(PiliTheme.colors.bgSecondary).padding(16.dp).width(448.dp)) {
                        VatSummary(
                            listOf(
                                VatSplitRow(VatCode.A, 21, 1210, 210),
                                VatSplitRow(VatCode.B, 12, 11200, 1200),
                                VatSplitRow(VatCode.C, 6, 106, 6),
                            ),
                        )
                    }
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/vat_summary.png")
    }
}
