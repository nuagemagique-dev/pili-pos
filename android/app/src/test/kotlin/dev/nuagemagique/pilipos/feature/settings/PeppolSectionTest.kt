package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class PeppolSectionTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val retried = mutableListOf<Long>()

    private fun show(state: PeppolUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    PeppolSection(state, PeppolCallbacks(onRetry = { retried += it }))
                }
            }
        }
        compose.waitForIdle()
    }

    private val manager = PeppolUiState(isManager = true, panel = peppolPanel())

    @Test
    fun theSetupTheCountsAndTheParkedRowsAreShown() {
        show(manager)

        compose.onNodeWithTag("peppol_section").assertIsDisplayed()
        compose.onNodeWithText("storecove").assertIsDisplayed()
        compose.onNodeWithText("0208:0123456789").assertIsDisplayed()
        compose.onNodeWithTag("peppol_queued").assertTextContains("2", substring = true)
        compose.onNodeWithTag("peppol_accepted").assertTextContains("14", substring = true)
        compose.onNodeWithTag("peppol_failed").assertTextContains("1", substring = true)
        compose.onNodeWithTag("peppol_parked_9").assertTextContains("Motif : recipient not on Peppol", substring = true)
        compose.onNodeWithTag("peppol_parked_9").assertTextContains("3 tentative(s)", substring = true)
        // Only the parked delivery is listed, not the one that is waiting its turn.
        compose.onNodeWithTag("peppol_parked_10").assertDoesNotExist()
    }

    @Test
    fun retryCallsBackWithTheDeliveryId() {
        show(manager)

        compose.onNodeWithTag("peppol_retry_9").assertIsEnabled().performClick()

        assertEquals(listOf(9L), retried)
    }

    @Test
    fun retryIsDisabledWhileOneIsRunning() {
        show(manager.copy(retrying = 9L))

        compose.onNodeWithTag("peppol_retry_9").assertIsNotEnabled()
    }

    @Test
    fun withNothingParkedThePanelSaysSo() {
        show(PeppolUiState(isManager = true, panel = peppolPanel(deliveries = emptyList(), failed = 0L)))

        compose.onNodeWithTag("peppol_none_parked").assertIsDisplayed()
    }

    @Test
    fun theReadErrorIsShownAboveTheSetup() {
        show(manager.copy(error = "Peppol n'est pas configuré sur cette installation."))

        compose.onNodeWithTag("peppol_error").assertTextContains("Peppol n'est pas configuré", substring = true)
    }

    @Test
    fun nothingIsDrawnWithoutPeppolOrForANonManager() {
        show(PeppolUiState(isManager = true, panel = peppolPanel(configured = false)))
        compose.onNodeWithTag("peppol_section").assertDoesNotExist()
    }

    @Test
    fun nothingIsDrawnForACashier() {
        show(PeppolUiState(isManager = false, panel = peppolPanel()))

        compose.onNodeWithTag("peppol_section").assertDoesNotExist()
    }
}
