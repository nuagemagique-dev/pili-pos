package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The longest kitchen note, in characters (`NOTE_MAX` of `NoteDialog.vue`, `KITCHEN_NOTE_MAX` of the engine). */
internal const val NOTE_MAX: Int = 60

private val DialogMaxWidth = 720.dp
private val FieldHeight = 64.dp
private val Hairline = 1.dp

/**
 * The text a note field keeps after [typed] was entered: no control character (a pasted line break would be
 * refused by the engine as `note_unprintable`) and at most [NOTE_MAX] characters. Whether the printer can print
 * every character stays the engine's call (`note_unprintable`, `note_too_long`).
 */
internal fun noteText(typed: String): String = typed.filterNot { it.isISOControl() }.take(NOTE_MAX)

/**
 * `NoteDialog.vue`: the kitchen note of a cart line, typed with the system keyboard (the Vue screen has its own
 * keypad). A counter shows `n/60`, and the hint says when the note prints: now when the line already went to the
 * kitchen ([sent]), else with the order. An empty text clears the note. The dialog only collects the text and calls
 * [onConfirm]; the engine validates and prints.
 *
 * Test tags: `note_dialog`, `note_text`, `note_counter`, `note_hint`, `note_confirm`, `note_cancel`.
 */
@Composable
internal fun NoteDialog(
    initial: String,
    sent: Boolean,
    onConfirm: (String) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var text by rememberSaveable { mutableStateOf(noteText(initial)) }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Box(modifier = modifier.testTag("note_dialog")) {
        PiliDialog(
            title = vueText("note_title"),
            onDismissRequest = onCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("delivery_cancel"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("note_cancel"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = { onConfirm(text.trim()) },
                    modifier = Modifier.testTag("note_confirm"),
                )
            },
        ) {
            NoteField(text, { text = noteText(it) }, { onConfirm(text.trim()) }, focus)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(
                    text = vueText(if (sent) "note_hint_now" else "note_hint_with_order"),
                    style = t.body.copy(fontSize = t.sizeSm),
                    color = c.textSecondary,
                    modifier = Modifier.weight(1f).testTag("note_hint"),
                )
                Text(
                    text = "${text.length}/$NOTE_MAX",
                    style = t.figure.copy(fontSize = t.sizeSm),
                    color = c.textSecondary,
                    modifier = Modifier.testTag("note_counter"),
                )
            }
        }
    }
}

/** The one-line text field of the note, on the system keyboard; "Done" confirms. */
@Composable
private fun NoteField(
    value: String,
    onValueChange: (String) -> Unit,
    onDone: () -> Unit,
    focus: FocusRequester,
) {
    val c = PiliTheme.colors
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        singleLine = true,
        textStyle = PiliTheme.typography.body.copy(color = c.textPrimary, fontSize = PiliTheme.typography.sizeLg),
        cursorBrush = SolidColor(c.textPrimary),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
        modifier = Modifier.fillMaxWidth().focusRequester(focus).testTag("note_text"),
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = FieldHeight)
                    .border(BorderStroke(Hairline, c.border), RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .padding(horizontal = PiliTheme.dimens.spaceMd),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.isEmpty()) {
                    Text(
                        vueText("note_placeholder"),
                        style = PiliTheme.typography.body,
                        color = c.textSecondary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                inner()
            }
        },
    )
}
