package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextClearance
import androidx.compose.ui.test.performTextInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** What the note field keeps of the typed text, as plain rules. */
class NoteTextTest {
    @Test
    fun aNoteKeepsSixtyCharactersAtMost() {
        assertEquals(60, noteText("x".repeat(80)).length)
        assertEquals("sans wasabi", noteText("sans wasabi"))
    }

    @Test
    fun controlCharactersAreDropped() {
        assertEquals("sanswasabi", noteText("sans\nwasabi"))
        assertEquals("ab", noteText("a\tb"))
    }

    @Test
    fun accentsStay() {
        assertEquals("crème brûlée", noteText("crème brûlée"))
    }
}

/** The kitchen note dialog (`NoteDialog.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class NoteDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val saved = mutableListOf<String>()
    private var cancelled = 0

    private fun show(
        initial: String = "",
        sent: Boolean = false,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { NoteDialog(initial, sent, onConfirm = { saved += it }, onCancel = { cancelled++ }) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun itOpensEmptyWithACounterAndSaysTheNotePrintsWithTheOrder() {
        show()

        compose.onNodeWithTag("note_dialog").assertIsDisplayed()
        compose.onNodeWithTag("note_counter").assertTextEquals("0/60")
        compose.onNodeWithTag("note_hint").assertTextEquals("Imprimée en cuisine avec la commande")
    }

    @Test
    fun aSentLineSaysTheNotePrintsNow() {
        show(sent = true)

        compose.onNodeWithTag("note_hint").assertTextEquals("Imprimée en cuisine maintenant")
    }

    @Test
    fun theTypedTextIsCountedAndSent() {
        show()

        compose.onNodeWithTag("note_text").performTextInput("sans wasabi")
        compose.onNodeWithTag("note_counter").assertTextEquals("11/60")
        compose.onNodeWithTag("note_confirm").performClick()

        assertEquals(listOf("sans wasabi"), saved)
    }

    @Test
    fun theTextIsTrimmedWhenSent() {
        show()

        compose.onNodeWithTag("note_text").performTextInput("  sans wasabi  ")
        compose.onNodeWithTag("note_confirm").performClick()

        assertEquals(listOf("sans wasabi"), saved)
    }

    @Test
    fun theFieldStopsAtSixtyCharacters() {
        show()

        compose.onNodeWithTag("note_text").performTextInput("a".repeat(75))

        compose.onNodeWithTag("note_counter").assertTextEquals("60/60")
    }

    @Test
    fun aNotedLineOpensWithItsNoteAndAnEmptyTextClearsIt() {
        show(initial = "sans wasabi", sent = true)

        compose.onNodeWithTag("note_counter").assertTextEquals("11/60")
        compose.onNodeWithTag("note_text").performTextClearance()
        compose.onNodeWithTag("note_confirm").performClick()

        assertEquals(listOf(""), saved)
    }

    @Test
    fun backCancelsWithoutSavingAnything() {
        show()

        compose.onNodeWithTag("note_text").performTextInput("abc")
        compose.onNodeWithTag("note_cancel").performClick()

        assertEquals(1, cancelled)
        assertTrue(saved.isEmpty())
    }
}

/** The cart line with its "Note" button and the note under the name. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class KitchenNoteCartTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<Long>()
    private val actions = OrderCartActions(onNote = { seen += it })

    private fun show(state: OrderCartState) {
        compose.setContent { PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { Box { OrderCartColumn(state, actions) } } } }
        compose.waitForIdle()
    }

    private val plain = CartRow(cartLine(1, 10, "Maki saumon"), LineAmount.Total(650), canNote = true)
    private val noted = CartRow(cartLine(2, 11, "Nigiri thon", kitchenNote = "sans wasabi"), LineAmount.Total(650), canNote = true)

    @Test
    fun theNoteButtonIsOnTheLineAndOpensTheDialog() {
        show(OrderCartState(order = order(table = 4, totalCents = 650), rows = listOf(plain)))

        compose.onNodeWithTag("cart_line_1_note").assertTextEquals("Note")
        compose.onNodeWithTag("cart_line_1_note").performClick()

        assertEquals(listOf(1L), seen)
    }

    @Test
    fun theNoteShowsUnderTheName() {
        show(OrderCartState(order = order(table = 4, totalCents = 1_300), rows = listOf(plain, noted)))

        compose.onNodeWithTag("cart_line_2_kitchen_note").assertTextEquals("sans wasabi")
        compose.onNodeWithTag("cart_line_1_kitchen_note").assertDoesNotExist()
    }

    @Test
    fun aLineThatMayNotBeNotedHasNoButton() {
        show(OrderCartState(order = order(table = 4, totalCents = 650), rows = listOf(plain.copy(canNote = false))))

        compose.onNodeWithTag("cart_line_1_note").assertDoesNotExist()
    }
}
