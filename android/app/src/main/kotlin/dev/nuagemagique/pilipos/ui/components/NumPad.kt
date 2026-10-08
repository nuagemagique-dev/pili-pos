@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Backspace
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** A key of the [NumPad]. */
sealed interface NumPadKey {
    /** A digit key, `'0'` to `'9'`. */
    data class Digit(
        val digit: Char,
    ) : NumPadKey {
        init {
            require(digit in '0'..'9') { "not a digit: $digit" }
        }
    }

    /** The C key: erase everything typed. */
    data object Clear : NumPadKey

    /** The backspace key: erase the last digit. */
    data object Backspace : NumPadKey
}

/** Width of a key (UX guide section 4.1: 96 x 72 recommended). */
private val KeyWidth = 96.dp

/** Height of a key. */
private val KeyHeight = 72.dp

private const val DISABLED_ALPHA = 0.4f

private val DigitRows: List<List<Char>> = listOf(listOf('1', '2', '3'), listOf('4', '5', '6'), listOf('7', '8', '9'))

/**
 * The numeric pad of `NumPad.vue`: three columns of 96 x 72 dp keys, rows 1 2 3 / 4 5 6 / 7 8 9 / C 0 backspace.
 * [clearLabel] and [backspaceLabel] are the content descriptions of the two glyph keys (the caller resolves the
 * text). Test tags: `numpad_key_0` to `numpad_key_9`, `numpad_clear`, `numpad_back`.
 */
@Composable
fun NumPad(
    onKey: (NumPadKey) -> Unit,
    clearLabel: String,
    backspaceLabel: String,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val digitStyle = typography.body.copy(fontSize = typography.sizeXl, fontWeight = typography.weightBold)
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        DigitRows.forEach { row ->
            KeyRow {
                row.forEach { d ->
                    PadKey(Modifier.testTag("numpad_key_$d"), enabled, { onKey(NumPadKey.Digit(d)) }) {
                        Text(d.toString(), style = digitStyle, color = colors.textPrimary)
                    }
                }
            }
        }
        KeyRow {
            PadKey(Modifier.testTag("numpad_clear"), enabled, { onKey(NumPadKey.Clear) }, clearLabel) {
                Text("C", style = digitStyle, color = colors.accentWarning)
            }
            PadKey(Modifier.testTag("numpad_key_0"), enabled, { onKey(NumPadKey.Digit('0')) }) {
                Text("0", style = digitStyle, color = colors.textPrimary)
            }
            PadKey(Modifier.testTag("numpad_back"), enabled, { onKey(NumPadKey.Backspace) }, backspaceLabel) {
                Icon(
                    imageVector = Icons.AutoMirrored.Outlined.Backspace,
                    contentDescription = null,
                    tint = colors.accentDanger,
                    modifier = Modifier.size(PiliTheme.dimens.spaceLg + PiliTheme.dimens.spaceXs),
                )
            }
        }
    }
}

@Composable
private fun KeyRow(content: @Composable () -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) { content() }
}

@Composable
private fun PadKey(
    modifier: Modifier,
    enabled: Boolean,
    onClick: () -> Unit,
    description: String? = null,
    content: @Composable () -> Unit,
) {
    val colors = PiliTheme.colors
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val fill: Color = if (pressed) colors.bgPressed else colors.bgSurface
    Box(
        modifier =
            modifier
                .size(KeyWidth, KeyHeight)
                .tapScale(interactionSource)
                .alpha(if (enabled) 1f else DISABLED_ALPHA)
                .background(fill, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    enabled = enabled,
                    role = Role.Button,
                    onClick = onClick,
                ).then(if (description != null) Modifier.semantics { contentDescription = description } else Modifier),
        contentAlignment = Alignment.Center,
    ) {
        content()
    }
}
