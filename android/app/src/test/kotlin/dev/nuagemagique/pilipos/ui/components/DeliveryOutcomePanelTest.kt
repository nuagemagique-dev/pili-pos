package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.CompletableDeferred
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.TicketMedium

internal fun deliveryOutcome(
    email: EmailStatus = EmailStatus.NOT_REQUESTED,
    paperPrinted: Boolean = false,
    needsPaper: Boolean = false,
    hint: String? = null,
    fiscalEventId: Long = 77L,
): DeliveryOutcome =
    DeliveryOutcome(
        fiscalEventId = fiscalEventId,
        deliveryId = null,
        medium = TicketMedium.DIGITAL,
        email = email,
        recipientHint = hint,
        paperPrinted = paperPrinted,
        needsPaper = needsPaper,
    )

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class DeliveryOutcomePanelTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val printed = mutableListOf<Long>()

    private fun show(
        outcome: DeliveryOutcome?,
        canPrint: Boolean = true,
        language: AppLanguage = AppLanguage.FR,
        result: suspend (Long) -> Boolean = { true },
        fiscalEventId: Long? = outcome?.fiscalEventId,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) {
                    DeliveryOutcomePanel(
                        outcome = outcome,
                        texts = defaultDeliveryOutcomeTexts(outcome),
                        canPrintMissing = canPrint,
                        onPrintMissing = { id ->
                            printed += id
                            result(id)
                        },
                        fiscalEventId = fiscalEventId,
                    )
                }
            }
        }
    }

    @Test
    fun drawsNothingWithoutAnOutcome() {
        show(null)
        compose.onNodeWithTag("delivery_outcome").assertDoesNotExist()
    }

    @Test
    fun saysWhereTheTicketWent() {
        show(deliveryOutcome(email = EmailStatus.SENT, hint = "j***@example.be"))
        compose.onNodeWithTag("delivery_message").assertTextEquals("Ticket envoyé à j***@example.be")
        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
    }

    @Test
    fun saysThatTheEmailIsQueued() {
        show(deliveryOutcome(email = EmailStatus.QUEUED))
        compose.onNodeWithTag("delivery_message").assertTextEquals("E-mail en attente, nouvel essai automatique")
    }

    @Test
    fun saysThatThePaperWasPrinted() {
        show(deliveryOutcome(paperPrinted = true))
        compose.onNodeWithTag("delivery_message").assertTextEquals("Ticket imprimé")
    }

    @Test
    fun theEmailMessageComesBeforeThePrintedOne() {
        show(deliveryOutcome(email = EmailStatus.SENT, paperPrinted = true, hint = "x"))
        compose.onNodeWithTag("delivery_message").assertTextEquals("Ticket envoyé à x")
    }

    @Test
    fun noMessageWhenNothingWasDelivered() {
        show(deliveryOutcome())
        compose.onNodeWithTag("delivery_outcome").assertExists()
        compose.onNodeWithTag("delivery_message").assertDoesNotExist()
    }

    @Test
    fun theMessageFollowsTheLanguage() {
        show(deliveryOutcome(paperPrinted = true), language = AppLanguage.EN)
        compose.onNodeWithTag("delivery_message").assertTextEquals("Ticket printed")
    }

    @Test
    fun offersThePrintOnlyWhenPaperIsNeededAndAllowed() {
        show(deliveryOutcome(needsPaper = false))
        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
    }

    @Test
    fun hidesThePrintFromAUserWhoCannotTakePayments() {
        show(deliveryOutcome(needsPaper = true), canPrint = false)
        compose.onNodeWithTag("delivery_outcome").assertExists()
        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
    }

    @Test
    fun printsTheOriginalOnceThenTheActionIsGone() {
        show(deliveryOutcome(needsPaper = true, fiscalEventId = 88L))
        compose.onNodeWithTag("delivery_print_missing").assertTextEquals("Imprimer le ticket")
        compose.onNodeWithTag("delivery_print_missing").performClick()
        compose.waitForIdle()
        assertEquals(listOf(88L), printed)
        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
    }

    @Test
    fun aSecondTapWhilePrintingDoesNotPrintAgain() {
        val gate = CompletableDeferred<Boolean>()
        show(deliveryOutcome(needsPaper = true), result = { gate.await() })
        compose.onNodeWithTag("delivery_print_missing").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("delivery_print_missing").performClick()
        compose.waitForIdle()
        assertEquals(1, printed.size)
        gate.complete(true)
        compose.waitForIdle()
        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
        assertEquals(1, printed.size)
    }

    @Test
    fun aRefusedPrintKeepsTheActionSoTheOperatorCanTryAgain() {
        val answers = mutableListOf(false, true)
        show(deliveryOutcome(needsPaper = true), result = { answers.removeAt(0) })
        compose.onNodeWithTag("delivery_print_missing").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("delivery_print_missing").assertIsDisplayed()
        compose.onNodeWithTag("delivery_print_missing").performClick()
        compose.waitForIdle()
        assertEquals(2, printed.size)
        compose.onNodeWithTag("delivery_print_missing").assertDoesNotExist()
    }

    @Test
    fun withoutAFiscalEventIdNothingIsPrinted() {
        show(deliveryOutcome(needsPaper = true), fiscalEventId = null)
        compose.onNodeWithTag("delivery_print_missing").performClick()
        compose.waitForIdle()
        assertEquals(emptyList<Long>(), printed)
        compose.onNodeWithTag("delivery_print_missing").assertIsDisplayed()
    }

    @Test
    fun kindOfOutcomeFollowsTheVueOrder() {
        assertNull(deliveryOutcomeKind(null))
        assertNull(deliveryOutcomeKind(deliveryOutcome()))
        assertEquals(DeliveryOutcomeKind.Sent, deliveryOutcomeKind(deliveryOutcome(email = EmailStatus.SENT, paperPrinted = true)))
        assertEquals(DeliveryOutcomeKind.Queued, deliveryOutcomeKind(deliveryOutcome(email = EmailStatus.QUEUED, paperPrinted = true)))
        assertEquals(DeliveryOutcomeKind.Printed, deliveryOutcomeKind(deliveryOutcome(paperPrinted = true)))
    }
}
