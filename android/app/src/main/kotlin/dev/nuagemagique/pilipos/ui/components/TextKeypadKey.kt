package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val BackspaceIconSize = 24.dp

/**
 * A key of the text keypads ([TextKeypad], [TextKeyboard]): surface fill, [tapScale] and a darker fill while pressed
 * ([on] draws the selected fill of a held toggle such as the shift key). [description] is the accessibility label of
 * a glyph key.
 */
@Composable
internal fun TextKeypadKey(
    modifier: Modifier,
    onClick: () -> Unit,
    description: String? = null,
    on: Boolean = false,
    content: @Composable () -> Unit,
) {
    val colors = PiliTheme.colors
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val fill =
        when {
            on -> colors.selectedBg
            pressed -> colors.bgPressed
            else -> colors.bgSurface
        }
    Box(
        modifier =
            modifier
                .tapScale(interactionSource)
                .background(fill, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(interactionSource = interactionSource, indication = null, role = Role.Button, onClick = onClick)
                .then(if (description != null) Modifier.semantics { contentDescription = description } else Modifier),
        contentAlignment = Alignment.Center,
    ) {
        content()
    }
}

/** The text of a character key. */
@Composable
internal fun KeyLabel(
    text: String,
    color: Color,
) {
    val typography = PiliTheme.typography
    Text(text, style = typography.body.copy(fontSize = typography.sizeLg, fontWeight = typography.weightBold), color = color)
}

/** The backspace glyph, red as on the Vue keys. */
@Composable
internal fun BackspaceGlyph() {
    Icon(
        imageVector = Icons.AutoMirrored.Outlined.Backspace,
        contentDescription = null,
        tint = PiliTheme.colors.accentDanger,
        modifier = Modifier.size(BackspaceIconSize),
    )
}
