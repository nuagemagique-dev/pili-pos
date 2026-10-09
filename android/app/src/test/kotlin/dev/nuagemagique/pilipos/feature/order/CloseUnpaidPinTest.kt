package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.OrderRefusedException
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** "Customer left without paying": the manager PIN dialog of the cart (`PinDialog.vue` over `CartSidebar.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CloseUnpaidPinTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()
    private var refusal: Throwable? = null

    private val callbacks =
        OrderCallbacks(
            onDismiss = { seen += "dismiss" },
            onUnpaidPin = {
                seen += "pin:$it"
                refusal?.let { failure -> throw failure }
            },
            describeError = { "refus:${it.message}" },
            onUnpaidDone = { seen += "done" },
        )

    private fun show() {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { Box { OrderDialogs(OrderDialog.CloseUnpaid("Alice"), callbacks) } }
            }
        }
        compose.waitForIdle()
    }

    private fun type(pin: String) {
        pin.forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.waitForIdle()
    }

    @Test
    fun theDialogAsksForAPinWithTheManagerNameAndTheUnpaidHint() {
        show()

        compose.onNodeWithTag("pin_pad").assertIsDisplayed()
        compose.onNodeWithText("Alice", substring = true).assertIsDisplayed()
        compose.onNodeWithText("sans paiement", substring = true).assertIsDisplayed()
    }

    @Test
    fun theFourthDigitSubmitsThePinAndASuccessCompletesTheFlow() {
        show()

        type("4321")

        assertEquals(listOf("pin:4321", "done"), seen)
    }

    @Test
    fun aRefusedPinShowsTheErrorOnThePadAndDoesNotComplete() {
        refusal = OrderRefusedException("pin_wrong")
        show()

        type("0000")

        assertEquals(listOf("pin:0000"), seen)
        compose.onNodeWithTag("pin_error").assertIsDisplayed()
        compose.onNodeWithText("refus:pin_wrong").assertIsDisplayed()
    }

    @Test
    fun cancelClosesTheDialogWithoutSubmitting() {
        show()

        compose.onNodeWithTag("pin_cancel").performClick()

        assertEquals(listOf("dismiss"), seen)
    }
}
