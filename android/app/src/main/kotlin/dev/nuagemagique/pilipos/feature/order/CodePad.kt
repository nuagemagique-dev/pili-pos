package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.key.utf16CodePoint
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Width of the pad beside the product grid (`.code-pad` of `CodePad.vue`). */
internal val CodePadWidth = 360.dp

/** Product columns while the pad is open (`:columns` of `MenuGrid.vue`). */
internal const val GRID_COLUMNS_WITH_PAD = 3

internal val CodeFieldHeight = 72.dp
private val MessageHeight = 44.dp
private val KeyHeight = 68.dp
private val LetterHeight = 52.dp
private const val LETTERS_PER_ROW = 6
private val DigitRows = listOf(listOf("7", "8", "9"), listOf("4", "5", "6"), listOf("1", "2", "3"))

/** What the pad can ask for. */
internal data class CodePadActions(
    val onKey: (String) -> Unit = {},
    val onBackspace: () -> Unit = {},
    val onClear: () -> Unit = {},
    val onSubmit: () -> Unit = {},
)

/**
 * The pad of the order entry by menu code (`CodePad.vue`): the field (the code, the quantity as a pill, a clear
 * button), the line telling what OK will do, the digits with the quantity key and backspace, the letters the menu codes
 * use, and OK. The grid beside it shows the products whose code starts with the typed text. A physical keyboard types
 * into it while it has the focus (taken when it opens). Test tags: `code_pad`, `code_pad_display`, `code_pad_qty`,
 * `code_pad_message`, `code_key_<digit|letter>`, `code_key_times`, `code_key_clear`, `code_key_back`, `code_key_ok`.
 */
@Composable
internal fun CodePad(
    pad: CodePadState,
    actions: CodePadActions,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    val c = PiliTheme.colors
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { runCatching { focus.requestFocus() } }
    val tone = codeTone(pad.message)
    Column(
        modifier =
            modifier
                .width(CodePadWidth)
                .fillMaxHeight()
                .focusRequester(focus)
                .focusable()
                .onKeyEvent { hardwareKey(it, actions) }
                .verticalScroll(rememberScrollState())
                .padding(start = d.spaceMd, top = d.spaceMd, bottom = d.spaceMd)
                .testTag("code_pad"),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        CodeField(pad.text, tone, actions.onClear)
        CodeMessageLine(pad.message, tone)
        DigitRows.forEach { row -> KeyRow { row.forEach { k -> CharKey(k, actions) } } }
        KeyRow {
            PadKey("×", "code_key_times", sub = vueText("code_pad_quantity")) { actions.onKey(TIMES) }
            CharKey("0", actions)
            PadKey("⌫", "code_key_back", c.textSecondary, description = vueText("code_pad_backspace")) { actions.onBackspace() }
        }
        pad.letters.chunked(LETTERS_PER_ROW).forEach { row ->
            KeyRow {
                row.forEach { l -> PadKey(l, "code_key_$l", height = LetterHeight, small = true) { actions.onKey(l) } }
                repeat(LETTERS_PER_ROW - row.size) { Box(Modifier.weight(1f)) }
            }
        }
        KeyRow { PadKey("OK", "code_key_ok", c.textOnAccent, fill = c.accentSuccess, height = CodeFieldHeight) { actions.onSubmit() } }
    }
}

/** A key of a physical keyboard: Enter, Backspace, Escape, a digit, a letter or `*`; true when the pad took it. */
private fun hardwareKey(
    event: KeyEvent,
    actions: CodePadActions,
): Boolean {
    val action: (() -> Unit)? =
        if (event.type != KeyEventType.KeyDown) {
            null
        } else {
            when (event.key) {
                Key.Enter, Key.NumPadEnter -> actions.onSubmit
                Key.Backspace -> actions.onBackspace
                Key.Escape -> actions.onClear
                else -> padChar(event.utf16CodePoint)?.let { key -> { actions.onKey(key) } }
            }
        }
    action?.invoke()
    return action != null
}

private const val ASCII_END = 128

/** The pad key a typed character stands for: an ASCII letter or digit, or `*`; null for anything else. */
private fun padChar(codePoint: Int): String? {
    val ch = codePoint.toChar()
    return ch.toString().takeIf { codePoint in 0 until ASCII_END && (ch.isLetterOrDigit() || it == TIMES) }
}

/** The text of the line under the field. */
@Composable
internal fun codeMessageText(message: CodeMessage): String {
    fun line(
        quantity: Int,
        item: uniffi.pos_ffi.MenuItem,
    ) = "$quantity × ${item.name} · ${formatCents(item.priceCents * quantity)}"
    return refusalText(message)
        ?: when (message) {
            is CodeMessage.WaitingProduct -> vueText("code_pad_multiplier", "n" to message.quantity)
            is CodeMessage.Results -> vueText("code_pad_results", "n" to message.count)
            is CodeMessage.Match -> line(message.quantity, message.item)
            is CodeMessage.Added -> "✓ " + line(message.quantity, message.item)
            else -> vueText("code_pad_hint")
        }
}

/** The text of a refusal of the pad; null for any other message. */
@Composable
private fun refusalText(message: CodeMessage): String? =
    when (message) {
        CodeMessage.Invalid -> vueText("code_pad_invalid")
        is CodeMessage.Unknown -> vueText("code_pad_unknown", "code" to message.code)
        is CodeMessage.Ambiguous -> vueText("code_pad_ambiguous", "code" to message.code)
        CodeMessage.NoMultiplier -> vueText("code_pad_no_multiplier")
        CodeMessage.SoldOut -> vueText("err_product_sold_out")
        CodeMessage.NoResult -> vueText("code_pad_no_result")
        else -> null
    }

@Composable
private fun CodeMessageLine(
    message: CodeMessage,
    tone: CodeTone,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val color =
        when (tone) {
            CodeTone.Hint -> c.textSecondary
            CodeTone.Match -> c.textPrimary
            CodeTone.Error -> c.accentDanger
            CodeTone.Done -> c.accentSuccess
        }
    Text(
        text = codeMessageText(message),
        style = t.body.copy(fontSize = t.sizeMd, fontWeight = if (tone == CodeTone.Hint) t.weightNormal else t.weightBold),
        color = color,
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = MessageHeight)
                .semantics { liveRegion = LiveRegionMode.Polite }
                .testTag("code_pad_message"),
    )
}

@Composable
private fun KeyRow(content: @Composable RowScope.() -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs), content = content)
}

@Composable
private fun RowScope.CharKey(
    key: String,
    actions: CodePadActions,
) = PadKey(key, "code_key_$key") { actions.onKey(key) }

@Suppress("LongParameterList")
@Composable
private fun RowScope.PadKey(
    label: String,
    tag: String,
    color: Color = PiliTheme.colors.textPrimary,
    fill: Color? = null,
    weight: Float = 1f,
    height: androidx.compose.ui.unit.Dp = KeyHeight,
    small: Boolean = false,
    description: String? = null,
    sub: String? = null,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    Box(
        modifier =
            Modifier
                .weight(weight)
                .height(height)
                .tapScale(source)
                .background(fill ?: if (pressed) c.bgPressed else c.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .then(if (description != null) Modifier.semantics { contentDescription = description } else Modifier)
                .testTag(tag),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text = label,
                style = t.body.copy(fontSize = if (small) t.sizeLg else t.sizeXl, fontWeight = FontWeight.Bold),
                color = color,
                textAlign = TextAlign.Center,
            )
            sub?.let { Text(it, style = t.body.copy(fontSize = t.sizeXs, fontWeight = FontWeight.Bold), color = c.textSecondary) }
        }
    }
}
