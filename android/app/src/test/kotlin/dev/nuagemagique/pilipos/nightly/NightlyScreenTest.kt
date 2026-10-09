package dev.nuagemagique.pilipos.nightly

import androidx.activity.ComponentActivity
import androidx.compose.runtime.Composable
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.Lifecycle
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.every
import io.mockk.mockk
import io.mockk.verify
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "fr-w960dp-h540dp-mdpi")
class NightlyScreenTest {
    @get:Rule
    val compose = createAndroidComposeRule<ComponentActivity>()

    private var retries = 0
    private var backs = 0

    private fun show(state: NightlyUiState) {
        compose.setContent { Themed { NightlyContent(state, onRetry = { retries += 1 }, onBack = { backs += 1 }) } }
    }

    @Composable
    private fun Themed(content: @Composable () -> Unit) = PiliTheme(content = content)

    @Test
    fun loadingShowsTheTitleAndNoButton() {
        show(NightlyUiState.Loading)
        compose.onNodeWithTag("nightly_screen").assertIsDisplayed()
        compose.onNodeWithText("Export nocturne").assertIsDisplayed()
        compose.onNodeWithTag("nightly_loading").assertIsDisplayed()
        compose.onNodeWithTag("nightly_retry").assertDoesNotExist()
    }

    @Test
    fun upToDateShowsItsRows() {
        show(NightlyUiState.Loaded(exportStatus()))
        compose.onNodeWithTag("nightly_state").assertTextEquals("À jour")
        compose.onNodeWithText("Destination").assertIsDisplayed()
        compose.onNodeWithText("https://exports.example.test:8443").assertIsDisplayed()
        compose.onNodeWithText("Dernier export").assertIsDisplayed()
        // 01:00 UTC on 9 October is 03:00 in Brussels (summer time).
        compose.onNodeWithText("2026-10-08 · 09/10 03:00").assertIsDisplayed()
        compose.onNodeWithText("09/10 03:05").assertIsDisplayed()
        compose.onNodeWithText("Conservés : 12 sur 30 jours").assertIsDisplayed()
        compose.onNodeWithTag("nightly_next_attempt").assertDoesNotExist()
        compose.onNodeWithTag("nightly_last_error").assertDoesNotExist()
        compose.onNodeWithTag("nightly_dropped").assertDoesNotExist()
        compose.onNodeWithTag("nightly_retry").assertIsEnabled()
    }

    @Test
    fun pendingShowsTheCountTheNextAttemptTheErrorAndTheDropped() {
        show(NightlyUiState.Loaded(pendingStatus()))
        compose.onNodeWithTag("nightly_state").assertTextEquals("3 exports en attente")
        compose.onNodeWithText("Prochain essai").assertIsDisplayed()
        compose.onNodeWithText("09/10 12:30").assertIsDisplayed()
        compose.onNodeWithText("Dernière erreur").assertIsDisplayed()
        compose.onNodeWithText("upload refused: HTTP 503 · 09/10 12:00").assertIsDisplayed()
        compose.onNodeWithText("Abandonnés").assertIsDisplayed()
        compose.onNodeWithText("2").assertIsDisplayed()
    }

    @Test
    fun oneWaitingExportUsesTheSingularForm() {
        show(NightlyUiState.Loaded(exportStatus(pending = 1)))
        compose.onNodeWithTag("nightly_state").assertTextEquals("1 export en attente")
    }

    @Test
    fun neverPreparedAndNeverUploadedSayNever() {
        show(NightlyUiState.Loaded(exportStatus(lastExportId = null, lastPreparedAt = null, lastUploadedAt = null, endpoint = null)))
        compose.onNodeWithTag("nightly_endpoint").assertDoesNotExist()
        compose.onNodeWithTag("nightly_last_export").assertIsDisplayed()
        compose.onAllNodesWithText("Jamais").assertCountEquals(2)
    }

    @Test
    fun anUnparsableTimeIsShownAsIs() {
        show(NightlyUiState.Loaded(exportStatus(lastUploadedAt = "yesterday")))
        compose.onNodeWithText("yesterday").assertIsDisplayed()
    }

