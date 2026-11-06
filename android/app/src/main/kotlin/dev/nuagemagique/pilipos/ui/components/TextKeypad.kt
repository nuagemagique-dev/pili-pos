package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The default longest text of a keypad (the kitchen note limit, `maxLen = 60` of `TextKeypad.vue`). */
const val TEXT_KEYPAD_MAX_LENGTH: Int = 60

/** Symbol keys the fields add to the keypad (`utils/keypadSymbols.ts`). */
object KeypadSymbols {
    /** The accents of French names and the dash and apostrophe of compound ones. */
    val Name: List<String> = listOf("é", "è", "ê", "à", "ç", "-", "'")

    /** A code or an identifier (channel names, authorization codes). */
    val Code: List<String> = listOf("-", "_", ".", ":", "/")

    /** The table plan is ranges and letters: "1-12, 20-25", "A1". */
    val TablePlan: List<String> = listOf("-", ",")
}

private val LetterRows: List<String> = listOf("qwertyuiop", "asdfghjkl", "zxcvbnm")
private const val DIGITS = "1234567890"

private val KeypadMaxWidth = 820.dp
private val KeyWidth = 64.dp
private val KeyHeight = 56.dp
private val SpaceKeyWidth = 160.dp
private val DisplayMinHeight = 64.dp
private val ChipHeight = 48.dp
private val ChipBorder = 2.dp
private val InvalidBorder = 2.dp
private const val CHIP_DISABLED_ALPHA = 0.3f

/** [buffer] with [chunk] appended; unchanged when the result would pass [maxLen] characters (`press` of `TextKeypad.vue`). */
fun keypadAppend(
    buffer: String,
    chunk: String,
    maxLen: Int,
): String = if (buffer.length + chunk.length > maxLen) buffer else buffer + chunk

/**
 * What a text keypad shows and asks. Plain data, so a field can hand it to the [KeypadHost] and a test can build one.
 *
 * [tag] is the prefix of the test tags: `<tag>_dialog` (root), `<tag>_text` (what is typed), `<tag>_placeholder`,
 * `<tag>_hint`, `<tag>_counter`, `<tag>_error`, `<tag>_chip_<index>`, `<tag>_key_<char>`, `<tag>_case`, `<tag>_space`,
 * `<tag>_backspace`, `<tag>_cancel`, `<tag>_clear`, `<tag>_confirm`.
 * [chips] are quick notes: a tap puts `chipAppend(buffer, chip)` in the field, nothing when it returns null (the chip is
 * faded then). [valid] says whether the buffer may be confirmed (an empty text included unless it refuses it).
 */
@Immutable
data class TextKeypadSpec(
    val tag: String,
    val title: String,
    val placeholder: String = "",
    val initial: String = "",
    val maxLen: Int = TEXT_KEYPAD_MAX_LENGTH,
    val chips: List<String> = emptyList(),
    val chipAppend: (String, String) -> String? = { _, _ -> null },
    val symbols: List<String> = emptyList(),
    val space: Boolean = false,
    val caseToggle: Boolean = false,
    val hint: String = "",
    val showCounter: Boolean = false,
    val valid: (String) -> Boolean = { true },
    val invalidText: String = "",
)

