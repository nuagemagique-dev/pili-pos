package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performScrollTo
import dev.nuagemagique.pilipos.feature.settings.alerts.OwnerAlertsCallbacks
import dev.nuagemagique.pilipos.feature.settings.alerts.OwnerAlertsSection
import dev.nuagemagique.pilipos.feature.settings.alerts.OwnerAlertsUiState
import dev.nuagemagique.pilipos.feature.settings.alerts.ownerAlerts
import dev.nuagemagique.pilipos.feature.settings.alerts.toDraft
import dev.nuagemagique.pilipos.feature.settings.courses.CoursesCallbacks
import dev.nuagemagique.pilipos.feature.settings.courses.CoursesUiState
import dev.nuagemagique.pilipos.feature.settings.vouchers.GiftVoucherCallbacks
import dev.nuagemagique.pilipos.feature.settings.vouchers.GiftVoucherUiState
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.GiftVoucherSettings

/**
 * The alerts, courses and gift voucher panels of the settings screen are drawn for a manager only, in the wide
 * and in the narrow layout (`SettingsView.vue`: `v-if="auth.isManager"`).
 */
@RunWith(RobolectricTestRunner::class)
abstract class ManagerPanelsRules {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private val alerts =
        ownerAlerts().let { OwnerAlertsUiState(manager = true, settings = it, draft = it.toDraft()) }
    private val courses =
        CoursesUiState(
            manager = true,
            categories = listOf(Category(id = 1, name = "Sushi", nameFr = null, nameZh = null, sortOrder = 1, color = null, active = true)),
            stored = CourseSettings(enabled = false, categoryCourses = emptyList()),
        )
    private val vouchers =
        GiftVoucherSettings(enabled = false, validityMonths = 12, maxAmountCents = 50_000L).let {
            GiftVoucherUiState(manager = true, stored = it, enabled = it.enabled, validity = "${it.validityMonths}")
        }

    private val noop =
        SettingsCallbacks(
            onToggleTraining = {},
            onLanguage = {},
            onProductLabel = {},
            onTablePlanInput = {},
            onSaveTablePlan = {},
            onRow = {},
            onNewName = {},
            onNewInsz = {},
            onNewRole = {},
            onAddEmployee = {},
            onPin = {},
            onDeactivate = {},
            onToggleHubrise = {},
            onReloadDeliveries = {},
        )

    private fun show(state: SettingsUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    SettingsContent(
                        state,
                        noop,
                        alertsSection = { OwnerAlertsSection(alerts, OwnerAlertsCallbacks()) },
                        coursesSection = {
                            ManagerPanels(courses, CoursesCallbacks({}, { _, _ -> }, {}), vouchers, GiftVoucherCallbacks({}, {}, {}, {}))
                        },
                    )
                }
            }
        }
        compose.waitForIdle()
    }

    private val panels = listOf("alerts_section", "courses_section", "gv_settings")

    @Test
    fun aNonManagerGetsNoneOfTheThreePanels() {
        show(workerState())

        panels.forEach { compose.onNodeWithTag(it).assertDoesNotExist() }
        compose.onNodeWithTag("settings_rows").assertIsDisplayed()
    }

    @Test
    fun aManagerGetsAllThree() {
        show(managerState())

        panels.forEach { compose.onNodeWithTag(it).performScrollTo().assertIsDisplayed() }
    }

    protected fun top(tag: String): Float = compose.onNodeWithTag(tag).fetchSemanticsNode().positionInRoot.y

    protected fun showManager() = show(managerState())
}

@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsManagerPanelsWideTest : ManagerPanelsRules() {
    @Test
    fun theEmployeesStayBesideThePanelsInTheSameColumn() {
        showManager()

        compose.onNodeWithTag("settings_workers").assertIsDisplayed()
        compose.onNodeWithTag("courses_section").assertIsDisplayed()
        // The employees come first, the courses and vouchers under them in the same column.
        assertTrue(top("settings_workers") < top("courses_section"))
        assertTrue(top("courses_section") < top("gv_settings"))
    }
}

@Config(qualifiers = "w800dp-h1280dp-mdpi")
class SettingsManagerPanelsNarrowTest : ManagerPanelsRules()
