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
import uniffi.pos_ffi.DiscountInput
import uniffi.pos_ffi.DiscountKind
import uniffi.pos_ffi.DiscountReason
import uniffi.pos_ffi.DiscountTarget
import uniffi.pos_ffi.PosException

/** The manager PIN of a discount and of its removal (`PinDialog.vue` over `CartSidebar.vue`, `discount_hint` / `discount_remove_hint`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class OrderDiscountPinTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()
    private var refusal: Throwable? = null

    private val input = DiscountInput(DiscountTarget.Order, DiscountKind.PERCENT, 10L, DiscountReason.COMMERCIAL_GESTURE, null)

    private val callbacks =
        OrderCallbacks(
            onDismiss = { seen += "dismiss" },
            onDiscountPin = {
                seen += "pin:$it"
                refusal?.let { failure -> throw failure }
            },
            describeError = { "refus:" + ((it as? PosException.Coded)?.code ?: it.message) },
            onDiscountDone = { seen += "done" },
        )

    private fun show(action: DiscountAction) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { Box { OrderDialogs(OrderDialog.DiscountPin("Alice", action), callbacks) } }
            }
        }
        compose.waitForIdle()
    }

    private fun type(pin: String) {
        pin.forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.waitForIdle()
    }

    @Test
    fun applyingAsksForThePinWithTheManagerNameAndTheFiscalTicketHint() {
        show(DiscountAction.Apply(input))

        compose.onNodeWithTag("pin_pad").assertIsDisplayed()
        compose
            .onNodeWithText("Alice\nLa remise figure sur le ticket fiscal. Confirmez avec votre PIN.")
            .assertIsDisplayed()
    }

    @Test
    fun removingAsksForThePinWithTheOriginalPriceHint() {
        show(DiscountAction.Remove(null))

        compose.onNodeWithText("Alice\nLe prix d'origine est rétabli. Confirmez avec votre PIN.").assertIsDisplayed()
    }

    @Test
    fun removingAnOfferedLineAsksTheSameHint() {
        show(DiscountAction.Remove(7L))

        compose.onNodeWithText("Alice\nLe prix d'origine est rétabli. Confirmez avec votre PIN.").assertIsDisplayed()
    }

    @Test
    fun theFourthDigitSubmitsThePinAndASuccessCompletesTheFlowOnce() {
        show(DiscountAction.Apply(input))

        type("4321")

        assertEquals(listOf("pin:4321", "done"), seen)
    }

    @Test
    fun aRefusedPinShowsTheErrorOnThePadAndDoesNotComplete() {
        refusal = OrderRefusedException("pin_wrong")
        show(DiscountAction.Apply(input))

        type("0000")

        assertEquals(listOf("pin:0000"), seen)
        compose.onNodeWithTag("pin_error").assertIsDisplayed()
        compose.onNodeWithText("refus:pin_wrong").assertIsDisplayed()
    }

    @Test
    fun anEngineRefusalOfARemovalIsShownTheSameWay() {
        refusal = PosException.Coded("discount_none", emptyMap())
        show(DiscountAction.Remove(null))

        type("4321")

        assertEquals(listOf("pin:4321"), seen)
        compose.onNodeWithText("refus:discount_none").assertIsDisplayed()
    }

    @Test
    fun cancelClosesTheDialogWithoutSubmitting() {
        show(DiscountAction.Apply(input))

        compose.onNodeWithTag("pin_cancel").performClick()

        assertEquals(listOf("dismiss"), seen)
    }
}
