package dev.nuagemagique.pilipos.feature.settings.vouchers

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.feature.settings.SettingsCallbacks
import dev.nuagemagique.pilipos.feature.settings.SettingsContent
import dev.nuagemagique.pilipos.feature.settings.managerState
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
import uniffi.pos_ffi.GiftVoucherSettings

/** Goldens of the gift voucher panel in the settings screen at 1920 x 1080 dp: off, and on with the saved message. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class GiftVoucherScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.SETTINGS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private fun capture(
        name: String,
        vouchers: GiftVoucherUiState,
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
                        clock = "12:30",
                        lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = true) },
                        content = {
                            SettingsContent(
                                managerState().copy(info = null, deliveries = emptyList(), rows = emptyList()),
                                SettingsCallbacks(),
                                coursesSection = { GiftVoucherSection(vouchers, GiftVoucherCallbacks()) },
                            )
                        },
                    )
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/settings/$name.png")
    }

    @Test
    fun off() {
        val off = GiftVoucherSettings(enabled = false, validityMonths = 12, maxAmountCents = 50_000L)
        capture("gift_vouchers_off", GiftVoucherUiState(manager = true, stored = off))
    }

    @Test
    fun onAndSaved() {
        val on = GiftVoucherSettings(enabled = true, validityMonths = 24, maxAmountCents = 25_000L)
        capture(
            "gift_vouchers_on",
            GiftVoucherUiState(manager = true, stored = on, enabled = true, validity = "24", maxEuros = "250", saved = true),
        )
    }
}
