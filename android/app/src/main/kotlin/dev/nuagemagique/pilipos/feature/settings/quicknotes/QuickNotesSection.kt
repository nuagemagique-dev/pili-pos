package dev.nuagemagique.pilipos.feature.settings.quicknotes

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.feature.order.QUICK_NOTES_MAX
import dev.nuagemagique.pilipos.feature.order.addQuickNote
import dev.nuagemagique.pilipos.feature.order.noteText
import dev.nuagemagique.pilipos.feature.settings.ActionButton
import dev.nuagemagique.pilipos.feature.settings.Panel
import dev.nuagemagique.pilipos.feature.settings.PanelText
import dev.nuagemagique.pilipos.feature.settings.PanelTitle
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val ControlSize = 48.dp
private val RowHeight = 56.dp
private val FieldHeight = 64.dp
private val DialogMaxWidth = 560.dp
private const val DISABLED_ALPHA = 0.3f

/**
 * The quick notes panel of the settings screen (`QuickNotesSettings.vue`, "Notes rapides"): the chips of the kitchen
 * note dialog, each with up, down and remove, an add button that asks the text, and a save button. Local data: nothing
 * here reaches the signed data. Test tags: `quick_notes_section`, `quick_notes_error`, `quick_notes_empty`,
 * `quick_note_<i>` (a row) with `quick_note_<i>_text`, `_up`, `_down`, `_remove`, `quick_notes_full`,
 * `quick_notes_add`, `quick_notes_save`, `quick_notes_saved`, and in the add dialog `quick_note_add_text`,
 * `quick_note_add_confirm`, `quick_note_add_cancel`.
 */
@Composable
internal fun QuickNotesSection(
    state: QuickNotesUiState,
    callbacks: QuickNotesCallbacks,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Panel(modifier = modifier.testTag("quick_notes_section")) {
        PanelTitle(vueText("quick_notes_title"), Modifier)
        state.error?.let {
            Text(it, style = PiliTheme.typography.body, color = colors.accentDanger, modifier = Modifier.testTag("quick_notes_error"))
        }
        PanelText(vueText("quick_notes_note"))
        if (state.stored != null) {
            if (state.notes.isEmpty()) {
                Text(
                    vueText("quick_notes_empty"),
                    style = PiliTheme.typography.body,
                    color = colors.textSecondary,
                    modifier = Modifier.testTag("quick_notes_empty"),
                )
            }
            state.notes.forEachIndexed { index, note -> NoteRow(index, note, state.notes.size, callbacks) }
            val full = state.notes.size >= QUICK_NOTES_MAX
            if (full) {
                Text(
                    vueText("quick_notes_full", "max" to QUICK_NOTES_MAX.toString()),
                    style = PiliTheme.typography.body,
                    color = colors.textSecondary,
                    modifier = Modifier.testTag("quick_notes_full"),
                )
            }
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                ActionButton(
                    text = vueText("quick_notes_add"),
                    onClick = callbacks.onAdd,
                    enabled = !full,
                    modifier = Modifier.testTag("quick_notes_add"),
                )
                ActionButton(
                    text = vueText("quick_notes_save"),
                    onClick = callbacks.onSave,
                    enabled = !state.busy && state.dirty,
                    variant = PiliButtonVariant.Primary,
                    modifier = Modifier.testTag("quick_notes_save"),
                )
                if (state.saved && !state.dirty) {
                    Text(
                        text = vueText("quick_notes_saved"),
                        style = PiliTheme.typography.body,
                        color = colors.accentSuccess,
                        modifier = Modifier.testTag("quick_notes_saved"),
                    )
                }
            }
        }
        if (state.adding) AddNoteDialog(state.notes, callbacks)
    }
}

/** A chip of the list with its three controls. */
@Composable
private fun NoteRow(
    index: Int,
    note: String,
    count: Int,
    callbacks: QuickNotesCallbacks,
) {
    val c = PiliTheme.colors
    Row(
        Modifier.fillMaxWidth().defaultMinSize(minHeight = RowHeight).testTag("quick_note_$index"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            note,
            style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeLg, fontWeight = PiliTheme.typography.weightBold),
            color = c.textPrimary,
            modifier = Modifier.weight(1f).testTag("quick_note_${index}_text"),
        )
        IconControl("▲", vueText("quick_notes_up"), index > 0, c.textPrimary, "quick_note_${index}_up") { callbacks.onUp(index) }
        IconControl(
            "▼",
            vueText("quick_notes_down"),
            index < count - 1,
            c.textPrimary,
            "quick_note_${index}_down",
        ) { callbacks.onDown(index) }
        IconControl("✕", vueText("quick_notes_remove"), true, c.accentDanger, "quick_note_${index}_remove") { callbacks.onRemove(index) }
    }
}

/** A 48 dp square button with a glyph; [enabled] false draws it faded and not clickable. */
@Composable
private fun IconControl(
    glyph: String,
    description: String,
    enabled: Boolean,
    color: androidx.compose.ui.graphics.Color,
    tag: String,
    onClick: () -> Unit,
) {
    Box(
        modifier =
            Modifier
                .defaultMinSize(minWidth = ControlSize, minHeight = ControlSize)
                .alpha(if (enabled) 1f else DISABLED_ALPHA)
                .let { m ->
                    m.clickable(enabled = enabled, role = Role.Button, onClick = onClick)
                }.semantics { contentDescription = description }
                .testTag(tag),
        contentAlignment = Alignment.Center,
    ) {
        Text(glyph, style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeMd), color = color)
    }
}

/** "Nouvelle note rapide": the text of the chip; "Valider" is disabled until the rules accept it. */
@Composable
private fun AddNoteDialog(
    notes: List<String>,
    callbacks: QuickNotesCallbacks,
) {
    var text by rememberSaveable { mutableStateOf("") }
    val c = PiliTheme.colors
    val valid = addQuickNote(notes, text) != null
    PiliDialog(
        title = vueText("quick_notes_add_title"),
        onDismissRequest = callbacks.onAddCancel,
        maxWidth = DialogMaxWidth,
        actions = {
            PiliButton(
                text = vueText("delivery_cancel"),
                onClick = callbacks.onAddCancel,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("quick_note_add_cancel"),
            )
            PiliButton(
                text = vueText("validate"),
                onClick = { callbacks.onAddConfirm(text) },
                enabled = valid,
                modifier = Modifier.testTag("quick_note_add_confirm"),
            )
        },
    ) {
        BasicTextField(
            value = text,
            onValueChange = { text = noteText(it) },
            singleLine = true,
            textStyle = PiliTheme.typography.body.copy(color = c.textPrimary, fontSize = PiliTheme.typography.sizeLg),
            cursorBrush = SolidColor(c.textPrimary),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { if (valid) callbacks.onAddConfirm(text) }),
            modifier = Modifier.fillMaxWidth().testTag("quick_note_add_text"),
            decorationBox = { inner ->
                Box(
                    Modifier
                        .fillMaxWidth()
                        .defaultMinSize(minHeight = FieldHeight)
                        .border(BorderStroke(1.dp, c.border), RoundedCornerShape(PiliTheme.dimens.radiusMd))
                        .padding(horizontal = PiliTheme.dimens.spaceMd),
                    contentAlignment = Alignment.CenterStart,
                ) { inner() }
            },
        )
    }
}
