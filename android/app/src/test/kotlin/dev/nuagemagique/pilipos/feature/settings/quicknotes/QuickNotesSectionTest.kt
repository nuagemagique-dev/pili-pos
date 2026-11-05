package dev.nuagemagique.pilipos.feature.settings.quicknotes

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.unit.dp
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

/** The quick notes panel (`QuickNotesSettings.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class QuickNotesSectionTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        QuickNotesCallbacks(
            onUp = { calls += "up:$it" },
            onDown = { calls += "down:$it" },
            onRemove = { calls += "remove:$it" },
            onAdd = { calls += "add" },
            onAddConfirm = { calls += "confirm:$it" },
            onAddCancel = { calls += "cancel" },
            onSave = { calls += "save" },
        )

    private val three = listOf("Sans wasabi", "Bien cuit", "Épicé")

    private fun loaded(
        notes: List<String> = three,
        stored: List<String> = notes,
        adding: Boolean = false,
        saved: Boolean = false,
        error: String? = null,
    ) = QuickNotesUiState(manager = true, stored = stored, notes = notes, adding = adding, saved = saved, error = error)

    private fun show(state: QuickNotesUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Column(Modifier.verticalScroll(rememberScrollState())) { QuickNotesSection(state, callbacks) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theNotesAreListedInOrderAndSaveWaitsForAChange() {
        show(loaded())

        compose.onNodeWithTag("quick_note_0_text").assertTextEquals("Sans wasabi")
        compose.onNodeWithTag("quick_note_2_text").assertTextEquals("Épicé")
        compose.onNodeWithTag("quick_notes_save").assertIsNotEnabled()
        compose.onNodeWithTag("quick_notes_add").assertIsEnabled()
    }

    @Test
    fun theControlsCallBackWithTheIndexAndTheEndsCannotMovePastThemselves() {
        show(loaded())

        compose.onNodeWithTag("quick_note_0_up").assertIsNotEnabled()
        compose.onNodeWithTag("quick_note_2_down").assertIsNotEnabled()
        compose.onNodeWithTag("quick_note_1_up").performScrollTo().performClick()
        compose.onNodeWithTag("quick_note_1_down").performClick()
        compose.onNodeWithTag("quick_note_1_remove").performClick()

        assertEquals(listOf("up:1", "down:1", "remove:1"), calls)
    }

    @Test
    fun theControlsAreTouchTargetsOfAtLeast48Dp() {
        show(loaded())

        for (tag in listOf("up", "down", "remove")) {
            compose.onNodeWithTag("quick_note_1_$tag").assertHeightIsAtLeast(48.dp)
        }
        compose.onNodeWithTag("quick_notes_add").performScrollTo().assertHeightIsAtLeast(48.dp)
    }

    @Test
    fun aChangedDraftCanBeSavedAndAddAsksTheText() {
        show(loaded(notes = three.dropLast(1), stored = three))

        compose.onNodeWithTag("quick_notes_save").performScrollTo().assertIsEnabled().performClick()
        compose.onNodeWithTag("quick_notes_add").performClick()

        assertEquals(listOf("save", "add"), calls)
    }

    @Test
    fun anEmptyListSaysSo() {
        show(loaded(notes = emptyList(), stored = three))

        compose.onNodeWithTag("quick_notes_empty").assertIsDisplayed()
        compose.onNodeWithTag("quick_notes_save").performScrollTo().assertIsEnabled()
    }

    @Test
    fun aFullListDisablesAdd() {
        show(loaded(notes = List(24) { "Note $it" }))

        compose.onNodeWithTag("quick_notes_full").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("quick_notes_add").assertIsNotEnabled()
    }

    @Test
    fun theSavedMessageAndTheErrorShow() {
        show(loaded(saved = true, error = "Trop de notes"))

        compose.onNodeWithTag("quick_notes_saved").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("quick_notes_error").assertTextContains("Trop de notes")
    }

    @Test
    fun theAddDialogValidatesOnlyWhatTheRulesAccept() {
        show(loaded(adding = true))

        compose.onNodeWithTag("quick_note_add_confirm").assertIsNotEnabled()
        compose.onNodeWithTag("quick_note_add_text").performTextInput("bien cuit")
        compose.onNodeWithTag("quick_note_add_confirm").assertIsNotEnabled()
    }

    @Test
    fun aNewNoteIsConfirmedAndCancelCallsBack() {
        show(loaded(adding = true))

        compose.onNodeWithTag("quick_note_add_text").performTextInput("Sans sauce")
        compose.onNodeWithTag("quick_note_add_confirm").assertIsEnabled().performClick()
        compose.onNodeWithTag("quick_note_add_cancel").performClick()

        assertEquals(listOf("confirm:Sans sauce", "cancel"), calls)
    }
}
