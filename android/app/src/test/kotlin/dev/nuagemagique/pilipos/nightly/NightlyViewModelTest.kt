package dev.nuagemagique.pilipos.nightly

import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.LaunchResult
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.coVerifyOrder
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.ExportStatus
import uniffi.pos_ffi.PosException

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class NightlyViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private var language = AppLanguage.FR

    @Before
    fun setUp() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    /** The real gateway over a ready fixture engine whose export calls the test stubs. */
    private class Rig(
        val fixture: EngineFixture,
        val gateway: EngineGateway,
    ) {
        val engine = fixture.engine
    }

    private suspend fun ready(scope: TestScope): Rig {
        val fixture = EngineFixture(scope, tmp.root)
        fixture.holder.open()
        return Rig(fixture, fixture.gateway(scope))
    }

    private fun viewModel(rig: Rig) = NightlyViewModel(rig.gateway, I18nTestSupport.errorTexts()) { language }

    @Test
    fun startsLoadingAndRefreshLoadsTheStatus() =
        runTest {
            val rig = ready(this)
            val status = exportStatus()
            coEvery { rig.engine.exportStatus() } returns status
            val vm = viewModel(rig)
            assertEquals(NightlyUiState.Loading, vm.state.value)
            vm.refresh()
            assertEquals(NightlyUiState.Loaded(status, running = false), vm.state.value)
        }

    @Test
    fun aRefusedCallShowsTheErrorTextsMessage() =
        runTest {
            val rig = ready(this)
            val refusal = PosException.Coded("pin_locked", mapOf("seconds" to "30"))
            coEvery { rig.engine.exportStatus() } throws refusal
            val vm = viewModel(rig)
            vm.refresh()
            val expected = I18nTestSupport.errorTexts().describe(refusal, AppLanguage.FR)
            assertEquals(NightlyUiState.Failed(expected), vm.state.value)
            language = AppLanguage.EN
            vm.refresh()
            val english = I18nTestSupport.errorTexts().describe(refusal, AppLanguage.EN)
            assertEquals(NightlyUiState.Failed(english), vm.state.value)
            assertTrue(english != expected)
        }

    @Test
    fun anEngineThatIsNotReadyGivesFailed() =
        runTest {
            val fixture = EngineFixture(this, tmp.root, LaunchResult.NotProvisioned)
            fixture.holder.open()
            val vm = NightlyViewModel(fixture.gateway(this), I18nTestSupport.errorTexts()) { AppLanguage.FR }
            vm.refresh()
            val failed = vm.state.value as NightlyUiState.Failed
            assertTrue(failed.message.isNotBlank())
            coVerify(exactly = 0) { fixture.engine.exportStatus() }
        }

    @Test
    fun runNowCallsExportRunNowThenReadsTheStatusAgain() =
        runTest {
            val rig = ready(this)
            val before = exportStatus(pending = 2)
            val after = exportStatus(pending = 0)
            coEvery { rig.engine.exportStatus() } returnsMany listOf(before, after)
            coEvery { rig.engine.exportRunNow() } returns before
            val vm = viewModel(rig)
            vm.refresh()
            vm.runNow()
            coVerifyOrder {
                rig.engine.exportStatus()
                rig.engine.exportRunNow()
                rig.engine.exportStatus()
            }
            assertEquals(NightlyUiState.Loaded(after, running = false), vm.state.value)
        }

    @Test
    fun theRunningFlagIsSetWhileRunNowIsInFlightAndASecondTapIsIgnored() =
        runTest {
            val rig = ready(this)
            val status = exportStatus(pending = 1)
            val gate = CompletableDeferred<ExportStatus>()
            coEvery { rig.engine.exportStatus() } returns status
            coEvery { rig.engine.exportRunNow() } coAnswers { gate.await() }
            val vm = viewModel(rig)
            vm.refresh()
            vm.runNow()
            assertEquals(NightlyUiState.Loaded(status, running = true), vm.state.value)
            vm.runNow()
            gate.complete(status)
            runCurrent()
            assertEquals(NightlyUiState.Loaded(status, running = false), vm.state.value)
            coVerify(exactly = 1) { rig.engine.exportRunNow() }
        }

    @Test
    fun runNowBeforeTheFirstStatusDoesNothing() =
        runTest {
            val rig = ready(this)
            val vm = viewModel(rig)
            vm.runNow()
            assertEquals(NightlyUiState.Loading, vm.state.value)
            coVerify(exactly = 0) { rig.engine.exportRunNow() }
        }

    @Test
    fun aRefusedRunNowShowsTheMessage() =
        runTest {
            val rig = ready(this)
            coEvery { rig.engine.exportStatus() } returns exportStatus(pending = 1)
            val refusal = PosException.Coded("not_admin", emptyMap())
            coEvery { rig.engine.exportRunNow() } throws refusal
            val vm = viewModel(rig)
            vm.refresh()
            vm.runNow()
            val expected = I18nTestSupport.errorTexts().describe(refusal, AppLanguage.FR)
            assertEquals(NightlyUiState.Failed(expected), vm.state.value)
        }

    @Test
    fun aPollDuringARetryKeepsTheRunningFlag() =
        runTest {
            val rig = ready(this)
            val status = exportStatus(pending = 1)
            val gate = CompletableDeferred<ExportStatus>()
            coEvery { rig.engine.exportStatus() } returns status
            coEvery { rig.engine.exportRunNow() } coAnswers { gate.await() }
            val vm = viewModel(rig)
            vm.refresh()
            vm.runNow()
            vm.startPolling()
            assertEquals(NightlyUiState.Loaded(status, running = true), vm.state.value)
            gate.complete(status)
            runCurrent()
            vm.stopPolling()
        }

    @Test
    fun pollingReadsEvery30SecondsUntilItIsStopped() =
        runTest {
            val rig = ready(this)
            coEvery { rig.engine.exportStatus() } returns exportStatus()
            val vm = viewModel(rig)
            vm.startPolling()
            vm.startPolling()
            coVerify(exactly = 1) { rig.engine.exportStatus() }
            advanceTimeBy(NIGHTLY_POLL_MS + 1)
            runCurrent()
            coVerify(exactly = 2) { rig.engine.exportStatus() }
            advanceTimeBy(NIGHTLY_POLL_MS)
            runCurrent()
            coVerify(exactly = 3) { rig.engine.exportStatus() }
            vm.stopPolling()
            advanceTimeBy(NIGHTLY_POLL_MS * 4)
            runCurrent()
            coVerify(exactly = 3) { rig.engine.exportStatus() }
            vm.startPolling()
            coVerify(exactly = 4) { rig.engine.exportStatus() }
            vm.stopPolling()
        }

    @Test
    fun aFailedPollRecoversOnTheNextOne() =
        runTest {
            val rig = ready(this)
            val status = exportStatus()
            coEvery { rig.engine.exportStatus() } throws PosException.Coded("not_admin", emptyMap()) andThen status
            val vm = viewModel(rig)
            vm.startPolling()
            assertTrue(vm.state.value is NightlyUiState.Failed)
            advanceTimeBy(NIGHTLY_POLL_MS + 1)
            runCurrent()
            assertEquals(NightlyUiState.Loaded(status, running = false), vm.state.value)
            vm.stopPolling()
        }

    @Test
    fun theDeviceLanguageMapsToTheAppLanguages() {
        val current = SystemNightlyLanguage().current()
        assertTrue(current in AppLanguage.entries)
    }
}
