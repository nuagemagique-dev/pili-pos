package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import dev.nuagemagique.pilipos.ui.components.KeypadField
import dev.nuagemagique.pilipos.ui.components.KeypadKind

/**
 * A single line field of the invoice customer form, typed on the in-app AZERTY keyboard (C13 / R6: the till never raises
 * the system keyboard, `InvoiceDialog.vue` has its own). A tap opens the keyboard in a dialog headed by [title]; capital
 * letters are its default and the view model keeps the text in capitals. [placeholder] shows while the field is empty;
 * [mono] draws the value in the figure font (the VAT number); [maxLength] is the most characters the field takes.
 */
@Composable
internal fun InvoiceTextField(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    tag: String,
    title: String,
    modifier: Modifier = Modifier,
    mono: Boolean = false,
    maxLength: Int = TEXT_MAX_CHARS,
) {
    KeypadField(
        value = value,
        onValueChange = onValueChange,
        title = title,
        placeholder = placeholder,
        modifier = modifier.testTag(tag),
        kind = KeypadKind.Keyboard(maxLength = maxLength),
        mono = mono,
    )
}
