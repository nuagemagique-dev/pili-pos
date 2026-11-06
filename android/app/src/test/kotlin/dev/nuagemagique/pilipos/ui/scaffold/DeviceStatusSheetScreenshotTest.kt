package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
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
import java.time.Instant

/** Golden of the status sheet of a light (UX audit L2): a printer that does not answer. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class DeviceStatusSheetScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    @Test
    fun printerDown() {
        val device =
            DeviceStatus(
                key = "ticket",
                name = "Imprimante ticket",
                state = LightState.ERR,
                reason = "L'imprimante ne répond pas. Vérifiez qu'elle est allumée et sur le réseau.",
                checkedAt = Instant.parse("2026-10-09T10:05:09Z"),
            )
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                        DeviceStatusSheet(device, busy = false, onRetry = {}, onClose = {})
                    }
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/device_status_sheet.png")
    }
}