/**
 * `TextKeypad.vue`: the on-screen keyboard for a short text (a note, a name, a setting). Lower case letters on three
 * rows, the digits, then the [TextKeypadSpec.symbols], an optional shift key, an optional space bar and the backspace.
 * Cancel, Clear and Validate sit under the keys; [onConfirm] gets the text, [onCancel] runs on Cancel and on a tap
 * outside the card. Drawn in the window like every dialog: put it last in a `Box` that fills the screen.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TextKeypad(
    spec: TextKeypadSpec,
    onConfirm: (String) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var buffer by rememberSaveable(spec.tag) { mutableStateOf(spec.initial.take(spec.maxLen)) }
    var upper by rememberSaveable(spec.tag) { mutableStateOf(false) }
    val valid = spec.valid(buffer)
    val tag = spec.tag
    Box(modifier = modifier.fillMaxSize().testTag("${tag}_dialog")) {
        PiliDialog(
            title = spec.title,
            onDismissRequest = onCancel,
            maxWidth = KeypadMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("delivery_cancel"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("${tag}_cancel"),
                )
                PiliButton(
                    text = vueText("clear"),
                    onClick = { buffer = "" },
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("${tag}_clear"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = { onConfirm(buffer) },
                    enabled = valid,
                    modifier = Modifier.testTag("${tag}_confirm"),
                )
            },
        ) {
            KeypadBody(spec, buffer, upper, valid, onBuffer = { buffer = it }, onUpper = { upper = !upper })
        }
    }
}

/** The display, the messages, the chips and the keys of a [TextKeypad]. */
@Composable
private fun KeypadBody(
    spec: TextKeypadSpec,
    buffer: String,
    upper: Boolean,
    valid: Boolean,
    onBuffer: (String) -> Unit,
    onUpper: () -> Unit,
) {
    val tag = spec.tag
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
    ) {
        KeypadDisplay(tag, buffer, spec.placeholder, showsError = buffer.isNotEmpty() && !valid)
        if (buffer.isNotEmpty() && !valid && spec.invalidText.isNotEmpty()) {
            Text(
                spec.invalidText,
                style = PiliTheme.typography.label,
                color = PiliTheme.colors.accentDanger,
                modifier = Modifier.testTag("${tag}_error"),
            )
        }
        if (spec.hint.isNotEmpty() || spec.showCounter) KeypadInfo(spec, buffer)
        if (spec.chips.isNotEmpty()) KeypadChips(spec, buffer, onBuffer)
        KeypadKeyRows(
            spec = spec,
            upper = upper,
            onUpper = onUpper,
            onAppend = { onBuffer(keypadAppend(buffer, it, spec.maxLen)) },
            onBackspace = { onBuffer(buffer.dropLast(1)) },
        )
    }
}

/** What is typed (or the placeholder), with a red border while it is not valid. */
@Composable
internal fun KeypadDisplay(
    tag: String,
    buffer: String,
    placeholder: String,
    showsError: Boolean,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val dimens = PiliTheme.dimens
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = DisplayMinHeight)
                .background(colors.bgPrimary, RoundedCornerShape(dimens.radiusMd))
                .border(
                    BorderStroke(InvalidBorder, if (showsError) colors.accentDanger else Color.Transparent),
                    RoundedCornerShape(dimens.radiusMd),
                ).padding(dimens.spaceMd)
                .testTag("${tag}_display"),
        contentAlignment = Alignment.Center,
    ) {
        if (buffer.isEmpty()) {
            Text(
                placeholder,
                style = typography.body,
                color = colors.textSecondary,
                modifier = Modifier.testTag("${tag}_placeholder"),
            )
        } else {
            Text(
                buffer,
                style = typography.body.copy(fontSize = typography.sizeXl, fontWeight = typography.weightBold),
                color = colors.textPrimary,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("${tag}_text"),
            )
        }
    }
}

@Composable
private fun KeypadInfo(
    spec: TextKeypadSpec,
    buffer: String,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(
            text = spec.hint,
            style = typography.body.copy(fontSize = typography.sizeSm),
            color = colors.textSecondary,
            modifier = Modifier.weight(1f).testTag("${spec.tag}_hint"),
        )
        if (spec.showCounter) {
            Text(
                text = "${buffer.length}/${spec.maxLen}",
                style = typography.figure.copy(fontSize = typography.sizeSm),
                color = colors.textSecondary,
                modifier = Modifier.testTag("${spec.tag}_counter"),
            )
        }
    }
}

