package dev.nuagemagique.pilipos.ui.components

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.IdleWarning
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class IdleLockBannerTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private fun show(warning: IdleWarning) {
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { IdleLockBanner(warning) } }
        }
        compose.waitForIdle()
    }

    @Test
    fun theBannerAnnouncesTheLockWithTheSecondsLeft() {
        compose.mainClock.autoAdvance = false
        show(IdleWarning(30_000L, 1L))

        compose.onNodeWithTag("idle_lock_banner").assertIsDisplayed()
        compose.onNodeWithTag("idle_lock_text").assertTextEquals("Verrouillage dans 30 s · touchez l'écran pour rester connecté")
        assertTrue("56 dp high", compose.onNodeWithTag("idle_lock_banner").getUnclippedBoundsInRoot().height >= 48.dp)
    }

    @Test
    fun theCountdownIsLive() {
        compose.mainClock.autoAdvance = false
        show(IdleWarning(30_000L, 1L))

        compose.mainClock.advanceTimeBy(5_100)
        compose.onNodeWithTag("idle_lock_text").assertTextEquals("Verrouillage dans 25 s · touchez l'écran pour rester connecté")
        compose.mainClock.advanceTimeBy(20_000)
        compose.onNodeWithTag("idle_lock_text").assertTextEquals("Verrouillage dans 5 s · touchez l'écran pour rester connecté")
    }

    @Test
    fun aSecondWarningRestartsTheCountdownFromItsOwnTime() {
        compose.mainClock.autoAdvance = false
        var warning by mutableStateOf(IdleWarning(30_000L, 1L))
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { IdleLockBanner(warning) } }
        }
        compose.mainClock.advanceTimeBy(10_100)
        compose.onNodeWithTag("idle_lock_text").assertTextEquals("Verrouillage dans 20 s · touchez l'écran pour rester connecté")

        warning = IdleWarning(30_000L, 2L)
        compose.mainClock.advanceTimeBy(100)
        compose.onNodeWithTag("idle_lock_text").assertTextEquals("Verrouillage dans 30 s · touchez l'écran pour rester connecté")
    }

    @Test
    fun theSecondsAreRoundedUpAndNeverNegative() {
        assertEquals(30, secondsLeft(30_000L))
        assertEquals(30, secondsLeft(29_001L))
        assertEquals(1, secondsLeft(1L))
        assertEquals(0, secondsLeft(0L))
        assertEquals(0, secondsLeft(-250L))
    }
}
