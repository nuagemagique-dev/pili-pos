package dev.nuagemagique.pilipos.platform.export

import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class ExportViewModelTest {
    private val sd = RemovableVolume("BFD1-0A11", "SD card")
    private val usb = RemovableVolume("1234-ABCD", "USB stick")
    private lateinit var grants: FakeGrants
    private lateinit var folders: ExportFolderStore
    private lateinit var volumes: FakeVolumes
    private lateinit var exporter: Exporter
    private val progress = MutableStateFlow(ExportProgress())
    private var access: ExportProblem? = null
    private val picker = Intent("pick")

    @Before
    fun setUp() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        val context = ApplicationProvider.getApplicationContext<Context>()
        val prefs = context.getSharedPreferences(ExportFolderStore.PREFS_NAME, Context.MODE_PRIVATE)
        prefs.edit().clear().commit()
        grants = FakeGrants()
        folders = ExportFolderStore(prefs, grants)
        volumes = FakeVolumes(intent = picker)
        exporter = mockk()
        every { exporter.progress } returns progress
        coEvery { exporter.access() } coAnswers { access }
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun viewModel() = ExportViewModel(exporter, volumes, folders, { _, _ -> mockk() })

    private fun ExportViewModel.ready(): ExportViewModel {
        volumes.volumes = listOf(sd)
        folders.take(sd.uuid, treeUri(sd.uuid, "PILI-EXPORT"))
        refresh()
        return this
    }

    @Test
    fun startsCheckingThenShowsTheManagerGate() =
        runTest {
            val vm = viewModel()
            assertEquals(ExportUiState.Checking, vm.state.value)
            access = ExportProblem.NOT_MANAGER
            vm.refresh()
            assertEquals(ExportUiState.NotManager, vm.state.value)
        }

    @Test
    fun anOpenOrderBlocksTheScreen() =
        runTest {
            access = ExportProblem.ORDERS_OPEN
            volumes.volumes = listOf(sd)
            val vm = viewModel().also { it.refresh() }
            assertEquals(ExportUiState.Blocked(ExportProblem.ORDERS_OPEN), vm.state.value)
        }

    @Test
    fun withoutAStickThereIsNoStickUntilOneIsInserted() =
        runTest {
            val vm = viewModel().also { it.refresh() }
            assertEquals(ExportUiState.NoStick, vm.state.value)
            volumes.volumes = listOf(sd)
            vm.refreshVolumes()
            assertEquals(ExportUiState.NeedsFolder(listOf(sd), sd), vm.state.value)
            volumes.volumes = emptyList()
            vm.refreshVolumes()
            assertEquals(ExportUiState.NoStick, vm.state.value)
        }

    @Test
    fun refreshingVolumesBeforeTheFirstCheckStaysChecking() =
        runTest {
            val vm = viewModel()
            volumes.volumes = listOf(sd)
            vm.refreshVolumes()
            assertEquals(ExportUiState.Checking, vm.state.value)
        }

    @Test
    fun pickingTheRootIsRefusedWithTheHintThenAFolderIsAccepted() =
        runTest {
            volumes.volumes = listOf(sd)
            val vm = viewModel().also { it.refresh() }
            vm.onFolderPicked(treeUri(sd.uuid, ""))
            assertEquals(ExportUiState.NeedsFolder(listOf(sd), sd, ExportProblem.ROOT), vm.state.value)
            vm.onFolderPicked(treeUri(sd.uuid, "PILI-EXPORT"))
            assertEquals(ExportUiState.Ready(listOf(sd), sd, "PILI-EXPORT"), vm.state.value)
        }

    @Test
    fun aStoredFolderGoesStraightToReadyAndChangeFolderAsksAgain() =
        runTest {
            val vm = viewModel().ready()
            assertEquals(ExportUiState.Ready(listOf(sd), sd, "PILI-EXPORT"), vm.state.value)
            vm.changeFolder()
            assertEquals(ExportUiState.NeedsFolder(listOf(sd), sd), vm.state.value)
            assertTrue(grants.log.last().startsWith("release "))
            assertNull(folders.folder(sd.uuid))
        }

    @Test
    fun thePickerIntentIsTheVolumes() =
        runTest {
            val vm = viewModel()
            assertNull(vm.pickerIntent())
            volumes.volumes = listOf(sd)
            vm.refresh()
            assertSame(picker, vm.pickerIntent())
        }

    @Test
    fun severalVolumesCanBeSwitched() =
        runTest {
            folders.take(usb.uuid, treeUri(usb.uuid, "PILI-EXPORT"))
            volumes.volumes = listOf(sd, usb)
            val vm = viewModel().also { it.refresh() }
            // The volume that already has a folder is preferred.
            assertEquals(ExportUiState.Ready(listOf(sd, usb), usb, "PILI-EXPORT"), vm.state.value)
            vm.selectVolume(sd.uuid)
            assertEquals(ExportUiState.NeedsFolder(listOf(sd, usb), sd), vm.state.value)
            vm.selectVolume(usb.uuid)
            assertTrue(vm.state.value is ExportUiState.Ready)
        }

    @Test
    fun startRunsTheExportAndShowsRunningThenDone() =
        runTest {
            val gate = CompletableDeferred<ExportOutcome>()
            coEvery { exporter.run(any(), any()) } coAnswers { gate.await() }
            val vm = viewModel().ready()
            vm.start()
            assertEquals(ExportUiState.Running(ExportProgress()), vm.state.value)
            progress.value = ExportProgress(filesDone = 2, filesTotal = 9, bytesDone = 5, bytesTotal = 50, current = "pos.db")
            assertEquals(ExportUiState.Running(progress.value), vm.state.value)
            val result = ExportResult("PILI-EXPORT", "pili-export-x", 9, 50, "ab".repeat(32))
            gate.complete(ExportOutcome.Done(result))
            assertEquals(ExportUiState.Done(result), vm.state.value)
            coVerify(exactly = 1) { exporter.run(any(), "PILI-EXPORT") }
            // A stick event while a result is shown does not change the screen.
            volumes.volumes = emptyList()
            vm.refreshVolumes()
            assertEquals(ExportUiState.Done(result), vm.state.value)
        }

    @Test
    fun aFailedExportShowsTheFailureUntilDismissed() =
        runTest {
            val failure = ExportFailure(ExportStep.VERIFY, "pos.db", ExportProblem.READ_BACK_MISMATCH, "x")
            coEvery { exporter.run(any(), any()) } returns ExportOutcome.Failed(failure)
            val vm = viewModel().ready()
            vm.start()
            assertEquals(ExportUiState.Failed(failure), vm.state.value)
            vm.dismiss()
            assertEquals(ExportUiState.Ready(listOf(sd), sd, "PILI-EXPORT"), vm.state.value)
        }

    @Test
    fun dismissDoesNothingOutsideAResultAndStartNeedsReady() =
        runTest {
            val vm = viewModel()
            vm.start()
            vm.dismiss()
            assertEquals(ExportUiState.Checking, vm.state.value)
            coVerify(exactly = 0) { exporter.run(any(), any()) }
        }

    @Test
    fun startWithAGrantThatIsGoneFallsBackToTheFolderChoice() =
        runTest {
            val vm = viewModel().ready()
            grants.held.clear()
            vm.start()
            assertEquals(ExportUiState.NeedsFolder(listOf(sd), sd), vm.state.value)
            coVerify(exactly = 0) { exporter.run(any(), any()) }
        }

    @Test
    fun onFolderPickedAndChangeFolderAreIgnoredWithoutAVolume() =
        runTest {
            val vm = viewModel()
            vm.onFolderPicked(treeUri(sd.uuid, "PILI-EXPORT"))
            vm.changeFolder()
            assertEquals(ExportUiState.Checking, vm.state.value)
            assertTrue(grants.log.isEmpty())
        }
}
