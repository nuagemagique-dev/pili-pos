package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The words and keys of the customer of a numbered takeaway (UX audit T2), mirroring `utils/takeaway.ts`. */
class TakeawayTextTest {
    @Test
    fun theTitleCarriesTheNumberAndTheCustomerAsFarAsKnown() {
        assertEquals("À emporter", TakeawayText.title("À emporter", null, null))
        assertEquals("À emporter E-61", TakeawayText.title("À emporter", "E-61", null))
        assertEquals("À emporter E-61", TakeawayText.title("À emporter", "E-61", "  "))
        assertEquals("À emporter E-61 · Dupont", TakeawayText.title("À emporter", "E-61", " Dupont "))
    }

    @Test
    fun thePhoneKeepsDigitsOnlyUpToFifteenAndBackspaceAndClearWork() {
        var digits = ""
        "0470123456789012345".forEach { digits = PhoneEntry.press(digits, NumPadKey.Digit(it)) }
        assertEquals("047012345678901", digits)
        assertEquals("04701234567890", PhoneEntry.press(digits, NumPadKey.Backspace))
        assertEquals("", PhoneEntry.press(digits, NumPadKey.Clear))
    }

    @Test
    fun thePhoneIsShownAndSentInGroups() {
        assertEquals("04", PhoneEntry.display("04"))
        assertEquals("0470", PhoneEntry.display("0470"))
        assertEquals("0470 12 34 56", PhoneEntry.display("0470123456"))
        assertEquals("0470 12 34 56", PhoneEntry.value("0470123456"))
        assertNull(PhoneEntry.value(""))
    }

    @Test
    fun theNameIsCutAtThirtyCharactersWithoutControlCharacters() {
        assertEquals("Dupont", customerNameText("Dupont"))
        assertEquals(30, customerNameText("x".repeat(45)).length)
        assertEquals("ab", customerNameText("a\nb"))
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TakeawayCustomerDialogsTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private fun show(content: @androidx.compose.runtime.Composable () -> Unit) {
        compose.setContent { PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { content() } } }
        compose.waitForIdle()
    }

    @Test
    fun theNameDialogStartsOnTheCurrentNameAndConfirmsWhatWasTyped() {
        show { CustomerNameDialog("Dupont", { calls += "name:$it" }, { calls += "cancel" }) }

        compose.onNodeWithTag("customer_name_dialog").assertIsDisplayed()
        compose.onNodeWithTag("customer_name_counter").assertTextEquals("6/30")
        compose.onNodeWithTag("customer_name_text").performTextInput("-Martin")
        compose.onNodeWithTag("customer_name_counter").assertTextEquals("13/30")
        assertTrue(compose.onNodeWithTag("customer_name_confirm").getUnclippedBoundsInRoot().height >= 48.dp)
        compose.onNodeWithTag("customer_name_confirm").performClick()

        assertEquals(listOf("name:Dupont-Martin"), calls)
    }

    @Test
    fun theNameDialogStopsAtThirtyCharactersAndCancelsWithoutSaving() {
        show { CustomerNameDialog("", { calls += "name:$it" }, { calls += "cancel" }) }

        compose.onNodeWithTag("customer_name_text").performTextInput("x".repeat(40))
        compose.onNodeWithTag("customer_name_counter").assertTextEquals("30/30")
        compose.onNodeWithTag("customer_name_cancel").performClick()

        assertEquals(listOf("cancel"), calls)
    }

    @Test
    fun thePhoneDialogTypesDigitsInGroupsAndConfirmsThem() {
        show { CustomerPhoneDialog(null, { calls += "phone:$it" }, { calls += "skip" }) }

        compose.onNodeWithTag("customer_phone_value").assertTextEquals("Pas de numéro")
        "0470123456".forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.onNodeWithTag("customer_phone_value").assertTextEquals("0470 12 34 56")
        compose.onNodeWithTag("customer_phone_confirm").performClick()

        assertEquals(listOf("phone:0470123456"), calls)
    }

    @Test
    fun thePhoneDialogStartsOnTheSavedPhoneAndSkipLeavesItAlone() {
        show { CustomerPhoneDialog("0470 12 34 56", { calls += "phone:$it" }, { calls += "skip" }) }

        compose.onNodeWithTag("customer_phone_value").assertTextEquals("0470 12 34 56")
        compose.onNodeWithTag("customer_phone_skip").performClick()

        assertEquals(listOf("skip"), calls)
    }
}
