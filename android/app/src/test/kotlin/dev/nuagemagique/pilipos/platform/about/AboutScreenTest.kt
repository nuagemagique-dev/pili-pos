package dev.nuagemagique.pilipos.platform.about

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
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
class AboutScreenTest {
    @get:Rule
    val compose = createComposeRule()

    private fun show(
        state: AboutUiState,
        onRefresh: () -> Unit = {},
    ) {
        compose.setContent { PiliTheme { AboutContent(state, onRefresh) } }
        compose.waitForIdle()
    }

    @Test
    fun loadingShowsTheRootOnly() {
        show(AboutUiState.Loading)
        compose.onNodeWithTag("about_root").assertIsDisplayed()
        compose.onNodeWithTag("about_apk_sha1").assertDoesNotExist()
    }

    @Test
    fun loadedShowsTheHashesAndTheDeviceId() {
        val report = sampleReport()
        show(AboutUiState.Loaded(report))
        compose.onNodeWithTag("about_root").assertIsDisplayed()
        compose.onNodeWithTag("about_apk_sha1").assertTextEquals(report.apkSha1.orEmpty())
        compose.onNodeWithTag("about_so_sha1").assertTextEquals(report.soSha1.orEmpty())
        compose.onNodeWithTag("about_device_id").assertTextEquals("SN-0001")
    }

    @Test
    fun aMissingLibraryHashShowsTheReason() {
        show(AboutUiState.Loaded(sampleReport(soSha1 = null)))
        compose.onNodeWithTag("about_so_sha1").performScrollTo().assertIsDisplayed()
    }

    @Test
    fun healthyStorageShowsNoWarning() {
        show(AboutUiState.Loaded(sampleReport()))
        compose.onNodeWithTag("about_free_space").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("about_warning_low_space").assertDoesNotExist()
        compose.onNodeWithTag("about_warning_backups").assertDoesNotExist()
    }

    @Test
    fun lowFreeSpaceShowsItsWarning() {
        show(AboutUiState.Loaded(sampleReport(freeInternalBytes = 1_000_000_000L, backupsBytes = 1_000L)))
        compose.onNodeWithTag("about_warning_low_space").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("about_warning_backups").assertDoesNotExist()
    }

    @Test
    fun heavyBackupsShowTheirWarning() {
        show(AboutUiState.Loaded(sampleReport(freeInternalBytes = 10_000_000_000L, backupsBytes = 6_000_000_000L)))
        compose.onNodeWithTag("about_warning_backups").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("about_warning_low_space").assertDoesNotExist()
    }

    @Test
    fun refreshCallsBack() {
        var refreshed = 0
        show(AboutUiState.Loaded(sampleReport())) { refreshed++ }
        compose.onNodeWithTag("about_refresh").performClick()
        assertEquals(1, refreshed)
    }
}
