package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The pad of the first takeaway number (UX audit T2): 1 to 999, validated before it leaves the dialog. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TakeawayFirstDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private fun show(typed: String) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    TakeawayFirstDialog(
                        typed,
                        TakeawayFirstActions(
                            onKey = { calls += "key:$it" },
                            onConfirm = { calls += "ok" },
                            onCancel = { calls += "cancel" },
                        ),
                    )
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theTypedNumberShowsAboveThePadAndAKeyIsHandedBack() {
        show("61")

        compose.onNodeWithTag("takeaway_first_value").assertTextEquals("61")
        compose.onNodeWithTag("numpad_key_7").performClick()
        assertEquals(listOf("key:${NumPadKey.Digit('7')}"), calls)
    }

    @Test
    fun theValidateButtonIsOnForOneToNineHundredNinetyNineOnly() {
        show("61")
        compose.onNodeWithTag("takeaway_first_ok").assertIsEnabled()
        compose.onNodeWithTag("takeaway_first_ok").performClick()
        compose.onNodeWithTag("takeaway_first_cancel").performClick()
        assertEquals(listOf("ok", "cancel"), calls)

        assertTrue(takeawayFirstValid("1"))
        assertTrue(takeawayFirstValid("999"))
        assertFalse(takeawayFirstValid("0"))
        assertFalse(takeawayFirstValid("1000"))
        assertFalse(takeawayFirstValid(""))
        assertFalse(takeawayFirstValid(null))
    }

    @Test
    fun anEmptyEntryCannotBeValidated() {
        show("")

        compose.onNodeWithTag("takeaway_first_value").assertTextEquals("0")
        compose.onNodeWithTag("takeaway_first_ok").assertIsNotEnabled()
    }
}
