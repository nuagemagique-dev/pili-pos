package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.BadgeLogEntry
import uniffi.pos_ffi.BadgeOverview
import uniffi.pos_ffi.User

/** The badge reader panel and the badge parts of the employee rows (`BadgeSettings.vue`, `SettingsView.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsBadgeContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val taps = mutableListOf<String>()

    private val callbacks =
        SettingsCallbacks(
            badge =
                BadgeCallbacks(
                    onToggle = { taps += "toggle" },
                    onReader = { taps += "reader:$it" },
                    onToggleLogin = { taps += "login" },
                    onEditField = { taps += "edit:${it.tag}" },
                    onKey = { taps += "key:$it" },
                    onSaveTiming = { taps += "save" },
                    onEmployee = { taps += "employee:${it.insz}" },
                ),
        )

    private fun text(
        key: String,
        vars: Map<String, Any> = emptyMap(),
    ): String = catalog.text(key, AppLanguage.FR, vars)

    private fun show(state: SettingsUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { SettingsContent(state, callbacks) }
            }
        }
        compose.waitForIdle()
    }

    private fun panel(badge: BadgePanelState) = managerState(badge = badge)

    // ── The panel ─────────────────────────────────────────────────

    @Test
    fun thePanelIsForManagersOnly() {
        show(workerState())

        compose.onNodeWithTag("settings_badges").assertDoesNotExist()
    }

    @Test
    fun thePanelIsFoldedByDefaultAndShowsTheReaderInUse() {
        show(panel(badgePanel(reader = "usb", open = false)))

        compose.onNodeWithTag("settings_badges").assertIsDisplayed()
        compose.onNodeWithTag("badge_state", useUnmergedTree = true).assertTextContains(text("badge_reader_usb"), substring = true)
        compose.onNodeWithTag("badge_reader_usb").assertDoesNotExist()
        compose.onNodeWithTag("badge_toggle").performClick()
        assertEquals(listOf("toggle"), taps)
    }

    @Test
    fun theHeaderNamesTheOffReader() {
        show(panel(badgePanel(reader = "off", open = false)))

        compose.onNodeWithTag("badge_state", useUnmergedTree = true).assertTextContains(text("badge_reader_off"), substring = true)
    }

    @Test
    fun theReaderChoiceMarksTheStoredOneAndCallsBack() {
        show(panel(badgePanel(reader = "usb")))

        compose.onNodeWithTag("badge_reader_usb").assertIsSelected()
        compose.onNodeWithTag("badge_reader_off").performClick()
        assertEquals(listOf("reader:off"), taps)
    }

    @Test
    fun theNfcChoiceNeedsFeatureNfc() {
        show(panel(badgePanel(reader = "usb", hasNfc = false)))
        compose.onNodeWithTag("badge_reader_nfc").assertDoesNotExist()
    }

    @Test
    fun aDeviceWithNfcOffersTheBuiltInReader() {
        show(panel(badgePanel(reader = "usb", hasNfc = true)))

        compose.onNodeWithTag("badge_reader_nfc").assertIsDisplayed()
        compose.onNodeWithTag("badge_reader_nfc").assertTextContains("NFC", substring = true)
        compose.onNodeWithTag("badge_reader_nfc").performClick()
        assertEquals(listOf("reader:nfc"), taps)
    }

    @Test
    fun aStoredNfcChoiceOnADeviceWithoutNfcGetsTheUnavailableNote() {
        show(panel(badgePanel(reader = "nfc", hasNfc = false)))

        compose.onNodeWithTag("badge_nfc_note").assertTextContains(text("settings_badge_nfc_unavailable"))
    }

    @Test
    fun nfcTurnedOffInTheDeviceSettingsGetsItsNote() {
        show(panel(badgePanel(reader = "nfc", hasNfc = true, nfcEnabled = false)))

        compose.onNodeWithTag("badge_nfc_note").assertTextContains("NFC", substring = true)
    }

    @Test
    fun nfcOnAndEnabledHasNoNote() {
        show(panel(badgePanel(reader = "nfc", hasNfc = true, nfcEnabled = true)))

        compose.onNodeWithTag("badge_nfc_note").assertDoesNotExist()
    }

    @Test
    fun theLoginSwitchAppearsWithAReaderAndCallsBack() {
        show(panel(badgePanel(reader = "usb", login = true)))

        compose.onNodeWithTag("badge_login_switch").assertIsDisplayed()
        compose.onNodeWithTag("badge_login_switch").performClick()
        assertEquals(listOf("login"), taps)
    }

    @Test
    fun withTheReaderOffThereIsNoLoginSwitchAndNoTiming() {
        show(panel(badgePanel(reader = "off")))

        compose.onNodeWithTag("badge_login_switch").assertDoesNotExist()
        compose.onNodeWithTag("badge_field_gap").assertDoesNotExist()
    }

    @Test
    fun theUsbTimingShowsTheThreeStoredNumbers() {
        show(panel(badgePanel(reader = "usb")))

        compose.onNodeWithTag("badge_field_gap").assertTextContains("50")
        compose.onNodeWithTag("badge_field_min").assertTextContains("6")
        compose.onNodeWithTag("badge_field_max").assertTextContains("32")
        compose.onNodeWithTag("badge_timing_save").assertDoesNotExist()
    }

    @Test
    fun theNfcReaderHasNoUsbTiming() {
        show(panel(badgePanel(reader = "nfc", hasNfc = true)))

        compose.onNodeWithTag("badge_field_gap").assertDoesNotExist()
    }

    @Test
    fun aFieldTapOpensThePadAndAKeyReachesTheModel() {
        show(panel(badgePanel(reader = "usb", editing = TimingField.Gap)))

        compose.onNodeWithTag("numpad_key_7").performClick()
        compose.onNodeWithTag("badge_field_min").performClick()

        assertEquals(listOf("key:${NumPadKey.Digit('7')}", "edit:min"), taps)
    }

    @Test
    fun theSaveButtonShowsWhenTheTimingChanged() {
        show(panel(badgePanel(reader = "usb", typed = BadgeTimingDraft("80", "6", "32"))))

        compose.onNodeWithTag("badge_field_gap").assertTextContains("80")
        compose.onNodeWithTag("badge_timing_save").performClick()
        assertEquals(listOf("save"), taps)
    }

    @Test
    fun aRefusedSettingShowsItsText() {
        show(panel(badgePanel(error = text("err_badge_settings_invalid"))))

        compose.onNodeWithTag("badge_error").assertTextContains(text("err_badge_settings_invalid"))
    }

    @Test
    fun theRelinkBannerShowsEvenWhenFolded() {
        show(panel(badgePanel(open = false, relink = true)))

        compose.onNodeWithTag("badge_relink").assertTextContains(text("settings_badge_relink"))
    }

    @Test
    fun theLogListsLinksAndSettingsChanges() {
        show(panel(badgePanel(reader = "usb")))

        compose.onNodeWithTag("badge_log_0").assertTextContains(text("badge_log_link"), substring = true)
        compose.onNodeWithTag("badge_log_0").assertTextContains("Bob", substring = true)
        compose.onNodeWithTag("badge_log_1").assertTextContains(text("badge_log_settings"), substring = true)
    }

    @Test
    fun theLogShowsAtMostSixRowsNewestFirst() {
        val rows =
            (0 until 8).map {
                BadgeLogEntry("link", CASHIER_INSZ, "N$it", ADMIN_INSZ, "Alice", "2026-10-17 10:00:00", null)
            }
        val state = panel(badgePanel(reader = "usb"))
        show(state.copy(badge = state.badge.copy(overview = BadgeOverview(listOf(bobBadge), rows))))

        for (i in 0..5) compose.onNodeWithTag("badge_log_$i").assertTextContains("N$i", substring = true)
        compose.onNodeWithTag("badge_log_6").assertDoesNotExist()
        compose.onNodeWithTag("badge_log_7").assertDoesNotExist()
    }

    @Test
    fun aLogRowNamesTheManagerAndTheBrusselsTime() {
        show(panel(badgePanel(reader = "usb")))

        // 10:00 UTC on 17 October is 12:00 in Brussels (CEST).
        compose.onNodeWithTag("badge_log_0").assertTextContains("Alice", substring = true)
        compose.onNodeWithTag("badge_log_0").assertTextContains("17/10 12:00", substring = true)
    }

    @Test
    fun aRevocationRowShowsItsLabelAndTheEmployee() {
        val revoke = BadgeLogEntry("revoke", CASHIER_INSZ, "Bob", ADMIN_INSZ, "Alice", "2026-10-17 11:00:00", "manager")
        val state = panel(badgePanel(reader = "usb"))
        show(state.copy(badge = state.badge.copy(overview = BadgeOverview(emptyList(), listOf(revoke)))))

        compose.onNodeWithTag("badge_log_0").assertTextContains(text("badge_log_revoke"), substring = true)
        compose.onNodeWithTag("badge_log_0").assertTextContains("Bob", substring = true)
    }

    // ── The narrow (handheld) layout ──────────────────────────────

    @Test
    @Config(qualifiers = "w320dp-h470dp-mdpi")
    fun onANarrowWindowAManagerGetsTheBadgePanelBetweenThePlanAndTheEmployees() {
        show(panel(badgePanel(reader = "usb", open = false)))

        compose.onNodeWithTag("settings_badges").assertExists()
        val plan = compose.onNodeWithTag("settings_tables").fetchSemanticsNode().positionInRoot.y
        val badges = compose.onNodeWithTag("settings_badges").fetchSemanticsNode().positionInRoot.y
        val workers = compose.onNodeWithTag("settings_workers").fetchSemanticsNode().positionInRoot.y
        assertTrue("plan $plan, badges $badges", plan < badges)
        assertTrue("badges $badges, workers $workers", badges < workers)
    }

    @Test
    @Config(qualifiers = "w320dp-h470dp-mdpi")
    fun onANarrowWindowAWorkerGetsNoBadgePanel() {
        show(workerState())

        compose.onNodeWithTag("settings_badges").assertDoesNotExist()
    }

    @Test
    fun anEmptyLogSaysSo() {
        show(panel(badgePanel(reader = "usb").copy(overview = null)))

        compose.onNodeWithTag("badge_log_empty").assertTextContains(text("settings_badge_log_empty"))
    }

    // ── The employee rows ─────────────────────────────────────────

    @Test
    fun eachRowSaysIfItHasABadgeAndOffersTheRightButton() {
        show(panel(badgePanel(reader = "usb", open = false)).copy(users = listOf(admin, cashier)))

        compose.onNodeWithTag("worker_${CASHIER_INSZ}_badge")
            .assertTextContains(text("badge_linked_on", mapOf("date" to "17/10/2026")))
        compose.onNodeWithTag("worker_${CASHIER_INSZ}_badge_button").assertTextContains(text("badge_revoke"))
        compose.onNodeWithTag("worker_${ADMIN_INSZ}_badge").assertTextContains(text("badge_none_label"))
        compose.onNodeWithTag("worker_${ADMIN_INSZ}_badge_button").assertTextContains(text("badge_link"))
    }

    @Test
    fun theBadgeButtonCallsBackWithTheEmployee() {
        show(panel(badgePanel(reader = "usb", open = false)).copy(users = listOf(admin, cashier)))

        compose.onNodeWithTag("worker_${CASHIER_INSZ}_badge_button").performSemanticsAction(SemanticsActions.OnClick)
        compose.onNodeWithTag("worker_${ADMIN_INSZ}_badge_button").performSemanticsAction(SemanticsActions.OnClick)

        assertEquals(listOf("employee:$CASHIER_INSZ", "employee:$ADMIN_INSZ"), taps)
    }

    @Test
    fun withTheReaderOffTheRowsHaveNoBadgeState() {
        show(panel(badgePanel(reader = "off", open = false)).copy(users = listOf(admin, cashier)))

        compose.onNodeWithTag("worker_${CASHIER_INSZ}_badge").assertDoesNotExist()
        compose.onNodeWithTag("worker_${CASHIER_INSZ}_badge_button").assertDoesNotExist()
    }

    @Test
    fun aPresenceOnlyEmployeeCanHoldABadgeToo() {
        val cook: User = cook
        show(panel(badgePanel(reader = "usb", open = false)).copy(users = listOf(admin, cook)))

        compose.onNodeWithTag("worker_${COOK_INSZ}_badge_button").assertTextContains(text("badge_link"))
    }
}
