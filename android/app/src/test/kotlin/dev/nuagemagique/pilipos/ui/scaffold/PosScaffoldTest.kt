package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.FdmSeverity

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class PosScaffoldTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val blocking = FdmUiState(connected = false, severity = FdmSeverity.BLOCKING, code = "FDM_NOT_OPERATIONAL")
    private val calls = mutableListOf<String>()

    private val actions =
        HeaderActions(
            onTables = { calls += "tables" },
            onNewTakeaway = { calls += "takeaway" },
            onOnlineOrders = { calls += "online" },
            onReports = { calls += "reports" },
            onSettings = { calls += "settings" },
            onLock = { calls += "lock" },
            onOpenPin = { calls += "pin" },
        )

    private fun show(
        area: HeaderArea,
        fdm: FdmUiState = blocking,
        header: HeaderUiState = HeaderUiState(area = area, userName = "Alice", roleKey = "role_admin", isManager = true),
        training: Boolean = false,
        overlay: @Composable BoxScope.() -> Unit = {},
    ) {
        compose.setContent {
            PiliTheme(training = training) {
                ProvideUiText(AppLanguage.FR, catalog) {
                    PosScaffoldContent(
                        header = header,
                        actions = actions,
                        fdm = fdm,
                        onCheckFdm = { calls += "check-fdm" },
                        training = training,
                        clock = "14:05",
                        overlay = overlay,
                    ) {
                        Box(Modifier.fillMaxSize().testTag("screen_content")) { Text("contenu") }
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    // ── The FDM veil (PD209) ──────────────────────────────────────

    private fun assertVeil(
        area: HeaderArea,
        shown: Boolean,
    ) {
        show(area)
        if (shown) {
            compose.onNodeWithTag("fdm_veil").assertIsDisplayed()
        } else {
            compose.onNodeWithTag("fdm_veil").assertDoesNotExist()
        }
        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("screen_content").assertExists()
    }

    @Test
    fun theVeilCoversTheTablesWhileTheFdmIsBlocking() {
        assertEquals(true, HeaderArea.TABLES.isSales)
        assertVeil(HeaderArea.TABLES, shown = true)
    }

    @Test
    fun theVeilCoversTheOrderScreenWhileTheFdmIsBlocking() {
        assertEquals(true, HeaderArea.ORDER.isSales)
        assertVeil(HeaderArea.ORDER, shown = true)
    }

    @Test
    fun theVeilCoversTheCheckoutWhileTheFdmIsBlocking() {
        assertEquals(true, HeaderArea.CHECKOUT.isSales)
        assertVeil(HeaderArea.CHECKOUT, shown = true)
    }

    @Test
    fun theVeilIsNotDrawnOnReports() {
        assertEquals(false, HeaderArea.REPORTS.isSales)
        assertVeil(HeaderArea.REPORTS, shown = false)
    }

    @Test
    fun theVeilIsNotDrawnOnSettings() {
        assertEquals(false, HeaderArea.SETTINGS.isSales)
        assertVeil(HeaderArea.SETTINGS, shown = false)
    }

    @Test
    fun theVeilIsNotDrawnOnTheOnlineOrdersBoard() {
        assertEquals(false, HeaderArea.WEBSHOP.isSales)
        assertVeil(HeaderArea.WEBSHOP, shown = false)
    }

    @Test
    fun noVeilOnAWarningReading() {
        show(HeaderArea.TABLES, fdm = FdmUiState(connected = true, severity = FdmSeverity.WARNING))

        compose.onNodeWithTag("fdm_veil").assertDoesNotExist()
        compose.onNodeWithTag("screen_content").assertIsDisplayed()
    }

    @Test
    fun noVeilOnAHealthyReading() {
        show(HeaderArea.ORDER, fdm = FdmUiState(connected = true, severity = FdmSeverity.INFO))

        compose.onNodeWithTag("fdm_veil").assertDoesNotExist()
        compose.onNodeWithTag("screen_content").assertIsDisplayed()
    }

    @Test
    fun theRetryButtonOfTheVeilCallsTheCheck() {
        show(HeaderArea.ORDER)

        compose.onNodeWithTag("fdm_veil_retry").performClick()

        assertEquals(listOf("check-fdm"), calls)
    }

    @Test
    fun theHeaderStaysUsableUnderTheVeilSoReportsAreReachable() {
        show(HeaderArea.TABLES)

        compose.onNodeWithTag("fdm_veil").assertIsDisplayed()
        compose.onNodeWithTag("header_nav_reports").performClick()
        compose.onNodeWithTag("header_lock").performClick()

        assertEquals(listOf("reports", "lock"), calls)
    }

    // ── Frame ─────────────────────────────────────────────────────

    @Test
    fun theHeaderTheContentAndTheOverlayAreDrawn() {
        show(
            HeaderArea.TABLES,
            fdm = FdmUiState(connected = true),
            overlay = { Box(Modifier.testTag("screen_overlay")) { Text("dialogue") } },
        )

        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("screen_content").assertIsDisplayed()
        compose.onNodeWithTag("screen_overlay").assertIsDisplayed()
        compose.onNodeWithTag("header_clock").assertIsDisplayed()
    }

    @Test
    fun noTrainingBannerInTheStandardMode() {
        show(HeaderArea.TABLES, fdm = FdmUiState(connected = true))
        compose.onNodeWithTag("training_banner").assertDoesNotExist()
    }

    @Test
    fun inTrainingModeTheBannerSitsBetweenTheHeaderAndTheContent() {
        show(HeaderArea.TABLES, fdm = FdmUiState(connected = true), training = true)
        compose.onNodeWithTag("training_banner").assertIsDisplayed()
        compose.onNodeWithTag("screen_content").assertIsDisplayed()
    }

    @Test
    fun theOwnPinDialogOpensFromTheHeaderStateAndSitsAboveTheContent() {
        show(
            HeaderArea.TABLES,
            fdm = FdmUiState(connected = true),
            header = HeaderUiState(area = HeaderArea.TABLES, userName = "Alice", roleKey = "role_admin", pinOpen = true),
        )

        compose.onNodeWithTag("pili_dialog").assertIsDisplayed()
        compose.onNodeWithTag("pin_pad").assertIsDisplayed()
    }
}
