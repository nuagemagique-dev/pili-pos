package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.state.EFT_UNKNOWN
import dev.nuagemagique.pilipos.ui.state.eftPayment
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod

/** The pay step with a card terminal linked: the terminal / manual choice, the locked parts and the waiting payments. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CheckoutEftContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        CheckoutCallbacks(
            onCardManual = { calls += "manual:$it" },
            onVerify = { calls += "verify:${it.id}" },
            onClearParts = { calls += "clear" },
            onBack = { calls += "back" },
            onValidate = { calls += "validate" },
        )

    private val card =
        CheckoutUiState(
            orderId = ORDER_ID,
            hasOrder = true,
            loadTried = true,
            orderType = OrderType.DINE_IN,
            tableNumber = 5,
            totalCents = 1_152L,
            canTakePayment = true,
            method = PaymentMethod.CARD,
            entry = AmountEntryState("1152"),
            fresh = true,
            amounts = CheckoutAmounts(restCents = 1_152L, dueCents = 1_152L, roundedDueCents = 1_152L, restShownCents = 1_152L),
            eftActive = true,
        )

    private fun ui(id: Int) = catalog.text(id, AppLanguage.FR)

    private fun show(state: CheckoutUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { CheckoutContent(state, callbacks) }
            }
        }
        compose.waitForIdle()
    }

    private fun node(tag: String) = compose.onNodeWithTag(tag, useUnmergedTree = true)

    // A button carries its text in a child: read it from the merged tree.
    private fun button(tag: String) = compose.onNodeWithTag(tag)

    @Test
    fun aCardPartOffersTheTerminalAndManualEntryAndHintsThatTheAmountIsSent() {
        show(card)

        button("checkout_eft_mode_terminal").assertTextContains(ui(R.string.ui_eft_mode_terminal), substring = true)
        button("checkout_eft_mode_manual").assertTextContains(ui(R.string.ui_eft_mode_manual), substring = true)
        node("checkout_card_hint").assertTextEquals(ui(R.string.ui_eft_hint_terminal))
        button("checkout_eft_mode_manual").performClick()
        button("checkout_eft_mode_terminal").performClick()
        assertEquals(listOf("manual:true", "manual:false"), calls)
    }

    @Test
    fun manualEntryShowsTheUsualCardHint() {
        show(card.copy(cardManual = true))

        node("checkout_card_hint").assertTextEquals(catalog.text("card_wait", AppLanguage.FR))
    }

    @Test
    fun withoutATerminalThereIsNoChoice() {
        show(card.copy(eftActive = false))

        node("checkout_eft_mode").assertDoesNotExist()
        node("checkout_card_hint").assertTextEquals(catalog.text("card_wait", AppLanguage.FR))
    }

    @Test
    fun cashHasNoTerminalChoice() {
        show(card.copy(method = PaymentMethod.CASH))

        node("checkout_eft_mode").assertDoesNotExist()
    }

    @Test
    fun anApprovedPartIsShownWithItsBrandAndTheApprovedBadge() {
        val locked = TakenPart(PaymentMethod.CARD, 800L, 0L, eftPaymentId = 12L, brand = "Bancontact")
        show(card.copy(method = PaymentMethod.CASH, taken = listOf(locked)))

        node("checkout_part_locked_0").assertIsDisplayed()
        compose.onNodeWithText("Bancontact", substring = true).assertIsDisplayed()
        node("checkout_part_eft_0").assertTextEquals(ui(R.string.ui_eft_approved))
        node("checkout_part_0").assertTextEquals("− 8,00 €")
    }

    @Test
    fun aPartTypedByHandHasNoBadge() {
        show(card.copy(method = PaymentMethod.CASH, taken = listOf(TakenPart(PaymentMethod.CARD, 800L, 0L))))

        node("checkout_part_eft_0").assertDoesNotExist()
        node("checkout_part_0").assertTextEquals("− 8,00 €")
    }

    @Test
    fun onlyLockedPartsLeaveBackInsteadOfClear() {
        val locked = TakenPart(PaymentMethod.CARD, 800L, 0L, eftPaymentId = 12L, brand = null)
        show(card.copy(taken = listOf(locked)))

        button("checkout_clear_parts").assertDoesNotExist()
        button("checkout_back").performClick()
        assertEquals(listOf("back"), calls)
    }

    @Test
    fun aHandTypedPartCanStillBeCleared() {
        val locked = TakenPart(PaymentMethod.CARD, 800L, 0L, eftPaymentId = 12L, brand = null)
        show(card.copy(taken = listOf(locked, TakenPart(PaymentMethod.CARD, 100L, 0L))))

        button("checkout_clear_parts").performClick()
        assertEquals(listOf("clear"), calls)
    }

    @Test
    fun aWaitingPaymentShowsVerifyAndBlocksTheSale() {
        val unknown = eftPayment(33, EFT_UNKNOWN, 1_152)
        show(card.copy(eftOpen = listOf(unknown)))

        compose.onNodeWithText(ui(R.string.ui_eft_unknown_row), substring = true).assertIsDisplayed()
        node("checkout_eft_blocking").assertTextEquals(ui(R.string.ui_eft_blocking))
        button("checkout_validate").assertIsNotEnabled()
        button("checkout_eft_verify_33").performClick()
        assertEquals(listOf("verify:33"), calls)
    }

    @Test
    fun whenEverythingIsPaidByTheTerminalValidateSignsTheSale() {
        val locked = TakenPart(PaymentMethod.CARD, 1_152L, 0L, eftPaymentId = 12L, brand = "Maestro")
        show(
            card.copy(
                entry = AmountEntryState(""),
                taken = listOf(locked),
                amounts = card.amounts.copy(restCents = 0L, dueCents = 0L, restShownCents = 0L, allPaid = true),
            ),
        )

        button("checkout_validate").assertIsEnabled().performClick()
        assertEquals(listOf("validate"), calls)
    }
}
