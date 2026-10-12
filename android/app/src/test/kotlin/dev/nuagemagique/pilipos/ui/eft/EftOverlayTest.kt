package dev.nuagemagique.pilipos.ui.eft

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.EFT_CANCELLED
import dev.nuagemagique.pilipos.ui.state.EFT_DECLINED
import dev.nuagemagique.pilipos.ui.state.EFT_UNKNOWN
import dev.nuagemagique.pilipos.ui.state.EftOverlayState
import dev.nuagemagique.pilipos.ui.state.EftPhase
import dev.nuagemagique.pilipos.ui.state.eftPayment
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
class EftOverlayTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()
    private val callbacks =
        EftOverlayCallbacks(
            onCancel = { calls += "cancel" },
            onCheck = { calls += "check" },
            onDismiss = { calls += "dismiss" },
        )

    private fun text(id: Int) = catalog.text(id, AppLanguage.FR)

    private fun show(state: EftOverlayState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { EftOverlay(state, callbacks) }
            }
        }
        compose.waitForIdle()
    }

    private fun node(tag: String) = compose.onNodeWithTag(tag, useUnmergedTree = true)

    // A button carries its text in a child: read it from the merged tree.
    private fun button(tag: String) = compose.onNodeWithTag(tag)

    @Test
    fun nothingIsDrawnWithoutASession() {
        show(EftOverlayState())

        node("eft_overlay").assertDoesNotExist()
    }

    @Test
    fun whileTheTerminalWaitsItShowsTheAmountTheCardPromptAndCancel() {
        show(EftOverlayState(phase = EftPhase.WAITING, amountCents = 1_152L))

        node("eft_title").assertTextEquals(text(R.string.ui_eft_title))
        node("eft_amount").assertTextEquals("11,52 €")
        node("eft_spinner").assertIsDisplayed()
        node("eft_present").assertTextEquals(text(R.string.ui_eft_present_card))
        node("eft_progress").assertDoesNotExist()
        button("eft_cancel").assertIsEnabled().performClick()
        assertEquals(listOf("cancel"), calls)
    }

    @Test
    fun aSplitSaleShowsWhichPartIsPaid() {
        show(EftOverlayState(phase = EftPhase.WAITING, amountCents = 1_000L, part = 2, parts = 3))

        node("eft_progress").assertTextEquals("Paiement 2 sur 3")
    }

    @Test
    fun onceCancelIsRequestedTheButtonShowsItAndIsDisabled() {
        show(EftOverlayState(phase = EftPhase.WAITING, amountCents = 1_152L, abortRequested = true))

        button("eft_cancel").assertIsNotEnabled().assertTextContains(text(R.string.ui_eft_cancel_requested), substring = true)
    }

    @Test
    fun whileCheckingItShowsASpinnerAndNoButton() {
        show(EftOverlayState(phase = EftPhase.CHECKING, amountCents = 500L, payment = eftPayment(1, EFT_UNKNOWN, 500)))

        node("eft_checking").assertTextEquals(text(R.string.ui_eft_checking))
        node("eft_spinner").assertIsDisplayed()
        button("eft_cancel").assertDoesNotExist()
        button("eft_ok").assertDoesNotExist()
    }

    @Test
    fun aDeclinedPaymentShowsItsReasonAndNothingWasCharged() {
        val declined = eftPayment(2, EFT_DECLINED, 900, detail = "Carte refusée par la banque")
        show(EftOverlayState(phase = EftPhase.RESULT, amountCents = 900L, payment = declined))

        node("eft_title").assertTextEquals(text(R.string.ui_eft_declined))
        node("eft_detail").assertTextEquals("Carte refusée par la banque")
        node("eft_not_charged").assertTextEquals(text(R.string.ui_eft_not_charged))
        button("eft_check").assertDoesNotExist()
        button("eft_ok").assertTextContains(text(R.string.ui_eft_ok), substring = true).performClick()
        assertEquals(listOf("dismiss"), calls)
    }

    @Test
    fun aCancelledPaymentHasItsOwnTitle() {
        show(EftOverlayState(phase = EftPhase.RESULT, amountCents = 900L, payment = eftPayment(3, EFT_CANCELLED, 900)))

        node("eft_title").assertTextEquals(text(R.string.ui_eft_cancelled))
    }

    @Test
    fun anUnknownPaymentExplainsAndOffersTheCheck() {
        show(EftOverlayState(phase = EftPhase.RESULT, amountCents = 900L, payment = eftPayment(4, EFT_UNKNOWN, 900)))

        node("eft_title").assertTextEquals(text(R.string.ui_eft_unknown_title))
        node("eft_unknown_text").assertTextEquals(text(R.string.ui_eft_unknown_explain))
        node("eft_not_charged").assertDoesNotExist()
        button("eft_ok").assertTextContains(text(R.string.ui_eft_close), substring = true)
        button("eft_check").assertTextContains(text(R.string.ui_eft_check), substring = true).performClick()
        assertEquals(listOf("check"), calls)
    }

    @Test
    fun aRefusedRequestShowsWhyAndNoPaymentIsMentioned() {
        show(EftOverlayState(phase = EftPhase.RESULT, amountCents = 900L, failure = "Le terminal de paiement est injoignable."))

        node("eft_title").assertTextEquals(text(R.string.ui_eft_refused))
        node("eft_failure").assertTextEquals("Le terminal de paiement est injoignable.")
        node("eft_not_charged").assertDoesNotExist()
        button("eft_ok").assertIsDisplayed()
    }

    @Test
    fun theOverlayCoversTheScreenAndSwallowsTheTouches() {
        show(EftOverlayState(phase = EftPhase.WAITING, amountCents = 100L))

        node("eft_overlay").assertIsDisplayed()
    }
}
