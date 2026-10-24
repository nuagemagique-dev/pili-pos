package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.runtime.Composable
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.QrMatrices
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
import io.mockk.coEvery
import io.mockk.mockk
import kotlinx.coroutines.Dispatchers
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

private const val GOLDEN_LINK = "https://manager.hubrise.com/oauth2/v1/authorize?client_id=pili&scope=location.orders"

/**
 * Goldens of the settings screen at 1920 x 1080 dp: the main view, the employees with a confirmation, the HubRise panel
 * and the connect dialog.
 */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.SETTINGS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    /** A QR-like matrix, the same for every capture, so the golden does not depend on the engine. */
    private val matrix: List<List<Boolean>> =
        List(29) { row -> List(29) { column -> (row * 7 + column * 11 + row * column) % 5 < 2 } }

    private fun capture(
        name: String,
        overlay: @Composable () -> Unit = {},
        content: @Composable () -> Unit,
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
                        content = content,
                    )
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/settings/$name.png")
    }

    private val log = listOf(switchChange(true), switchChange(false, source = "config", by = null, at = "2026-10-08 16:20:00"))

    @Test
    fun main() {
        capture("settings_main") { SettingsContent(managerState(), SettingsCallbacks()) }
    }

    @Test
    fun badges() {
        // The unfolded reader panel: USB reader, badge login on, the gap being edited on the pad, the log.
        val state =
            managerState(
                badge =
                    badgePanel(
                        reader = "usb",
                        login = true,
                        hasNfc = true,
                        typed = BadgeTimingDraft("80", "6", "32"),
                        editing = TimingField.Gap,
                    ),
            )
        capture("settings_badges") { SettingsContent(state, SettingsCallbacks()) }
    }

    @Test
    fun badgeEnrol() {
        val state =
            managerState(badge = badgePanel(reader = "usb", open = false))
                .copy(enrol = BadgeEnrolFlow(WAITER_INSZ, "Chen"), openShifts = mapOf(CASHIER_INSZ to "2026-10-17T10:02:00Z"))
        capture(
            "settings_badge_enrol",
            overlay = { SettingsDialogs(state, SettingsPinActions(), DeactivateActions(), BadgeDialogActions()) },
        ) {
            SettingsContent(state, SettingsCallbacks())
        }
    }

    @Test
    fun users() {
        val state =
            managerState().copy(
                // The manager's view of presence: one employee on shift, one presence-only kitchen employee.
                users = listOf(admin, cashier, waiter, cook),
                openShifts = mapOf(CASHIER_INSZ to "2026-10-17T10:02:00Z"),
                confirmDeactivate = CASHIER_INSZ,
                // A reader is on: each row says if it holds a badge and offers its button.
                badge = badgePanel(reader = "usb", open = false),
                form =
                    NewEmployeeForm(
                        name = "Eve",
                        insz = "9904",
                        error = catalog.text("settings_err_11_digits_required", AppLanguage.FR),
                    ),
            )
        capture("settings_users", overlay = { SettingsDialogs(state, SettingsPinActions(), DeactivateActions()) }) {
            SettingsContent(state, SettingsCallbacks())
        }
    }

    @Test
    fun hubrise() {
        val state = managerState(hubrise = hubriseOn(open = 1L, log = log)).copy(info = null, deliveries = emptyList())
        capture("settings_hubrise") {
            SettingsContent(state, SettingsCallbacks(), hubriseSection = {
                HubriseSection(
                    connectedState(adoptions = listOf(adoption(1L, "Sushi saumon"), adoption(2L, "Maki thon"))),
                    2,
                    HubriseCallbacks(),
                )
            })
        }
    }

    @Test
    fun hubriseConnect() {
        val natives = mockk<EngineNatives>()
        coEvery { natives.qrMatrix(GOLDEN_LINK) } returns matrix
        val matrices = QrMatrices(EngineHelpers(natives, Dispatchers.Unconfined))
        val state = managerState(hubrise = hubriseOn()).copy(info = null)
        val dialog = HubriseUiState(active = true, isManager = true, status = connection(false), authorizeUrl = GOLDEN_LINK)
        capture(
            "settings_hubrise_connect",
            overlay = { HubriseDialogs(dialog, matrices, HubriseConnectActions(), {}, {}) },
        ) {
            SettingsContent(state, SettingsCallbacks(), hubriseSection = {
                HubriseSection(HubriseUiState(active = true, isManager = true, status = connection(false)), 0, HubriseCallbacks())
            })
        }
    }
}
