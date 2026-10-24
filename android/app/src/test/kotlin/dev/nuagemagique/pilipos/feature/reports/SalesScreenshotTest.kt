package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.scaffold.DeviceLights
import dev.nuagemagique.pilipos.ui.scaffold.FakeProbe
import dev.nuagemagique.pilipos.ui.scaffold.HeaderActions
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.HeaderUiState
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffoldContent
import dev.nuagemagique.pilipos.ui.scaffold.StatusLights
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Goldens of the sales tab at 1920 x 1080 dp: a report with all three kinds of period, a custom range, an empty period. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SalesScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.REPORTS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)
    private val base = ReportsState(signedIn = true, tab = ReportsTab.SALES)

    private fun capture(
        sales: SalesState,
        name: String,
    ) {
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    PosScaffoldContent(
                        header = header,
                        actions = HeaderActions(),
                        fdm = FdmUiState(connected = true),
                        onCheckFdm = {},
                        training = false,
                        clock = "18:30",
                        lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = false) },
                        overlay = {},
                    ) { ReportsContent(base, ReportsCallbacks(), sales = sales) }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/reports/$name.png")
    }

    private val shown =
        SalesState(from = "2026-10-16", to = "2026-10-17", report = salesReport(from = "2026-10-16"), businessDate = "2026-10-17")

    @Test
    fun salesWithTheConcordanceOfEveryPeriod() {
        capture(shown, "reports_sales")
    }

    @Test
    fun salesCustomRange() {
        val custom = shown.copy(preset = SalesPreset.CUSTOM, fromDigits = "01102026", toDigits = "1510", field = SalesField.TO)
        capture(custom, "reports_sales_custom")
    }

    @Test
    fun salesOfAnEmptyPeriod() {
        val empty =
            salesReport(days = emptyList()).copy(
                byProduct = emptyList(),
                byHour = emptyList(),
                ticketCount = 0L,
                totalCents = 0L,
                refundsCents = 0L,
                priceChangesCents = 0L,
                roundingCents = 0L,
            )
        capture(shown.copy(report = empty), "reports_sales_empty")
    }
}
