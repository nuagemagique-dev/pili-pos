package dev.nuagemagique.pilipos.ui.fdm

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.FdmSeverity
import java.time.Clock
import java.time.Instant
import java.time.ZoneId
import java.time.ZoneOffset

/** A clock the test moves by hand. */
private class HandClock(
    var now: Instant,
) : Clock() {
    override fun getZone() = ZoneOffset.UTC

    override fun withZone(zone: ZoneId?): Clock = this

    override fun instant(): Instant = now
}

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class FdmBlockingVeilTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val start = Instant.parse("2026-10-09T10:00:00Z")
    private val clock = HandClock(start)
    private var state by mutableStateOf(FdmUiState())
    private var checks = 0

    private val blocking =
        FdmUiState(connected = false, severity = FdmSeverity.BLOCKING, code = "FDM_NOT_OPERATIONAL")

    private fun show(
        initial: FdmUiState,
        language: AppLanguage = AppLanguage.FR,
    ) {
        state = initial
        compose.setContent {
            PiliTheme { ProvideUiText(language, catalog) { FdmBlockingVeil(state, onCheckNow = { checks += 1 }, clock = clock) } }
        }
    }

    private fun tick(seconds: Int) {
        clock.now = clock.now.plusSeconds(seconds.toLong())
        compose.mainClock.advanceTimeBy(seconds * 1000L)
    }

    @Test
    fun aBlockingReadingCoversTheScreenWithItsMessage() {
        show(blocking)

        compose.onNodeWithTag("fdm_veil").assertIsDisplayed()
        compose.onNodeWithTag("fdm_veil_message")
            .assertTextEquals("La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes.")
        compose.onNodeWithTag("fdm_veil_code").assertTextEquals("FDM_NOT_OPERATIONAL")
        compose.onNodeWithTag("fdm_veil_retry").assertIsDisplayed()
    }

    @Test
    fun aWarningDoesNotShowTheVeil() {
        show(blocking.copy(severity = FdmSeverity.WARNING))
        compose.onNodeWithTag("fdm_veil").assertDoesNotExist()
        compose.onNodeWithTag("fdm_veil_retry").assertDoesNotExist()
    }

    @Test
    fun anInfoReadingDoesNotShowTheVeil() {
        show(blocking.copy(severity = FdmSeverity.INFO))
        compose.onNodeWithTag("fdm_veil").assertDoesNotExist()
    }

    @Test
    fun theVeilAppearsAndGoesWithTheReading() {
        show(FdmUiState(connected = true))
        compose.onNodeWithTag("fdm_veil").assertDoesNotExist()

        state = blocking
        compose.waitForIdle()
        compose.onNodeWithTag("fdm_veil").assertIsDisplayed()

        state = FdmUiState(connected = true)
        compose.waitForIdle()
        compose.onNodeWithTag("fdm_veil").assertDoesNotExist()
    }

    @Test
    fun theCountdownToTheNextTryTicksEachSecond() {
        show(blocking.copy(nextCheckAt = start.plusSeconds(30)))
        compose.onNodeWithTag("fdm_veil_code")
            .assertTextEquals("FDM_NOT_OPERATIONAL · nouvel essai automatique dans 30 s")

        tick(1)
        compose.onNodeWithTag("fdm_veil_code")
            .assertTextEquals("FDM_NOT_OPERATIONAL · nouvel essai automatique dans 29 s")

        tick(28)
        compose.onNodeWithTag("fdm_veil_code")
            .assertTextEquals("FDM_NOT_OPERATIONAL · nouvel essai automatique dans 1 s")
    }

    @Test
    fun theCountdownStopsAtZero() {
        show(blocking.copy(nextCheckAt = start.plusSeconds(2)))

        tick(5)

        compose.onNodeWithTag("fdm_veil_code")
            .assertTextEquals("FDM_NOT_OPERATIONAL · nouvel essai automatique dans 0 s")
    }

    @Test
    fun noNextCheckMeansNoCountdown() {
        show(blocking.copy(nextCheckAt = null))
        tick(3)
        compose.onNodeWithTag("fdm_veil_code").assertTextEquals("FDM_NOT_OPERATIONAL")
    }

    @Test
    fun theButtonAsksForACheckNow() {
        show(blocking)

        compose.onNodeWithTag("fdm_veil_retry").assertIsEnabled().performClick()

        assertEquals(1, checks)
    }

    @Test
    fun theButtonIsDisabledWhileChecking() {
        show(blocking.copy(checking = true))

        compose.onNodeWithTag("fdm_veil_retry").assertIsNotEnabled().performClick()

        assertEquals(0, checks)
    }

    @Test
    fun aRawTransportErrorShowsTheConnectionMessageAndTheFallbackCode() {
        show(blocking.copy(code = "error sending request: connect refused"))

        compose.onNodeWithTag("fdm_veil_message").assertTextEquals("Connexion perdue avec la boîte noire. Vérifiez le câble réseau.")
        compose.onNodeWithTag("fdm_veil_code").assertTextEquals("FDM_NOT_OPERATIONAL")
    }

    @Test
    fun theChineseLineIsShownInChinese() {
        show(blocking, AppLanguage.ZH)
        compose.onNodeWithTag("fdm_veil_message_zh").assertTextEquals("黑匣子无法工作，无法登记销售。")
    }

    @Test
    fun noChineseLineInFrench() {
        show(blocking, AppLanguage.FR)
        compose.onNodeWithTag("fdm_veil_message_zh").assertDoesNotExist()
    }

    @Test
    fun theVeilSwallowsTheTouchesMeantForTheScreenBelow() {
        var below = 0
        state = blocking
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) {
                        Box(Modifier.fillMaxSize().clickable { below += 1 }.testTag("below"))
                        FdmBlockingVeil(state, onCheckNow = {}, clock = clock)
                    }
                }
            }
        }

        compose.onNodeWithTag("fdm_veil").performTouchInput { click(Offset(4f, 4f)) }

        assertEquals(0, below)
    }

    @Test
    fun secondsUntilRoundsUpAndNeverGoesBelowZero() {
        val now = Instant.parse("2026-10-09T10:00:00Z")
        assertNull(secondsUntil(null, now))
        assertEquals(0L, secondsUntil(now.minusSeconds(3), now))
        assertEquals(0L, secondsUntil(now, now))
        assertEquals(1L, secondsUntil(now.plusMillis(1), now))
        assertEquals(1L, secondsUntil(now.plusMillis(1000), now))
        assertEquals(2L, secondsUntil(now.plusMillis(1001), now))
        assertEquals(30L, secondsUntil(now.plusSeconds(30), now))
    }
}