/** The quick notes as chips: a tap puts [TextKeypadSpec.chipAppend] of the text in the field. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun KeypadChips(
    spec: TextKeypadSpec,
    buffer: String,
    onBuffer: (String) -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    FlowRow(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(dimens.spaceSm),
        verticalArrangement = Arrangement.spacedBy(dimens.spaceSm),
    ) {
        spec.chips.forEachIndexed { index, chip ->
            val next = spec.chipAppend(buffer, chip)
            Box(
                modifier =
                    Modifier
                        .defaultMinSize(minHeight = ChipHeight)
                        .alpha(if (next == null) CHIP_DISABLED_ALPHA else 1f)
                        .background(colors.bgSecondary, RoundedCornerShape(ChipHeight / 2))
                        .border(BorderStroke(ChipBorder, colors.textSecondary), RoundedCornerShape(ChipHeight / 2))
                        .clickable(enabled = next != null, role = Role.Button) { next?.let(onBuffer) }
                        .padding(horizontal = dimens.spaceMd)
                        .testTag("${spec.tag}_chip_$index"),
                contentAlignment = Alignment.Center,
            ) {
                val typography = PiliTheme.typography
                Text(
                    chip,
                    style = typography.body.copy(fontSize = typography.sizeSm, fontWeight = typography.weightBold),
                    color = colors.textPrimary,
                )
            }
        }
    }
}

/** The letter rows, the digit row, then the symbols with the shift key, the space bar and the backspace. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun KeypadKeyRows(
    spec: TextKeypadSpec,
    upper: Boolean,
    onUpper: () -> Unit,
    onAppend: (String) -> Unit,
    onBackspace: () -> Unit,
) {
    val colors = PiliTheme.colors
    val gap = PiliTheme.dimens.spaceSm
    val tag = spec.tag

    fun shown(ch: Char): String = if (upper) ch.uppercase() else ch.toString()

    fun shownSymbol(symbol: String): String = if (upper) symbol.uppercase() else symbol
    Column(verticalArrangement = Arrangement.spacedBy(gap), horizontalAlignment = Alignment.CenterHorizontally) {
        LetterRows.forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
                row.forEach { ch ->
                    TextKeypadKey(
                        modifier = Modifier.size(KeyWidth, KeyHeight).testTag("${tag}_key_${shown(ch)}"),
                        onClick = { onAppend(shown(ch)) },
                    ) { KeyLabel(shown(ch), colors.textPrimary) }
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
            DIGITS.forEach { ch ->
                TextKeypadKey(
                    modifier = Modifier.size(KeyWidth, KeyHeight).testTag("${tag}_key_$ch"),
                    onClick = { onAppend(ch.toString()) },
                ) { KeyLabel(ch.toString(), colors.textPrimary) }
            }
        }
        // Many symbols wrap onto a second line, as the flex row of the Vue keypad does.
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(gap, Alignment.CenterHorizontally),
            verticalArrangement = Arrangement.spacedBy(gap),
        ) {
            spec.symbols.forEach { symbol ->
                TextKeypadKey(
                    modifier = Modifier.size(KeyWidth, KeyHeight).testTag("${tag}_key_${shownSymbol(symbol)}"),
                    onClick = { onAppend(shownSymbol(symbol)) },
                ) { KeyLabel(shownSymbol(symbol), colors.textSecondary) }
            }
            if (spec.caseToggle) {
                TextKeypadKey(
                    modifier = Modifier.size(KeyWidth, KeyHeight).semantics { selected = upper }.testTag("${tag}_case"),
                    onClick = onUpper,
                    description = vueText("menu_ed_case"),
                    on = upper,
                ) { KeyLabel("Aa", if (upper) colors.selectedFg else colors.textSecondary) }
            }
            if (spec.space) {
                TextKeypadKey(
                    modifier = Modifier.size(SpaceKeyWidth, KeyHeight).testTag("${tag}_space"),
                    onClick = { onAppend(" ") },
                    description = vueText("keypad_space"),
                ) { KeyLabel("\u2423", colors.textSecondary) }
            }
            TextKeypadKey(
                modifier = Modifier.size(KeyWidth, KeyHeight).testTag("${tag}_backspace"),
                onClick = onBackspace,
                description = vueText("delivery_edit"),
            ) { BackspaceGlyph() }
        }
    }
}
