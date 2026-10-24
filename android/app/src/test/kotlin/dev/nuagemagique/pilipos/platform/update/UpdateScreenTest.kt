package dev.nuagemagique.pilipos.platform.update

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.io.File

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class UpdateScreenTest {
    @get:Rule
    val compose = createComposeRule()

    private val installed = InstalledInfo("1.0.1", 101, "f".repeat(40))
    private val verified = verifiedOf(File("x.apk"))
    private val base = UpdateUiState(installed = installed, checked = true)

    private fun show(
        state: UpdateUiState,
        actions: UpdateActions = UpdateActions(),
    ) {
        compose.setContent { PiliTheme { UpdateContent(state, actions) } }
        compose.waitForIdle()
    }

    private fun tag(name: String) = compose.onNodeWithTag(name).performScrollTo()

    @Test
    fun theRootShowsTheInstalledVersionAndSha1() {
        show(base)
        compose.onNodeWithTag("update_root").assertIsDisplayed()
        tag("update_installed").assertIsDisplayed()
        tag("update_installed_sha1").assertTextContains("f".repeat(40), substring = true)
    }

    @Test
    fun aMissingShaShowsUnknown() {
        show(base.copy(installed = installed.copy(apkSha1 = null)))
        tag("update_installed_sha1").assertIsDisplayed()
    }

    @Test
    fun everyPhaseRenders() {
        var state by mutableStateOf(base)
        compose.setContent { PiliTheme { UpdateContent(state, UpdateActions()) } }
        listOf(
            UpdatePhase.Idle,
            UpdatePhase.Checking("ghcr"),
            UpdatePhase.NoStick,
            UpdatePhase.Verified(verified),
            UpdatePhase.Refused(UpdateRefusal(UpdateRefusalCode.NOT_NEWER, "102 is not greater than 102")),
            UpdatePhase.Installing,
            UpdatePhase.InstallFailed(InstallRefusal.SILENT_INSTALL_REFUSED, "status 3"),
            UpdatePhase.InstallFailed(InstallRefusal.BACKUP_FAILED, null),
        ).forEach {
            state = base.copy(phase = it)
            compose.waitForIdle()
            compose.onNodeWithTag("update_root").assertIsDisplayed()
        }
    }

    @Test
    fun aNonManagerSeesOnlyTheGate() {
        show(base.copy(failures = listOf(PreconditionFailure.NOT_MANAGER)))
        tag("update_not_manager").assertIsDisplayed()
        compose.onNodeWithTag("update_check_online").assertDoesNotExist()
        compose.onNodeWithTag("update_install").assertDoesNotExist()
    }

    @Test
    fun theTwoSourceButtonsCallTheirActions() {
        var online = 0
        var usb = 0
        show(base, UpdateActions(onCheckOnline = { online++ }, onFromUsb = { usb++ }))
        tag("update_check_online").assertIsEnabled().performClick()
        tag("update_from_usb").assertIsEnabled().performClick()
        assertEquals(1, online)
        assertEquals(1, usb)
    }

    @Test
    fun theButtonsAreDisabledWhileChecking() {
        show(base.copy(phase = UpdatePhase.Checking("ghcr")))
        tag("update_check_online").assertIsNotEnabled()
        tag("update_from_usb").assertIsNotEnabled()
        tag("update_checking").assertIsDisplayed()
    }

    @Test
    fun noStickAsksToInsertOne() {
        show(base.copy(phase = UpdatePhase.NoStick))
        tag("update_no_stick").assertIsDisplayed()
    }

    @Test
    fun aVerifiedUpdateShowsItsFactsAndEnablesInstall() {
        var installs = 0
        show(base.copy(phase = UpdatePhase.Verified(verified)), UpdateActions(onInstall = { installs++ }))
        tag("update_result").assertIsDisplayed()
        tag("update_result_version").assertTextContains("1.2.0", substring = true)
        tag("update_result_sha1").assertTextContains("a".repeat(40), substring = true)
        tag("update_result_signer").assertTextContains("c".repeat(64), substring = true)
        tag("update_install").assertIsEnabled().performClick()
        assertEquals(1, installs)
    }

    @Test
    fun aPreconditionFailureListsItsTextAndDisablesInstall() {
        val failures = listOf(PreconditionFailure.OPEN_ORDER, PreconditionFailure.Z_DUE)
        show(base.copy(phase = UpdatePhase.Verified(verified), failures = failures))
        tag("update_failures").assertIsDisplayed()
        tag("update_failure_open_order").assertIsDisplayed()
        tag("update_failure_z_due").assertIsDisplayed()
        tag("update_install").assertIsNotEnabled()
    }

    @Test
    fun aRefusalShowsItsMessageAndNoInstallButton() {
        show(base.copy(phase = UpdatePhase.Refused(UpdateRefusal(UpdateRefusalCode.SHA256_MISMATCH, "SHA-256 is 00"))))
        tag("update_refusal").assertIsDisplayed()
        tag("update_refusal_text").assertIsDisplayed()
        tag("update_refusal_detail").assertTextContains("SHA-256 is 00", substring = true)
        compose.onNodeWithTag("update_install").assertDoesNotExist()
    }

    @Test
    fun installingHidesTheButtons() {
        show(base.copy(phase = UpdatePhase.Installing))
        tag("update_installing").assertIsDisplayed()
        compose.onNodeWithTag("update_check_online").assertDoesNotExist()
        compose.onNodeWithTag("update_install").assertDoesNotExist()
    }

    @Test
    fun aFailedInstallShowsItsReasonAndNoInstallButton() {
        show(base.copy(phase = UpdatePhase.InstallFailed(InstallRefusal.ENGINE_BUSY, "busy")))
        tag("update_install_failed").assertIsDisplayed()
        tag("update_install_failed_text").assertIsDisplayed()
        compose.onNodeWithTag("update_install").assertDoesNotExist()
    }
}
