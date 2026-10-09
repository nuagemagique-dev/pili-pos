package dev.nuagemagique.pilipos.platform.about

import app.cash.turbine.test
import io.mockk.coEvery
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

@OptIn(ExperimentalCoroutinesApi::class)
class AboutViewModelTest {
    @Before
    fun setUp() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun startsLoadingThenShowsTheReport() =
        runTest {
            val gate = CompletableDeferred<IdentityReport>()
            val identity = mockk<IdentityProvider>()
            coEvery { identity.report() } coAnswers { gate.await() }
            val viewModel = AboutViewModel(identity)
            viewModel.state.test {
                assertEquals(AboutUiState.Loading, awaitItem())
                val report = sampleReport()
                gate.complete(report)
                assertEquals(AboutUiState.Loaded(report), awaitItem())
            }
        }

    @Test
    fun refreshKeepsTheOldReportUntilTheNewOneIsReady() =
        runTest {
            val first = sampleReport(backupsCount = 1)
            val second = sampleReport(backupsCount = 2)
            val gate = CompletableDeferred<IdentityReport>()
            val identity = mockk<IdentityProvider>()
            var calls = 0
            coEvery { identity.report() } coAnswers { if (calls++ == 0) first else gate.await() }
            val viewModel = AboutViewModel(identity)
            viewModel.state.test {
                assertEquals(AboutUiState.Loaded(first), awaitItem())
                viewModel.refresh()
                expectNoEvents()
                gate.complete(second)
                assertEquals(AboutUiState.Loaded(second), awaitItem())
            }
            assertEquals(2, calls)
        }

    @Test
    fun aSecondRefreshReplacesTheRunningOne() =
        runTest {
            val slow = CompletableDeferred<IdentityReport>()
            val last = sampleReport(backupsCount = 9)
            val identity = mockk<IdentityProvider>()
            var calls = 0
            coEvery { identity.report() } coAnswers { if (calls++ == 0) slow.await() else last }
            val viewModel = AboutViewModel(identity)
            assertEquals(AboutUiState.Loading, viewModel.state.value)
            viewModel.refresh()
            assertEquals(AboutUiState.Loaded(last), viewModel.state.value)
            slow.complete(sampleReport(backupsCount = 1))
            assertEquals(AboutUiState.Loaded(last), viewModel.state.value)
        }
}
