package dev.nuagemagique.pilipos.shell

import app.cash.turbine.test
import dev.nuagemagique.pilipos.engine.ClosedReason
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.provisioning.BundleSource
import dev.nuagemagique.pilipos.provisioning.ImportResult
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem
import io.mockk.coEvery
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import uniffi.pos_ffi.PosEvent
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
class ShellViewModelTest {
    private val source = BundleSource.Directory(File("inbox"))

    @Before
    fun setUp() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun opensTheEngineOnInit() {
        val fixture = ShellFixture()
        fixture.viewModel()
        assertEquals(listOf("open"), fixture.calls)
    }

    @Test
    fun notDeviceOwnerShowsNeedsDeviceOwner() {
        val fixture = ShellFixture(owner = false, initial = EngineState.Ready("1.0.0"))
        assertEquals(ShellState.NeedsDeviceOwner, fixture.viewModel().state.value)
    }

    @Test
    fun followsTheEngineState() =
        runTest {
            val fixture = ShellFixture()
            val viewModel = fixture.viewModel()
            viewModel.state.test {
                assertEquals(ShellState.NotProvisioned(importing = false, problem = null), awaitItem())
                fixture.engineState.value = EngineState.Closed(ClosedReason.DeviceMismatch("A", "B"))
                assertEquals(ShellState.DeviceMismatch("A", "B", importing = false, problem = null), awaitItem())
                fixture.engineState.value = EngineState.Closed(ClosedReason.DeviceIdUnavailable("x"))
                assertEquals(ShellState.DeviceIdUnavailable("x"), awaitItem())
                fixture.engineState.value = EngineState.Opening
                assertEquals(ShellState.Opening, awaitItem())
                fixture.engineState.value = EngineState.Ready("1.2.3")
                assertEquals(ShellState.Ready("1.2.3"), awaitItem())
                fixture.engineState.value = EngineState.Fatal("boom")
                assertEquals(ShellState.Fatal("boom"), awaitItem())
            }
        }

    @Test
    fun sessionLockedEmitsNavigateToLoginOncePerEvent() =
        runTest {
            val fixture = ShellFixture()
            val viewModel = fixture.viewModel()
            viewModel.effects.test {
                fixture.eventFlow.emit(PosEvent.SessionLocked(mockk(relaxed = true)))
                assertEquals(ShellEffect.NavigateToLogin, awaitItem())
                fixture.eventFlow.emit(PosEvent.Resync(1L))
                fixture.eventFlow.emit(PosEvent.SessionLocked(mockk(relaxed = true)))
                assertEquals(ShellEffect.NavigateToLogin, awaitItem())
                expectNoEvents()
            }
        }

    @Test
    fun importRunsCloseImportOpenAndTogglesTheImportingFlag() =
        runTest {
            val fixture = ShellFixture()
            val gate = CompletableDeferred<ImportResult>()
            coEvery { fixture.importer.import(source) } coAnswers
                {
                    fixture.calls += "import"
                    gate.await()
                }
            val viewModel = fixture.viewModel()
            viewModel.state.test {
                assertEquals(ShellState.NotProvisioned(importing = false, problem = null), awaitItem())
                viewModel.importFrom(source)
                assertEquals(ShellState.NotProvisioned(importing = true, problem = null), awaitItem())
                gate.complete(ImportResult.Success(mockk(relaxed = true)))
                assertEquals(ShellState.NotProvisioned(importing = false, problem = null), awaitItem())
            }
            assertEquals(listOf("open", "close", "import", "open"), fixture.calls)
        }

    @Test
    fun aFailedImportShowsTheProblemAndReopensTheEngine() =
        runTest {
            val fixture = ShellFixture()
            val problem = ProvisioningProblem.of("bundle_file_missing", "path" to "pos.json")
            coEvery { fixture.importer.import(source) } returns ImportResult.Failure(problem)
            val viewModel = fixture.viewModel()
            viewModel.importFrom(source)
            assertEquals(ShellState.NotProvisioned(importing = false, problem = problem), viewModel.state.value)
            assertEquals(listOf("open", "close", "open"), fixture.calls)
        }

    @Test
    fun aNewImportClearsThePreviousProblem() =
        runTest {
            val fixture = ShellFixture()
            val problem = ProvisioningProblem.of("bundle_signature_invalid")
            val gate = CompletableDeferred<ImportResult>()
            var runs = 0
            coEvery { fixture.importer.import(source) } coAnswers
                {
                    if (runs++ == 0) ImportResult.Failure(problem) else gate.await()
                }
            val viewModel = fixture.viewModel()
            viewModel.importFrom(source)
            viewModel.importFrom(source)
            assertEquals(ShellState.NotProvisioned(importing = true, problem = null), viewModel.state.value)
            gate.complete(ImportResult.Success(mockk(relaxed = true)))
            assertEquals(ShellState.NotProvisioned(importing = false, problem = null), viewModel.state.value)
        }

    @Test
    fun aSecondImportIsIgnoredWhileOneRuns() =
        runTest {
            val fixture = ShellFixture()
            val gate = CompletableDeferred<ImportResult>()
            coEvery { fixture.importer.import(source) } coAnswers { gate.await() }
            val viewModel = fixture.viewModel()
            viewModel.importFrom(source)
            viewModel.importFrom(source)
            gate.complete(ImportResult.Success(mockk(relaxed = true)))
            assertEquals(listOf("open", "close", "open"), fixture.calls)
        }

    @Test
    fun importFromInboxUsesTheDebugInboxDirectory() =
        runTest {
            val fixture = ShellFixture()
            val inbox = File("debug-inbox")
            every { fixture.paths.debugInboxDir() } returns inbox
            coEvery { fixture.importer.import(BundleSource.Directory(inbox)) } returns
                ImportResult.Failure(ProvisioningProblem.of("busy"))
            val viewModel = fixture.viewModel()
            viewModel.importFromInbox()
            assertEquals("busy", (viewModel.state.value as ShellState.NotProvisioned).problem?.code)
        }

    @Test
    fun importFromInboxWithoutADirectoryShowsAProblem() =
        runTest {
            val fixture = ShellFixture()
            every { fixture.paths.debugInboxDir() } returns null
            val viewModel = fixture.viewModel()
            viewModel.importFromInbox()
            val shown = viewModel.state.value as ShellState.NotProvisioned
            assertEquals("copy_failed", shown.problem?.code)
            assertEquals(false, shown.importing)
        }

    @Test
    fun theRegistryIsExposedForTheReadyContent() {
        val fixture = ShellFixture()
        assertEquals(fixture.registry, fixture.viewModel().registry)
    }
}
