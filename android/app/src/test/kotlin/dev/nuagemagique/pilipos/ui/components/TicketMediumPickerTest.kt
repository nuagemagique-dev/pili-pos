package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TicketMediumPickerTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val paper = TicketDeliveryRequest(RequestedMedium.PAPER, null)

    /** What the callers received, in order: `paper`, or `medium:address`. */
    private val calls = mutableListOf<String>()
    private lateinit var pickerState: TicketMediumPickerState

    private fun show(
        status: MailStatus?,
        initial: TicketDeliveryRequest = paper,
        language: AppLanguage = AppLanguage.FR,
    ) {
        var mail by mutableStateOf(status)
        var request by mutableStateOf(initial)
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) {
                    pickerState = rememberTicketMediumPickerState()
                    Box(Modifier.fillMaxSize()) {
                        Column {
                            TicketMediumPicker(
                                mailStatus = mail,
                                request = request,
                                state = pickerState,
                                onPaper = {
                                    calls += "paper"
                                    request = paper
                                },
                                texts = defaultTicketMediumPickerTexts(),
                            )
                        }
                        TicketMediumKeypad(
                            state = pickerState,
                            texts = defaultEmailKeypadTexts(),
                            onEmail = { medium, email ->
                                calls += "$medium:$email"
                                request = TicketDeliveryRequest(medium, email)
                            },
                            onPaper = {
                                calls += "paper"
                                request = paper
                            },
                        )
                    }
                }
            }
        }
    }

    private fun type(text: String) {
        text.forEach { compose.onNodeWithTag("email_key_$it").performClick() }
    }

    @Test
    fun isHiddenWithoutAMailStatus() {
        show(null)
        compose.onNodeWithTag("ticket_medium_picker").assertDoesNotExist()
    }

    @Test
    fun isHiddenWhenMailIsNotConfigured() {
        show(MailStatus(configured = false, reachable = true, allowDigitalOnly = true))
        compose.onNodeWithTag("ticket_medium_picker").assertDoesNotExist()
        compose.onNodeWithTag("medium_paper").assertDoesNotExist()
    }

    @Test
    fun offersPaperAndBothWhenDigitalOnlyIsNotAllowed() {
        show(MailStatus(configured = true, reachable = true, allowDigitalOnly = false))
        compose.onNodeWithTag("medium_paper").assertTextEquals("Papier")
        compose.onNodeWithTag("medium_both").assertTextEquals("Les deux")
        compose.onNodeWithTag("medium_digital").assertDoesNotExist()
        compose.onNodeWithTag("medium_note").assertDoesNotExist()
    }

    @Test
    fun offersEmailOnlyWhenAllowedAndReachable() {
        show(MailStatus(configured = true, reachable = true, allowDigitalOnly = true))
        compose.onNodeWithTag("medium_digital").assertTextEquals("E-mail")
    }

    @Test
    fun hidesEmailOnlyWhenAllowedButUnreachableAndShowsTheNote() {
        show(MailStatus(configured = true, reachable = false, allowDigitalOnly = true))
        compose.onNodeWithTag("medium_digital").assertDoesNotExist()
        compose.onNodeWithTag("medium_note").assertIsDisplayed()
        compose.onNodeWithTag("medium_note").assertTextEquals(
            "Service e-mail indisponible : le ticket papier sera imprimé, l'e-mail sera envoyé dès que possible.",
        )
    }

    @Test
    fun theTextsFollowTheLanguage() {
        show(MailStatus(configured = true, reachable = true, allowDigitalOnly = true), language = AppLanguage.EN)
        compose.onNodeWithTag("medium_paper").assertTextEquals("Paper")
    }

    @Test
    fun paperCallsBackWithoutOpeningTheKeypad() {
        show(MailStatus(true, true, true), initial = TicketDeliveryRequest(RequestedMedium.DIGITAL, "a@b.be"))
        compose.onNodeWithTag("medium_paper").performClick()
        assertEquals(listOf("paper"), calls)
        compose.onNodeWithTag("email_keypad").assertDoesNotExist()
    }

    @Test
    fun bothOpensTheKeypadAndConfirmGivesTheMediumAndTheAddress() {
        show(MailStatus(true, true, true))
        compose.onNodeWithTag("medium_both").performClick()
        compose.onNodeWithTag("email_keypad").assertIsDisplayed()
        type("a@b.be")
        compose.onNodeWithTag("email_validate").performClick()
        compose.onNodeWithTag("email_confirm").performClick()
        assertEquals(listOf("PAPER_DIGITAL:a@b.be"), calls)
        compose.onNodeWithTag("email_keypad").assertDoesNotExist()
        assertNull(pickerState.pendingMedium)
    }

    @Test
    fun emailOnlyOpensTheKeypadForTheDigitalMedium() {
        show(MailStatus(true, true, true))
        compose.onNodeWithTag("medium_digital").performClick()
        assertEquals(RequestedMedium.DIGITAL, pickerState.pendingMedium)
        type("a@b.be")
        compose.onNodeWithTag("email_validate").performClick()
        compose.onNodeWithTag("email_confirm").performClick()
        assertEquals(listOf("DIGITAL:a@b.be"), calls)
    }

    @Test
    fun cancellingTheKeypadReturnsToPaper() {
        show(MailStatus(true, true, true))
        compose.onNodeWithTag("medium_both").performClick()
        type("a@b")
        compose.onNodeWithTag("email_cancel").performClick()
        assertEquals(listOf("paper"), calls)
        compose.onNodeWithTag("email_keypad").assertDoesNotExist()
        assertNull(pickerState.pendingMedium)
    }

    @Test
    fun theKeypadOpensEmptyEachTime() {
        show(MailStatus(true, true, true))
        compose.onNodeWithTag("medium_both").performClick()
        type("abc")
        compose.onNodeWithTag("email_cancel").performClick()
        compose.onNodeWithTag("medium_both").performClick()
        compose.onNodeWithTag("email_placeholder").assertIsDisplayed()
    }
}
