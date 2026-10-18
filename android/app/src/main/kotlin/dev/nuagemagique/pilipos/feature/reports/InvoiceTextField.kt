package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val FieldHeight = 56.dp

/**
 * A single line field of the invoice customer form, typed on the system keyboard (V11-D19: the Vue UI has its own
 * on-screen keyboard). Capital letters are the keyboard's default; the view model keeps the text in capitals.
 * [placeholder] shows while the field is empty; [mono] draws the value in the figure font (the VAT number).
 */
@Composable
internal fun InvoiceTextField(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    tag: String,
    modifier: Modifier = Modifier,
    mono: Boolean = false,
    imeAction: ImeAction = ImeAction.Next,
) {
    val colors = PiliTheme.colors
    val style =
        PiliTheme.typography.body.copy(
            color = colors.textPrimary,
            fontFamily = if (mono) PiliTheme.typography.familyFigures else FontFamily.Default,
        )
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        singleLine = true,
        textStyle = style,
        cursorBrush = SolidColor(colors.textPrimary),
        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters, imeAction = imeAction),
        keyboardActions = KeyboardActions(),
        modifier = modifier.testTag(tag),
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(FieldHeight)
                    .border(1.dp, colors.border, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .padding(horizontal = PiliTheme.dimens.spaceMd),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.isEmpty()) {
                    Text(placeholder, style = PiliTheme.typography.body, color = colors.textSecondary, maxLines = 1)
                }
                inner()
            }
        },
    )
}
