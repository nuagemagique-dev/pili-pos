package dev.nuagemagique.pilipos.shell

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.feature.login.LoginEntry
import dev.nuagemagique.pilipos.nav.AppNavHost
import dev.nuagemagique.pilipos.nav.registryOf
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class ShellContentTest {
    @get:Rule
    val compose = createComposeRule()

    private var inboxClicks = 0

    private fun show(
        state: ShellState,
        withInbox: Boolean = true,
    ) {
        compose.setContent {
            PiliTheme {
                ShellContent(
                    state = state,
                    onImportTree = {},
                    onImportInbox = if (withInbox) ({ inboxClicks++ }) else null,
                ) {
                    AppNavHost(registry = registryOf(setOf(LoginEntry())))
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun needsDeviceOwnerShowsTheOwnerScreenWithTheAdbCommandInDebug() {
        show(ShellState.NeedsDeviceOwner)
        compose.onNodeWithTag("shell_owner_required").assertIsDisplayed()
        assertTrue(BuildConfig.DEBUG)
        compose
            .onNodeWithTag("shell_owner_adb")
            .assertTextContains("adb shell dpm set-device-owner ", substring = true)
            .assertTextContains("/dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver", substring = true)
    }

    @Test
    fun notProvisionedShowsTheProvisioningScreenWithBothButtonsInDebug() {
        show(ShellState.NotProvisioned(importing = false, problem = null))
        compose.onNodeWithTag("shell_provisioning").assertIsDisplayed()
        compose.onNodeWithTag("prov_choose_folder").assertIsDisplayed()
        compose.onNodeWithTag("prov_import_inbox").assertIsDisplayed().performClick()
        assertEquals(1, inboxClicks)
    }

    @Test
    fun noInboxCallbackHidesTheInboxButton() {
        show(ShellState.NotProvisioned(importing = false, problem = null), withInbox = false)
        compose.onNodeWithTag("prov_choose_folder").assertIsDisplayed()
        compose.onNodeWithTag("prov_import_inbox").assertDoesNotExist()
    }

    @Test
    fun importingShowsTheProgressInsteadOfTheButtons() {
        show(ShellState.NotProvisioned(importing = true, problem = null))
        compose.onNodeWithTag("prov_importing_progress").assertIsDisplayed()
        compose.onNodeWithTag("prov_choose_folder").assertDoesNotExist()
        compose.onNodeWithTag("prov_import_inbox").assertDoesNotExist()
    }

    @Test
    fun aProblemIsShownWithItsParametersReplaced() {
        val problem = ProvisioningProblem.of("bundle_file_missing", "path" to "pos.json")
        show(ShellState.NotProvisioned(importing = false, problem = problem))
        compose.onNodeWithTag("shell_problem").assertIsDisplayed().assertTextContains("pos.json", substring = true)
        compose.onNodeWithText("{path}", substring = true).assertDoesNotExist()
    }

    @Test
    fun mismatchShowsBothIdentifiersAndTheImportButtons() {
        show(ShellState.DeviceMismatch("SN-OLD", "SN-NEW", importing = false, problem = null))
        compose.onNodeWithTag("shell_mismatch").assertIsDisplayed()
        compose.onNodeWithText("SN-OLD", substring = true).assertIsDisplayed()
        compose.onNodeWithText("SN-NEW", substring = true).assertIsDisplayed()
        compose.onNodeWithText("{stored}", substring = true).assertDoesNotExist()
        compose.onNodeWithTag("prov_choose_folder").assertIsDisplayed()
        compose.onNodeWithTag("prov_import_inbox").assertIsDisplayed()
    }

    @Test
    fun deviceIdScreenShowsTheDetail() {
        show(ShellState.DeviceIdUnavailable("permission denied"))
        compose.onNodeWithTag("shell_device_id").assertIsDisplayed()
        compose.onNodeWithText("permission denied", substring = true).assertIsDisplayed()
        compose.onNodeWithText("{detail}", substring = true).assertDoesNotExist()
    }

    @Test
    fun openingShowsTheProgress() {
        show(ShellState.Opening)
        compose.onNodeWithTag("shell_opening").assertIsDisplayed()
        compose.onNodeWithTag("shell_opening_progress").assertIsDisplayed()
    }

    @Test
    fun fatalShowsTheTechnicalReason() {
        show(ShellState.Fatal("open: boom"))
        compose.onNodeWithTag("shell_fatal").assertIsDisplayed()
        compose.onNodeWithTag("shell_fatal_reason").assertTextContains("open: boom")
    }

    @Test
    fun readyShowsTheNavigationHostOnLogin() {
        show(ShellState.Ready("1.0.0"))
        compose.onNodeWithTag("placeholder_login").assertIsDisplayed()
        compose.onNodeWithTag("shell_provisioning").assertDoesNotExist()
    }
}