    @Test
    fun disabledShowsTheStateOnlyAndNoButton() {
        show(NightlyUiState.Loaded(disabledStatus()))
        compose.onNodeWithTag("nightly_state").assertTextEquals("Non configuré")
        compose.onNodeWithTag("nightly_retry").assertDoesNotExist()
        compose.onNodeWithTag("nightly_last_export").assertDoesNotExist()
        compose.onNodeWithTag("nightly_kept").assertDoesNotExist()
    }

    @Test
    fun invalidShowsTheProblemAndNoButton() {
        show(NightlyUiState.Loaded(exportStatus(enabled = false, problem = "export.url must use https")))
        compose.onNodeWithTag("nightly_state").assertTextEquals("Configuration invalide")
        compose.onNodeWithTag("nightly_problem").assertTextEquals("export.url must use https")
        compose.onNodeWithTag("nightly_retry").assertDoesNotExist()
        compose.onNodeWithTag("nightly_last_upload").assertDoesNotExist()
    }

    @Test
    fun failedShowsTheMessage() {
        show(NightlyUiState.Failed("Moteur indisponible"))
        compose.onNodeWithTag("nightly_failed").assertTextEquals("Moteur indisponible")
        compose.onNodeWithTag("nightly_retry").assertDoesNotExist()
    }

    @Test
    fun retryNowCallsBack() {
        show(NightlyUiState.Loaded(pendingStatus()))
        compose.onNodeWithText("Réessayer maintenant").assertIsDisplayed()
        compose.onNodeWithTag("nightly_retry").performClick()
        assertEquals(1, retries)
    }

    @Test
    fun whileRunningTheButtonIsDisabledAndSaysSo() {
        show(NightlyUiState.Loaded(pendingStatus(), running = true))
        compose.onNodeWithTag("nightly_retry").assertIsNotEnabled()
        compose.onNodeWithText("Nouvel essai…").assertIsDisplayed()
    }

    @Test
    fun backCallsBack() {
        show(NightlyUiState.Loading)
        compose.onNodeWithTag("nightly_back").performClick()
        assertEquals(1, backs)
    }

    @Test
    @Config(qualifiers = "en-w960dp-h540dp-mdpi")
    fun englishTexts() {
        show(NightlyUiState.Loaded(pendingStatus(), running = false))
        compose.onNodeWithText("Nightly export").assertIsDisplayed()
        compose.onNodeWithTag("nightly_state").assertTextEquals("3 exports waiting")
        compose.onNodeWithText("Retry now").assertIsDisplayed()
        compose.onNodeWithText("Next attempt").assertIsDisplayed()
        compose.onNodeWithText("Last error").assertIsDisplayed()
        compose.onNodeWithText("Kept: 12 of 30 days").assertIsDisplayed()
        compose.onNodeWithText("Dropped").assertIsDisplayed()
    }

    @Test
    @Config(qualifiers = "zh-w960dp-h540dp-mdpi")
    fun chineseTexts() {
        show(NightlyUiState.Loaded(pendingStatus()))
        compose.onNodeWithText("夜间导出").assertIsDisplayed()
        compose.onNodeWithTag("nightly_state").assertTextEquals("3 个导出待上传")
        compose.onNodeWithText("立即重试").assertIsDisplayed()
        compose.onNodeWithText("已保留：12 个（30 天内）").assertIsDisplayed()
    }

    @Test
    @Config(qualifiers = "en-w960dp-h540dp-mdpi")
    fun winterTimesFollowBrusselsNotTheDeviceZone() {
        show(NightlyUiState.Loaded(exportStatus(lastUploadedAt = "2026-01-15T12:00:00+00:00")))
        compose.onNodeWithText("15/01 13:00").assertIsDisplayed()
    }

    @Test
    fun theScreenStartsPollingWhenStartedAndStopsWhenStopped() {
        val viewModel = mockk<NightlyViewModel>(relaxed = true)
        every { viewModel.state } returns MutableStateFlow(NightlyUiState.Loaded(exportStatus()))
        compose.setContent { PiliTheme { NightlyScreen(onBack = { backs += 1 }, viewModel = viewModel) } }
        compose.waitForIdle()
        verify(exactly = 1) { viewModel.startPolling() }
        compose.onNodeWithTag("nightly_retry").performClick()
        verify(exactly = 1) { viewModel.runNow() }
        compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
        compose.waitForIdle()
        verify(exactly = 1) { viewModel.stopPolling() }
        compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        compose.waitForIdle()
        verify(exactly = 2) { viewModel.startPolling() }
    }
}
