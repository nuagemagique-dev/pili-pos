package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.FdmSeverity

/** Goldens of the staff header and of the scaffold in the training palette, at 1920 x 1080 dp. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class ScaffoldScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private val orderHeader =
        HeaderUiState(
            area = HeaderArea.ORDER,
            userName = "Alice Dupont",
            roleKey = "role_admin",
            isManager = true,
            tableNumber = 12,
            dineInOrder = true,
            hubriseVisible = true,
            pendingOrders = 3,
        )

    private fun probe() =
        FakeProbe().apply {
            kitchenAnswer = { false }
        }

    @Test
    fun headerStandard() {
        val lights = DeviceLights(probe())
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    AppHeader(orderHeader, HeaderActions(), clock = "14:05") {
                        StatusLights(FdmUiState(connected = true, severity = FdmSeverity.INFO), lights, hubriseVisible = true)
                    }
                }
            }
        }
        rule.waitForIdle()
        rule.onNodeWithTag("app_header").captureRoboImage("src/test/screenshots/components/header_standard.png")
    }

    @Test
    fun statusLights() {
        val lights = DeviceLights(probe().apply { hubriseAnswer = { quietHubrise().copy(staleOrders = 2) } })
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.background(PiliTheme.colors.bgSecondary).padding(PiliTheme.dimens.spaceMd)) {
                        StatusLights(FdmUiState(connected = true, severity = FdmSeverity.WARNING), lights, hubriseVisible = true)
                    }
                }
            }
        }
        rule.waitForIdle()
        rule.onNodeWithTag("status_lights").captureRoboImage("src/test/screenshots/components/status_lights.png")
    }

    @Test
    fun scaffoldTraining() {
        val lights = DeviceLights(probe())
        rule.setContent {
            PiliTheme(training = true) {
                ProvideUiText(AppLanguage.FR, catalog) {
                    PosScaffoldContent(
                        header = orderHeader.copy(area = HeaderArea.TABLES, tableNumber = null, dineInOrder = false),
                        actions = HeaderActions(),
                        fdm = FdmUiState(connected = true),
                        onCheckFdm = {},
                        training = true,
                        clock = "14:05",
                        lights = { StatusLights(FdmUiState(connected = true), lights, hubriseVisible = true) },
                    ) {
                        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            Text("contenu de l'écran", color = PiliTheme.colors.textPrimary)
                        }
                    }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/components/scaffold_training.png")
    }
}
