package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The most digits an amount takes: up to 9 999,99 EUR, far above any menu price, and the display stays short. */
const val AMOUNT_MAX_DIGITS: Int = 6

/** Width of the dialog card: the pad (296 dp) plus the card padding. */
private val AmountDialogMaxWidth = 400.dp

/**
 * The digits typed in an [AmountDialog], read as cents like a payment terminal: `2 0 0 0` is 20,00 EUR.
 * At most [AMOUNT_MAX_DIGITS] digits and no leading zero; BACKSPACE erases the last digit, CLEAR all of them.
 * Pure, so the rules are tested without a screen.
 */
@Immutable
data class AmountEntryState(
    val digits: String = "",
) {
    /** The digits read as a number (no arithmetic: a parse of the text); 0 while nothing is typed. */
    val cents: Long get() = digits.toLongOrNull() ?: 0L

    /** The state after [key]; a digit that would break a rule leaves the state as it is. */
    fun press(key: NumPadKey): AmountEntryState =
        when (key) {
            NumPadKey.Backspace -> AmountEntryState(digits.dropLast(1))
            NumPadKey.Clear -> AmountEntryState()
            is NumPadKey.Digit ->
                if (digits.length >= AMOUNT_MAX_DIGITS || (digits.isEmpty() && key.digit == '0')) {
                    this
                } else {
                    AmountEntryState(digits + key.digit)
                }
        }

    /** True when the amount may be confirmed: above 0, or 0 when [allowZero] (an empty drawer counted). */
    fun isValid(allowZero: Boolean): Boolean = cents > 0L || (allowZero && cents == 0L)
}

/** The captions of an [AmountDialog], resolved by the caller. */
@Immutable
data class AmountDialogTexts(
    val confirm: String,
    val cancel: String,
    val clearLabel: String,
    val backspaceLabel: String,
)

/** The usual captions: Vue `validate` and `back`, and the two glyph keys of the pad. Needs `ProvideUiText` above. */
@Composable
fun defaultAmountDialogTexts(): AmountDialogTexts =
    AmountDialogTexts(
        confirm = vueText("validate"),
        cancel = vueText("back"),
        clearLabel = uiText(R.string.ui_common_numpad_clear),
        backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
    )

/**
 * `PriceDialog.vue`: amount entry on a [NumPad], for the price of a free-price product or the cash counted in
 * the drawer. The typed amount is shown with [formatEuro]; confirm is enabled when the amount is valid
 * ([allowZero] lets 0,00 EUR through; [initialDigits] starts from a known amount, in cents) and calls [onConfirm] with
 * the cents. A tap outside the card or the cancel button calls [onCancel].
 *
 * Test tags: `amount_dialog` (root), `amount_value`, `amount_confirm`, `amount_cancel`, plus the pad keys.
 */
@Composable
fun AmountDialog(
    title: String,
    itemName: String?,
    texts: AmountDialogTexts,
    onConfirm: (cents: Long) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
    allowZero: Boolean = false,
    initialDigits: String = "",
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    var digits by rememberSaveable { mutableStateOf(initialDigits) }
    val state = AmountEntryState(digits)
    Box(modifier = modifier.fillMaxSize().testTag("amount_dialog")) {
        PiliDialog(
            title = title,
            onDismissRequest = onCancel,
            maxWidth = AmountDialogMaxWidth,
            actions = {
                PiliButton(
                    text = texts.cancel,
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("amount_cancel"),
                )
                PiliButton(
                    text = texts.confirm,
                    onClick = { if (state.isValid(allowZero)) onConfirm(state.cents) },
                    enabled = state.isValid(allowZero),
                    modifier = Modifier.testTag("amount_confirm"),
                )
            },
        ) {
            Column(
                modifier = Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                if (itemName != null) {
                    Text(itemName, style = typography.body, color = colors.textSecondary)
                }
                Text(
                    text = formatEuro(state.cents),
                    style = typography.price.copy(fontSize = typography.sizeXl),
                    color = colors.accentPrimary,
                    modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite }.testTag("amount_value"),
                )
                NumPad(
                    onKey = { key -> digits = state.press(key).digits },
                    clearLabel = texts.clearLabel,
                    backspaceLabel = texts.backspaceLabel,
                )
            }
        }
    }
}
