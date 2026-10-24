package dev.nuagemagique.pilipos.platform.export

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
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

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class ExportScreenTest {
    @get:Rule
    val compose = createComposeRule()

    private val sd = RemovableVolume("BFD1-0A11", "SD card")
    private val usb = RemovableVolume("1234-ABCD", "")
    private val result = ExportResult("PILI-EXPORT", "pili-export-20261009-101530-SN-0001", 21, 12_345_678, "ab".repeat(32))

    private fun show(
        state: ExportUiState,
        actions: ExportActions = ExportActions(),
    ) {
        compose.setContent { PiliTheme { ExportContent(state, actions) } }
        compose.waitForIdle()
    }

    private fun tag(name: String) = compose.onNodeWithTag(name).performScrollTo()

    @Test
    fun everyStateShowsTheRoot() {
        var state by mutableStateOf<ExportUiState>(ExportUiState.Checking)
        compose.setContent { PiliTheme { ExportContent(state, ExportActions()) } }
        listOf(
            ExportUiState.Checking,
            ExportUiState.NotManager,
            ExportUiState.Blocked(ExportProblem.ORDERS_OPEN),
            ExportUiState.NoStick,
        ).forEach {
            state = it
            compose.waitForIdle()
            compose.onNodeWithTag("export_root").assertIsDisplayed()
        }
    }

    @Test
    fun theGatesShowTheirMessage() {
        show(ExportUiState.NotManager)
        tag("export_not_manager").assertIsDisplayed()
    }

    @Test
    fun anOpenOrderShowsItsReason() {
        show(ExportUiState.Blocked(ExportProblem.ORDERS_OPEN))
        tag("export_blocked").assertIsDisplayed()
    }

    @Test
    fun noStickAsksToInsertOne() {
        show(ExportUiState.NoStick)
        tag("export_no_stick").assertIsDisplayed()
        compose.onNodeWithTag("export_start").assertDoesNotExist()
    }

    @Test
    fun needsFolderShowsThePickerButtonAndTheRootError() {
        var picked = 0
        show(ExportUiState.NeedsFolder(listOf(sd), sd, ExportProblem.ROOT), ExportActions(onPick = { picked++ }))
        tag("export_error").assertIsDisplayed()
        tag("export_pick").performClick()
        assertEquals(1, picked)
    }

    @Test
    fun needsFolderWithoutAnErrorShowsNone() {
        show(ExportUiState.NeedsFolder(listOf(sd), sd))
        tag("export_pick").assertIsDisplayed()
        compose.onNodeWithTag("export_error").assertDoesNotExist()
    }

    @Test
    fun readyShowsTheFolderAndStartsOrChanges() {
        var started = 0
        var changed = 0
        show(ExportUiState.Ready(listOf(sd), sd, "PILI-EXPORT"), ExportActions(onStart = { started++ }, onChangeFolder = { changed++ }))
        tag("export_folder").assertIsDisplayed()
        tag("export_start").assertIsEnabled().performClick()
        tag("export_change").performClick()
        assertEquals(1, started)
        assertEquals(1, changed)
    }

    @Test
    fun severalVolumesOfferTheOthers() {
        val chosen = mutableListOf<String>()
        show(ExportUiState.Ready(listOf(sd, usb), sd, "PILI-EXPORT"), ExportActions(onSelectVolume = { chosen += it }))
        compose.onNodeWithTag("export_volume_${sd.uuid}").assertDoesNotExist()
        tag("export_volume_${usb.uuid}").performClick()
        assertEquals(listOf(usb.uuid), chosen)
    }

    @Test
    fun runningShowsTheProgress() {
        show(ExportUiState.Running(ExportProgress(filesDone = 3, filesTotal = 10, bytesDone = 50, bytesTotal = 100, current = "pos.db")))
        tag("export_progress").assertIsDisplayed()
        compose.onNodeWithTag("export_start").assertDoesNotExist()
    }

    @Test
    fun runningBeforeTheTotalIsKnownDoesNotDivideByZero() {
        show(ExportUiState.Running(ExportProgress()))
        tag("export_progress").assertIsDisplayed()
    }

    @Test
    fun doneShowsTheFolderTheTotalsAndTheManifestHash() {
        var closed = 0
        show(ExportUiState.Done(result), ExportActions(onDismiss = { closed++ }))
        tag("export_done").assertIsDisplayed()
        tag("export_manifest_sha").assertIsDisplayed()
        tag("export_dismiss").performClick()
        assertEquals(1, closed)
    }

    @Test
    fun failedShowsTheStepTheFileAndTheKeptNotice() {
        show(ExportUiState.Failed(ExportFailure(ExportStep.VERIFY, "pos.db", ExportProblem.READ_BACK_MISMATCH, "x != y")))
        tag("export_failed").assertIsDisplayed()
        tag("export_failed_reason").assertIsDisplayed()
        tag("export_failed_kept").assertIsDisplayed()
    }

    @Test
    fun aRefusalBeforeAnythingWasWrittenHasNoKeptNotice() {
        show(ExportUiState.Failed(ExportFailure(ExportStep.PRECHECK, null, ExportProblem.NOT_MANAGER)))
        tag("export_failed_reason").assertIsDisplayed()
        compose.onNodeWithTag("export_failed_kept").assertDoesNotExist()
    }
}
