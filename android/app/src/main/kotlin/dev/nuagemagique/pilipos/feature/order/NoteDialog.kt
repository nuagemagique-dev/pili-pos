package dev.nuagemagique.pilipos.feature.order

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import dev.nuagemagique.pilipos.ui.components.TextKeypad
import dev.nuagemagique.pilipos.ui.components.TextKeypadSpec
import dev.nuagemagique.pilipos.ui.text.vueText

/** The longest kitchen note, in characters (`NOTE_MAX` of `NoteDialog.vue`, `KITCHEN_NOTE_MAX` of the engine). */
internal const val NOTE_MAX: Int = 60

/** Accents of French notes and the usual punctuation that the letter rows lack; every one prints on the slip printer (CP858). */
internal val NoteSymbols: List<String> = listOf("é", "è", "à", "ç", "ê", ",", ".", "-", "'", "/", "(", ")", "+", "%")

/**
 * The text a note field keeps after [typed] was entered: no control character (a pasted line break would be
 * refused by the engine as `note_unprintable`) and at most [NOTE_MAX] characters. Whether the printer can print
 * every character stays the engine's call (`note_unprintable`, `note_too_long`).
 */
internal fun noteText(typed: String): String = typed.filterNot { it.isISOControl() }.take(NOTE_MAX)

/**
 * `NoteDialog.vue`: the kitchen note of a cart line, typed on the in-app text keypad (C13 / R6: the till never raises the
 * system keyboard). A counter shows `n/60`, and the hint says when the note prints: now when the line already went to the
 * kitchen ([sent]), else with the order. An empty text clears the note. The dialog only collects the text and calls
 * [onConfirm]; the engine validates and prints.
 *
 * Above the keys the quick notes of the installation are chips (C11, `keypad-chip` of `TextKeypad.vue`): a tap appends
 * the chip to the note, comma separated; a chip already in the note, or one that would pass 60 characters, is faded and
 * does nothing.
 *
 * Test tags: `note_dialog`, `note_text` (the typed text), `note_placeholder`, `note_counter`, `note_hint`,
 * `note_chip_<index>`, `note_key_<char>`, `note_space`, `note_backspace`, `note_clear`, `note_confirm`, `note_cancel`.
 */
@Composable
internal fun NoteDialog(
    initial: String,
    sent: Boolean,
    chips: List<String>,
    onConfirm: (String) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    TextKeypad(
        spec =
            TextKeypadSpec(
                tag = "note",
                title = vueText("note_title"),
                placeholder = vueText("note_placeholder"),
                initial = noteText(initial),
                maxLen = NOTE_MAX,
                chips = chips,
                chipAppend = { note, chip -> appendQuickNote(note, chip) },
                symbols = NoteSymbols,
                space = true,
                caseToggle = true,
                hint = vueText(if (sent) "note_hint_now" else "note_hint_with_order"),
                showCounter = true,
            ),
        onConfirm = { onConfirm(it.trim()) },
        onCancel = onCancel,
        modifier = modifier,
    )
}
