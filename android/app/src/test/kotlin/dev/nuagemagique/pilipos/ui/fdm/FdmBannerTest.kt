package dev.nuagemagique.pilipos.ui.fdm

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.assertTopPositionInRootIsEqualTo
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.FdmSeverity

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class FdmBannerTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private var state by mutableStateOf(FdmUiState())

    private val warning =
        FdmUiState(connected = true, severity = FdmSeverity.WARNING, code = "BUFFER_NEAR_FULL")

    private fun show(
        initial: FdmUiState,
        language: AppLanguage = AppLanguage.FR,
    ) {
        state = initial
        compose.setContent {
            PiliTheme { ProvideUiText(language, catalog) { FdmBanner(state) } }
        }
    }

    private fun assertShown() = compose.onNodeWithTag("fdm_banner").assertIsDisplayed()

    private fun assertGone() = compose.onNodeWithTag("fdm_banner").assertDoesNotExist()

    private fun countdownText() = compose.onNodeWithTag("fdm_banner_countdown")

    @Test
    fun aWarningShowsTheFrenchMessage() {
        show(warning)
        assertShown()
        compose.onNodeWithTag("fdm_banner_message").assertTextEquals("La boîte noire est presque pleine. Programmez une intervention.")
    }

    @Test
    fun aMandatoryMessageIsStillThereAfterTenSeconds() {
        show(warning.copy(mandatory = true))
        assertShown()

        compose.mainClock.advanceTimeBy(FDM_BANNER_AUTO_CLOSE_S * 1000L + 5000L)

        assertShown()
        countdownText().assertDoesNotExist()
    }

    @Test
    fun anOptionalMessageIsStillThereJustBeforeTenSecondsAndGoneAtTen() {
        show(warning)
        assertShown()
        countdownText().assertIsDisplayed()

        compose.mainClock.advanceTimeBy(9000L)
        assertShown()
        compose.onNodeWithTag("fdm_banner_countdown").assertTextEquals("se ferme dans 1 s")

        compose.mainClock.advanceTimeBy(1000L)
        assertGone()
    }

    @Test
    fun theCountdownTicksEachSecond() {
        show(warning)
        compose.onNodeWithTag("fdm_banner_countdown").assertTextEquals("se ferme dans 10 s")

        compose.mainClock.advanceTimeBy(3000L)

        compose.onNodeWithTag("fdm_banner_countdown").assertTextEquals("se ferme dans 7 s")
    }

    @Test
    fun aNewReadingShowsTheBannerAgainWithAFreshCountdown() {
        show(warning)
        compose.mainClock.advanceTimeBy(10_000L)
        assertGone()

        state = warning.copy(code = "UNAUTHORIZED")
        compose.waitForIdle()

        assertShown()
        compose.onNodeWithTag("fdm_banner_countdown").assertTextEquals("se ferme dans 10 s")
    }

    @Test
    fun aReadingThatBecomesMandatoryStopsTheCountdown() {
        show(warning)
        compose.mainClock.advanceTimeBy(4000L)

        state = warning.copy(mandatory = true)
        compose.waitForIdle()
        compose.mainClock.advanceTimeBy(30_000L)

        assertShown()
    }

    @Test
    fun theBannerGoesWhenTheFdmIsBackToNormal() {
        show(warning.copy(mandatory = true))
        assertShown()

        state = FdmUiState(connected = true)
        compose.waitForIdle()

        assertGone()
    }

    @Test
    fun anFdmThatAnswersButIsNotReadyShowsTheBannerWithoutBlocking() {
        show(FdmUiState(connected = false, severity = FdmSeverity.INFO, code = "FDM_NOT_INITIALIZED"))
        assertShown()
        compose.onNodeWithTag("fdm_banner_message").assertTextEquals("La boîte noire n'est pas encore initialisée.")
    }

    @Test
    fun aBlockingReadingBelongsToTheVeilNotTheBanner() {
        show(FdmUiState(connected = false, severity = FdmSeverity.BLOCKING, code = "FDM_LOCKED"))
        assertGone()
    }

    @Test
    fun nothingIsDrawnWithoutAMessage() {
        show(FdmUiState(connected = true, severity = FdmSeverity.WARNING, code = null))
        assertGone()
    }

    @Test
    fun nothingIsDrawnForAHealthyFdm() {
        show(FdmUiState(connected = true, severity = FdmSeverity.INFO, code = "BUFFER_NEAR_FULL"))
        assertGone()
    }

    @Test
    fun theChineseLineFollowsTheMessageInChinese() {
        show(warning, AppLanguage.ZH)
        compose.onNodeWithTag("fdm_banner_message").assertTextEquals("La boîte noire est presque pleine. Programmez une intervention.")
        compose.onNodeWithTag("fdm_banner_message_zh").assertTextEquals("黑匣子快满了。请安排维护。")
    }

    @Test
    fun noChineseLineInFrench() {
        show(warning, AppLanguage.FR)
        compose.onNodeWithTag("fdm_banner_message_zh").assertDoesNotExist()
    }

    @Test
    fun theOverlayDrawsTheBannerUnderTheHeaderBand() {
        val store = mockk<FdmStore>()
        every { store.state } returns MutableStateFlow(warning)
        val settings = mockk<PosSettingsStore>()
        every { settings.language } returns MutableStateFlow(AppLanguage.FR)
        val overlay = FdmBannerOverlay(store, mockk<UiEventToasts>(), settings, catalog)

        compose.setContent { PiliTheme { overlay.Content() } }

        assertEquals(FDM_BANNER_OVERLAY_ORDER, overlay.order)
        assertEquals(200, overlay.order)
        compose.onNodeWithTag("fdm_banner").assertTopPositionInRootIsEqualTo(64.dp)
    }

    @Test
    fun theOverlayDrawsNothingWhileTheFdmIsFine() {
        val store = mockk<FdmStore>()
        every { store.state } returns MutableStateFlow(FdmUiState())
        val settings = mockk<PosSettingsStore>()
        every { settings.language } returns MutableStateFlow(AppLanguage.FR)
        val overlay = FdmBannerOverlay(store, mockk<UiEventToasts>(), settings, catalog)

        compose.setContent { PiliTheme { overlay.Content() } }

        assertGone()
    }
}
