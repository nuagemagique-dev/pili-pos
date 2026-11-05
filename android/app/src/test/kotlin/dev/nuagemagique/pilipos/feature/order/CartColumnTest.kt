package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

/** The cart column (`CartSidebar.vue`): send to kitchen, bill, close unpaid with the manager PIN, pay. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class CartColumnTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val actions =
        OrderCartActions(
            onPay = { seen += "pay" },
            onSend = { seen += "send" },
            onPreBill = { seen += "prebill" },
            onCloseUnpaid = { seen += "unpaid" },
        )

    private val sentLine = CartRow(cartLine(1, 10, "Maki saumon", quantity = 2), LineAmount.Total(1_300))
    private val newLine = CartRow(cartLine(2, 11, "Nigiri thon", quantity = 3, unitPriceCents = 480, unsent = 3), LineAmount.Total(1_440))

    private val dineIn =
        OrderCartState(
            order = order(table = 4, totalCents = 2_740),
            rows = listOf(sentLine, newLine),
            canPay = true,
            vatRows = listOf(VatSplitRow(VatCode.B, 12, 2_740, 294)),
            unsentCount = 3,
        )

    /** Everything sent: PAYER is the Volt primary again. */
    private val allSent = dineIn.copy(rows = listOf(sentLine), unsentCount = 0)

    /** Only a line a course holds back is unsent: it does not count for the primary (C4). */
    private val heldOnly =
        dineIn.copy(
            rows = listOf(sentLine, CartRow(cartLine(3, 12, "Mochi", unsent = 2, kitchenState = "held"), LineAmount.Total(600))),
            unsentCount = 2,
        )

    private fun show(
        state: OrderCartState,
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) { Box { OrderCartColumn(state, actions) } }
            }
        }
        compose.waitForIdle()
    }

    // ── Send to kitchen ───────────────────────────────────────────

    @Test
    fun sendToKitchenIsTheVoltPrimaryWithAPillWhileLinesWait() {
        show(dineIn)

        compose.onNodeWithTag("cart_send_primary").assertIsEnabled()
        compose.onNodeWithTag("cart_send_pill", useUnmergedTree = true).assertTextEquals("1 ligne")
        compose.onNodeWithTag("cart_send").assertDoesNotExist()
        compose.onNodeWithTag("cart_send_primary").performClick()

        assertEquals(listOf("send"), seen)
    }

    @Test
    fun thePillCountsLinesNotUnits() {
        val second = CartRow(cartLine(4, 13, "Edamame", quantity = 5, unsent = 5), LineAmount.Total(1_000))
        show(dineIn.copy(rows = listOf(sentLine, newLine, second), unsentCount = 8))

        compose.onNodeWithTag("cart_send_pill", useUnmergedTree = true).assertTextEquals("2 lignes")
    }

    @Test
    fun payIsAnOutlinedSecondaryThatStillShowsTheTotalWhileLinesWait() {
        show(dineIn)

        compose.onNodeWithTag("cart_pay").assertDoesNotExist()
        compose.onNodeWithTag("cart_pay_secondary").assertIsEnabled()
        compose.onNodeWithTag("cart_pay_secondary_amount", useUnmergedTree = true).assertTextEquals("27,40 €")
        compose.onNodeWithTag("cart_pay_secondary").performClick()

        assertEquals(listOf("pay"), seen)
    }

    @Test
    fun onceEverythingIsSentPayIsThePrimaryAgainAndSendIsASecondary() {
        show(allSent)

        compose.onNodeWithTag("cart_send_primary").assertDoesNotExist()
        compose.onNodeWithTag("cart_pay_secondary").assertDoesNotExist()
        compose.onNodeWithTag("cart_pay").assertIsDisplayed()
        compose.onNodeWithTag("cart_send").assertIsNotEnabled()
    }

    @Test
    fun linesHeldBackByACourseDoNotCountSoPayStaysThePrimary() {
        show(heldOnly)

        compose.onNodeWithTag("cart_send_primary").assertDoesNotExist()
        compose.onNodeWithTag("cart_pay").assertIsDisplayed()
    }

    @Test
    fun sendToKitchenShowsTheUnsentCountAndCallsBack() {
        // Only a held line is unsent: the secondary button is still there and still counts units, as before.
        show(heldOnly)

        compose.onNodeWithTag("cart_send").assertIsEnabled()
        compose.onNodeWithTag("cart_send_count", useUnmergedTree = true).assertTextEquals("2")
        compose.onNodeWithTag("cart_send").performClick()

        assertEquals(listOf("send"), seen)
    }

    @Test
    fun aBowlOptionIsNotALineOfItsOwn() {
        val option = CartRow(cartLine(5, 14, "Bouillon", unsent = 1, bowlLineId = 2), LineAmount.Total(100))
        show(dineIn.copy(rows = listOf(sentLine, newLine, option), unsentCount = 4))

        compose.onNodeWithTag("cart_send_pill", useUnmergedTree = true).assertTextEquals("1 ligne")
    }

    @Test
    fun sendToKitchenIsDisabledWhenNothingIsUnsentForARoleThatCannotPay() {
        show(allSent.copy(canPay = false))

        compose.onNodeWithTag("cart_send_primary").assertIsNotEnabled()
        compose.onNodeWithTag("cart_send_pill", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithTag("cart_send_primary").performClick()

        assertEquals(emptyList<String>(), seen)
    }

    @Test
    fun sendToKitchenIsDisabledWhileItRuns() {
        show(dineIn.copy(sending = true))

        compose.onNodeWithTag("cart_send_primary").assertIsNotEnabled()
        compose.onNodeWithTag("cart_send_primary").performClick()

        assertEquals(emptyList<String>(), seen)
    }

    @Test
    fun sendToKitchenIsDisabledWhenNothingIsUnsent() {
        show(allSent)

        compose.onNodeWithTag("cart_send").assertIsNotEnabled()
        compose.onNodeWithTag("cart_send_count", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithTag("cart_send").performClick()

        assertEquals(emptyList<String>(), seen)
    }

    @Test
    fun aTakeawayHasNeitherSendNorBillNorPendingLabel() {
        show(dineIn.copy(order = order(table = null, totalCents = 2_740)))

        compose.onNodeWithTag("cart_send").assertDoesNotExist()
        compose.onNodeWithTag("cart_send_primary").assertDoesNotExist()
        compose.onNodeWithTag("cart_prebill").assertDoesNotExist()
        compose.onNodeWithTag("cart_pending_label").assertDoesNotExist()
        compose.onNodeWithTag("cart_pay").assertIsDisplayed()
    }

    @Test
    fun theUnsentLinesGoLastUnderTheAmberLabel() {
        show(dineIn)

        compose.onNodeWithTag("cart_pending_label").assertIsDisplayed()
        compose.onNodeWithTag("cart_pending_label").assertTextContains("PAS ENCORE EN CUISINE", substring = true)
        compose.onNodeWithTag("cart_line_1").assertIsDisplayed()
        compose.onNodeWithTag("cart_line_2").assertIsDisplayed()
    }

    // ── Provisional bill ──────────────────────────────────────────

    @Test
    fun theBillButtonCallsBackForADineInOrderWithLines() {
        show(dineIn)

        compose.onNodeWithTag("cart_prebill").assertIsEnabled()
        compose.onNodeWithTag("cart_prebill").performClick()

        assertEquals(listOf("prebill"), seen)
    }

    @Test
    fun theBillButtonIsDisabledWithoutLines() {
        show(dineIn.copy(rows = emptyList(), unsentCount = 0))
        compose.onNodeWithTag("cart_prebill").assertIsNotEnabled()
    }

    @Test
    fun theBillButtonIsDisabledWhilePrinting() {
        show(dineIn.copy(preBilling = true))

        compose.onNodeWithTag("cart_prebill").assertIsNotEnabled()
    }

    // ── Payment right ─────────────────────────────────────────────

    @Test
    fun payAndBillAreHiddenWithoutThePaymentRightButSendStays() {
        show(dineIn.copy(canPay = false))

        compose.onNodeWithTag("cart_pay").assertDoesNotExist()
        compose.onNodeWithTag("cart_pay_secondary").assertDoesNotExist()
        compose.onNodeWithTag("cart_prebill").assertDoesNotExist()
        compose.onNodeWithTag("cart_send_primary").assertIsDisplayed()
        compose.onNodeWithTag("cart_total").assertTextEquals("27,40 €")
    }

    @Test
    fun thePayButtonShowsTheEngineTotalAndCallsBack() {
        show(allSent)

        compose.onNodeWithTag("cart_pay_amount", useUnmergedTree = true).assertTextEquals("27,40 €")
        compose.onNodeWithTag("cart_pay").performClick()

        assertEquals(listOf("pay"), seen)
    }

    // ── Close unpaid: a row of the Plus menu ─────────────────────

    @Test
    fun theUnpaidButtonIsOnlyThereForAManagerOfAnOpenOrder() {
        show(dineIn)
        compose.onNodeWithTag("cart_unpaid").assertDoesNotExist()
        compose.onNodeWithTag("cart_more").assertDoesNotExist()
    }

    @Test
    fun theUnpaidButtonAsksForTheManagerFlowThroughTheCallback() {
        val menu = OrderDialog.CartMore(canTransfer = false, canDiscount = false, orderDiscounted = false, canCloseUnpaid = true)
        val callbacks = OrderCallbacks(onMoreUnpaid = { seen += "unpaid" })
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { Box { CartMoreDialog(menu, callbacks) } } }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("cart_unpaid").assertTextEquals("Client parti sans payer")
        compose.onNodeWithTag("cart_unpaid").performClick()

        assertEquals(listOf("unpaid"), seen)
    }

    @Test
    fun theFooterNoLongerCarriesTheRareActions() {
        show(dineIn.copy(canCloseUnpaid = true, canDiscount = true, canTransfer = true))

        compose.onNodeWithTag("cart_unpaid").assertDoesNotExist()
        compose.onNodeWithTag("cart_discount").assertDoesNotExist()
        compose.onNodeWithTag("cart_transfer").assertDoesNotExist()
        compose.onNodeWithTag("cart_more").assertTextContains("Plus", substring = true)
    }

    @Test
    fun thePlusButtonOpensTheMenuThroughTheCallback() {
        val calls = mutableListOf<String>()
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { OrderCartColumn(dineIn.copy(canCloseUnpaid = true), OrderCartActions(onMore = { calls += "more" })) }
                }
            }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("cart_more").performClick()

        assertEquals(listOf("more"), calls)
    }

    @Test
    fun thePlusButtonIsAbsentWhenNoActionApplies() {
        show(dineIn)

        compose.onNodeWithTag("cart_more").assertDoesNotExist()
    }

    // ── Footer and header ─────────────────────────────────────────

    @Test
    fun theTotalAndTheVatIncludedAreShown() {
        show(dineIn)

        compose.onNodeWithTag("cart_total").assertTextEquals("27,40 €")
        compose.onNodeWithTag("vat_summary").assertTextContains("12%", substring = true)
        compose.onNodeWithTag("vat_summary").assertTextContains("2,94", substring = true)
    }

    @Test
    fun theHeaderSaysWhenTheOrderWasOpenedAndForHowLong() {
        show(dineIn)

        compose.onNodeWithTag("cart_opened").assertTextContains("min", substring = true)
    }

    @Test
    fun anEmptyCartSaysSoAndShowsNoOpenedAtWithoutAnOrder() {
        show(OrderCartState())

        compose.onNodeWithTag("cart_empty").assertIsDisplayed()
        compose.onNodeWithTag("cart_opened").assertDoesNotExist()
        compose.onNodeWithTag("cart_total").assertTextEquals("0,00 €")
    }

    @Test
    fun theCartSpeaksChineseInTheChineseUi() {
        show(dineIn, AppLanguage.ZH)

        compose.onNodeWithTag("cart_title").assertTextContains("桌", substring = true)
    }
}
