package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val FieldHeight = 56.dp

/** Which keypad a [KeypadField] opens. */
sealed interface KeypadKind {
    /** The lower case [TextKeypad], with the given extra keys. */
    data class Text(
        val maxLen: Int = TEXT_KEYPAD_MAX_LENGTH,
        val symbols: List<String> = emptyList(),
        val space: Boolean = false,
        val caseToggle: Boolean = false,
    ) : KeypadKind

    /** The AZERTY [TextKeyboard] (names and addresses, upper case by default). */
    data class Keyboard(
        val maxLength: Int = TEXT_KEYBOARD_MAX_LENGTH,
        val caseToggle: Boolean = false,
    ) : KeypadKind

    /** Whole numbers on the [NumPad]. */
    data class Number(
        val maxDigits: Int = 6,
        val exactDigits: Int? = null,
        val min: Long? = null,
        val max: Long? = null,
    ) : KeypadKind
}

/**
 * A field that never raises the system keyboard (UX audit C13 / R6, `KeypadField.vue`): it shows its [value] (or the
 * [placeholder]) on a hairline outline and a tap asks the [KeypadHost] for the keypad of [kind]. The value changes once,
 * when the keypad is validated: [onValueChange] gets the text, then [onDone] runs. [title] heads the keypad.
 * The caller's [modifier] carries its test tag; the keypad's own tags start with `keypad_` (text), `keyboard_` (AZERTY)
 * or `field_number_` (numbers).
 */
@Composable
fun KeypadField(
    value: String,
    onValueChange: (String) -> Unit,
    title: String,
    placeholder: String,
    modifier: Modifier = Modifier,
    kind: KeypadKind = KeypadKind.Text(),
    mono: Boolean = false,
    enabled: Boolean = true,
    onDone: (() -> Unit)? = null,
) {
    val colors = PiliTheme.colors
    val host = LocalKeypadHost.current
    val style =
        PiliTheme.typography.body.copy(
            color = colors.textPrimary,
            fontFamily = if (mono) PiliTheme.typography.familyFigures else FontFamily.Default,
        )
    val confirm: (String) -> Unit = {
        if (it != value) onValueChange(it)
        onDone?.invoke()
    }
    Box(
        modifier =
            modifier
                .fillMaxWidth()
                .height(FieldHeight)
                .border(1.dp, colors.border, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(enabled = enabled, role = Role.Button) { host.open(request(kind, title, placeholder, value, confirm)) }
                .padding(horizontal = PiliTheme.dimens.spaceMd)
                .semantics(mergeDescendants = true) { },
        contentAlignment = Alignment.CenterStart,
    ) {
        if (value.isEmpty()) {
            Text(
                placeholder,
                style = PiliTheme.typography.body,
                color = colors.textSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        } else {
            Text(value, style = style, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

/** The request to the host for a field of [kind]. */
private fun request(
    kind: KeypadKind,
    title: String,
    placeholder: String,
    value: String,
    confirm: (String) -> Unit,
): KeypadRequest =
    when (kind) {
        is KeypadKind.Text ->
            KeypadRequest.Text(
                TextKeypadSpec(
                    tag = "keypad",
                    title = title,
                    placeholder = placeholder,
                    initial = value,
                    maxLen = kind.maxLen,
                    symbols = kind.symbols,
                    space = kind.space,
                    caseToggle = kind.caseToggle,
                ),
                confirm,
            )
        is KeypadKind.Keyboard ->
            KeypadRequest.Keyboard(
                TextKeyboardSpec(
                    tag = "keyboard",
                    title = title,
                    placeholder = placeholder,
                    initial = value,
                    maxLength = kind.maxLength,
                    caseToggle = kind.caseToggle,
                ),
                confirm,
            )
        is KeypadKind.Number ->
            KeypadRequest.Number(
                NumberKeypadSpec(
                    tag = "field_number",
                    title = title,
                    initial = value,
                    maxDigits = kind.maxDigits,
                    exactDigits = kind.exactDigits,
                    min = kind.min,
                    max = kind.max,
                ),
                confirm,
            )
    }
