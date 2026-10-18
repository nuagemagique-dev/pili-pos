package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
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
import uniffi.pos_ffi.GiftVoucherView

/** The gift voucher helpers shared by the checkout dialog and the reports tab. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class GiftVouchersTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private fun voucher(
        status: String = "active",
        training: Boolean = false,
    ) = GiftVoucherView(7L, "**** **** 9095", 5_000L, 3_000L, "2026-10-01", "2027-10-01", status, training, emptyList())

    @Test
    fun theCodeTakesTwelveDigitsAndNothingElse() {
        var digits = ""
        "1234567890123".forEach { digits = pressVoucherKey(digits, NumPadKey.Digit(it)) }

        assertEquals("123456789012", digits)
        assertEquals("12345678901", pressVoucherKey(digits, NumPadKey.Backspace))
        assertEquals("", pressVoucherKey(digits, NumPadKey.Clear))
        assertEquals("", pressVoucherKey("", NumPadKey.Backspace))
    }

    @Test
    fun theCodeIsShownInGroupsOfFourAsPrinted() {
        assertEquals("", spacedVoucherCode(""))
        assertEquals("12", spacedVoucherCode("12"))
        assertEquals("1234 5678 9095", spacedVoucherCode("123456789095"))
    }

    @Test
    fun aDateIsShownDayFirst() {
        assertEquals("17.10.2027", voucherDate("2027-10-17"))
        assertEquals("not a date", voucherDate("not a date"))
    }

    private fun show(
        view: GiftVoucherView,
        use: Long?,
    ) {
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { GiftVoucherSummary(view, useCents = use) } }
        }
        compose.waitForIdle()
    }

    @Test
    fun theSummaryShowsTheMaskedCodeTheStatusTheBalanceTheExpiryAndTheAmountUsed() {
        show(voucher(), use = 2_000L)

        compose.onNodeWithTag("gv_found").assertIsDisplayed()
        compose.onNodeWithTag("gv_status").assertTextContains("Valide", substring = true)
        compose.onNodeWithTag("gv_balance").assertTextContains("30,00", substring = true)
        compose.onNodeWithTag("gv_amount").assertTextContains("20,00", substring = true)
    }

    @Test
    fun aVoucherThatIsNotActiveShowsNoAmountToUse() {
        show(voucher(status = "used", training = true), use = 2_000L)

        compose.onNodeWithTag("gv_status").assertTextContains("Épuisé", substring = true)
        compose.onNodeWithTag("gv_amount").assertDoesNotExist()
    }
}
