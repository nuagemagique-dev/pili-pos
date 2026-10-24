package dev.nuagemagique.pilipos.feature.settings.alerts

import androidx.compose.runtime.Composable
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

/**
 * Goldens of the owner alerts panel in the settings screen at 1920 x 1080 dp: off with no bot token, on with a
 * token and a log, and the chat id dialog. They are the captures the plan of the owner-features lane lists for the panel.
 */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class OwnerAlertsScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.SETTINGS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private fun capture(
        name: String,
        overlay: @Composable () -> Unit = {},
        alerts: OwnerAlertsUiState,
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
                        overlay = { overlay() },
                        content = {
                            SettingsContent(
                                managerState().copy(info = null, deliveries = emptyList(), rows = emptyList()),
                                SettingsCallbacks(),
                                alertsSection = { OwnerAlertsSection(alerts, OwnerAlertsCallbacks()) },
                            )
                        },
                    )
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/settings/$name.png")
    }

    private fun loaded(
        settings: uniffi.pos_ffi.OwnerAlerts,
        log: List<uniffi.pos_ffi.AlertLogRow> = emptyList(),
    ) = OwnerAlertsUiState(manager = true, settings = settings, draft = settings.toDraft(), log = log)

    @Test
    fun offWithoutToken() {
        capture("alerts_off_no_token", alerts = loaded(ownerAlerts(token = false)))
    }

    @Test
    fun onWithTokenAndLog() {
        val log =
            listOf(
                logRow(3L, "test", "test"),
                logRow(2L, "resolved", "fdm"),
                logRow(1L, "raised", "fdm", "suppressed"),
            )
        capture(
            "alerts_on_with_token",
            alerts = loaded(ownerAlerts(enabled = true, chatId = "-1001234567890", label = "Salle 1"), log).copy(testSent = true),
        )
    }

    @Test
    fun chatIdDialog() {
        val entry = numberEntryFor(AlertNumber.ChatId, "-1001234567890")
        capture(
            "alerts_chat_id_dialog",
            overlay = { NumberEntryDialog(entry, NumberEntryActions()) },
            alerts = loaded(ownerAlerts(enabled = true, chatId = "-1001234567890")),
        )
    }
}
