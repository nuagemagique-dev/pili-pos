package dev.nuagemagique.pilipos.ui.fdm

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
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
import uniffi.pos_ffi.FdmSeverity
import java.time.Clock
import java.time.Instant
import java.time.ZoneOffset

/** Goldens of the FDM banner (optional with its countdown, mandatory) and of the blocking veil. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class FdmScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private fun banner(state: FdmUiState): Unit =
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) { FdmBanner(state) }
                }
            }
        }

    @Test
    fun bannerWarning() {
        banner(FdmUiState(connected = true, severity = FdmSeverity.WARNING, code = "CLIENT_CERT_NEAR_EXPIRATION"))
        rule.onNodeWithTag("fdm_banner").captureRoboImage("src/test/screenshots/components/fdm_banner_warning.png")
    }

    @Test
    fun bannerMandatory() {
        banner(FdmUiState(connected = true, severity = FdmSeverity.WARNING, code = "BUFFER_NEAR_FULL", mandatory = true))
        rule.onNodeWithTag("fdm_banner").captureRoboImage("src/test/screenshots/components/fdm_banner_mandatory.png")
    }

    @Test
    fun veil() {
        val now = Instant.parse("2026-10-09T10:00:00Z")
        val clock = Clock.fixed(now, ZoneOffset.UTC)
        val state =
            FdmUiState(
                connected = false,
                severity = FdmSeverity.BLOCKING,
                code = "FDM_NOT_OPERATIONAL",
                nextCheckAt = now.plusSeconds(24),
            )
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
                        FdmBlockingVeil(state, onCheckNow = {}, clock = clock)
                    }
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/fdm_veil.png")
    }
}
