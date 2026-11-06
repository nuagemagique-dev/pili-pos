package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The longest text a keyboard field takes by default (`maxLength = 80` of `TextKeyboard.vue`). */
const val TEXT_KEYBOARD_MAX_LENGTH: Int = 80

private const val DIGITS = "1234567890"
private val Rows: List<String> = listOf("AZERTYUIOP", "QSDFGHJKLM", "WXCVBN")
private val Accents: List<Char> = listOf('É', 'È', 'Ê', 'À', 'Ç', 'Ù')
private val Punctuation: List<Char> = listOf('-', '\'', '.', ',', '/', '&')

private val KeyWidth = 72.dp
private val KeyHeight = 64.dp
private val ClearWidth = 152.dp
private val EnterWidth = 152.dp
private val SpaceWidth = 440.dp
private val KeyboardMaxWidth = 1040.dp

/**
 * `TextKeyboard.vue`: the Belgian AZERTY keyboard for names, addresses and VAT numbers. Upper case by default (invoices are
 * printed in capitals), the accents of French names and the punctuation of addresses; [caseToggle] adds the "Aa" key
 * for lower case. The component owns no text: [onValueChange] gets the whole new value. [onSubmit] adds the enter key
 * (the caller says what it does); null leaves it out.
 * Test tags, with [tag] as prefix: `<tag>_key_<UPPER CHAR>`, `<tag>_backspace`, `<tag>_clear`, `<tag>_case`
 * (selected while lower case is on), `<tag>_space`, `<tag>_enter`.
 */
@Composable
fun TextKeyboard(
    tag: String,
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    maxLength: Int = TEXT_KEYBOARD_MAX_LENGTH,
    caseToggle: Boolean = false,
    enterLabel: String? = null,
    onSubmit: () -> Unit = {},
) {
    var lower by rememberSaveable(tag) { mutableStateOf(false) }
    val colors = PiliTheme.colors
    val gap = PiliTheme.dimens.spaceSm

    fun shown(ch: Char): String = if (lower) ch.lowercase() else ch.toString()

    fun press(ch: Char) {
        if (value.length < maxLength) onValueChange(value + shown(ch))
    }

    @Composable
    fun Key(
        ch: Char,
        color: androidx.compose.ui.graphics.Color,
    ) {
        TextKeypadKey(
            modifier = Modifier.size(KeyWidth, KeyHeight).testTag("${tag}_key_$ch"),
            onClick = { press(ch) },
        ) { KeyLabel(shown(ch), color) }
    }
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(gap),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(gap)) { DIGITS.forEach { Key(it, colors.textPrimary) } }
        Rows.forEachIndexed { i, row ->
            Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
                row.forEach { Key(it, colors.textPrimary) }
                if (i == Rows.lastIndex) {
                    TextKeypadKey(
                        modifier = Modifier.size(KeyWidth, KeyHeight).testTag("${tag}_backspace"),
                        onClick = { onValueChange(value.dropLast(1)) },
                        description = vueText("keyboard_backspace"),
                    ) { BackspaceGlyph() }
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
            Accents.forEach { Key(it, colors.textSecondary) }
            Punctuation.forEach { Key(it, colors.textSecondary) }
        }
        KeyboardBottomRow(
            KeyboardOptions(tag, maxLength, caseToggle, enterLabel),
            value,
            onValueChange,
            lower,
            { lower = !lower },
            onSubmit,
        )
    }
}

/** What the bottom row of the keyboard needs to know. */
private data class KeyboardOptions(
    val tag: String,
    val maxLength: Int,
    val caseToggle: Boolean,
    val enterLabel: String?,
)

/** Clear, the case key, the space bar and the optional enter key. */
@Composable
private fun KeyboardBottomRow(
    options: KeyboardOptions,
    value: String,
    onValueChange: (String) -> Unit,
    lower: Boolean,
    onLower: () -> Unit,
    onSubmit: () -> Unit,
) {
    val colors = PiliTheme.colors
    val tag = options.tag
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        TextKeypadKey(
            modifier = Modifier.size(ClearWidth, KeyHeight).testTag("${tag}_clear"),
            onClick = { onValueChange("") },
        ) { KeyLabel(vueText("clear"), colors.accentWarning) }
        if (options.caseToggle) {
            TextKeypadKey(
                modifier = Modifier.size(KeyWidth, KeyHeight).semantics { selected = lower }.testTag("${tag}_case"),
                onClick = onLower,
                description = vueText("menu_ed_case"),
                on = lower,
            ) { KeyLabel("Aa", if (lower) colors.selectedFg else colors.textSecondary) }
        }
        TextKeypadKey(
            modifier = Modifier.size(SpaceWidth, KeyHeight).testTag("${tag}_space"),
            onClick = { if (value.length < options.maxLength) onValueChange("$value ") },
            description = vueText("keyboard_space"),
        ) { KeyLabel("\u2423", colors.textSecondary) }
        if (options.enterLabel != null) {
            TextKeypadKey(
                modifier = Modifier.size(EnterWidth, KeyHeight).testTag("${tag}_enter"),
                onClick = onSubmit,
            ) { KeyLabel(options.enterLabel, colors.textPrimary) }
        }
    }
}

/** What a keyboard dialog shows and asks; [tag] is the prefix of its test tags (see [TextKeyboardDialog]). */
@Immutable
data class TextKeyboardSpec(
    val tag: String,
    val title: String,
    val placeholder: String = "",
    val initial: String = "",
    val maxLength: Int = TEXT_KEYBOARD_MAX_LENGTH,
    val caseToggle: Boolean = false,
)

/**
 * The AZERTY [TextKeyboard] in a dialog, for one field (an invoice customer line, a menu name): the text typed so far
 * above the keys, Cancel and Validate under them. [onConfirm] gets the text. Test tags: `<tag>_dialog`, `<tag>_display`,
 * `<tag>_text`, `<tag>_placeholder`, `<tag>_cancel`, `<tag>_confirm`, plus the keys of [TextKeyboard].
 */
@Composable
fun TextKeyboardDialog(
    spec: TextKeyboardSpec,
    onConfirm: (String) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var buffer by rememberSaveable(spec.tag) { mutableStateOf(spec.initial.take(spec.maxLength)) }
    val tag = spec.tag
    Box(modifier = modifier.fillMaxSize().testTag("${tag}_dialog")) {
        PiliDialog(
            title = spec.title,
            onDismissRequest = onCancel,
            maxWidth = KeyboardMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("delivery_cancel"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("${tag}_cancel"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = { onConfirm(buffer) },
                    modifier = Modifier.testTag("${tag}_confirm"),
                )
            },
        ) {
            Column(
                modifier = Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                KeypadDisplay(tag, buffer, spec.placeholder, showsError = false)
                TextKeyboard(
                    tag = tag,
                    value = buffer,
                    onValueChange = { buffer = it },
                    maxLength = spec.maxLength,
                    caseToggle = spec.caseToggle,
                )
            }
        }
    }
}
