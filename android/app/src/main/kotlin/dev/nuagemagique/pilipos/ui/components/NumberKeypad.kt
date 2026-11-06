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
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val DialogMaxWidth = 420.dp

/**
 * What a whole number dialog asks (`NumberEntryDialog.vue` without the minus sign): [maxDigits] at most, or [exactDigits]
 * exactly (a national number keeps its leading zeros), else a value between [min] and [max]. [tag] is the prefix of the
 * test tags: `<tag>_dialog`, `<tag>_value`, `<tag>_ok`, `<tag>_cancel`, plus the `numpad_*` keys.
 */
@Immutable
data class NumberKeypadSpec(
    val tag: String,
    val title: String,
    val initial: String = "",
    val maxDigits: Int = 6,
    val exactDigits: Int? = null,
    val min: Long? = null,
    val max: Long? = null,
)

/** Pure rules of the number dialog, as in `NumberEntryDialog.vue`. */
object NumberKeypadRules {
    /** [digits] after a key. */
    fun press(
        digits: String,
        key: NumPadKey,
        maxDigits: Int,
    ): String =
        when (key) {
            NumPadKey.Backspace -> digits.dropLast(1)
            NumPadKey.Clear -> ""
            is NumPadKey.Digit -> if (digits.length < maxDigits) digits + key.digit else digits
        }

    /** Whether [digits] may be confirmed under [spec]. */
    fun valid(
        digits: String,
        spec: NumberKeypadSpec,
    ): Boolean {
        val n = digits.toLongOrNull()
        return when {
            digits.isEmpty() -> false
            spec.exactDigits != null -> digits.length == spec.exactDigits
            n == null -> false
            else -> n >= (spec.min ?: 0L) && n <= (spec.max ?: Long.MAX_VALUE)
        }
    }

    /** The value confirmed for [digits]: as typed for an exact length, else the plain number ("007" gives "7"). */
    fun result(
        digits: String,
        spec: NumberKeypadSpec,
    ): String = if (spec.exactDigits != null) digits else (digits.toLongOrNull() ?: 0L).toString()
}

/** The digits of [text] before any decimal mark: the pad takes whole numbers, "12.5" opens as 12. */
private fun wholePart(text: String): String = text.substringBefore('.').substringBefore(',').filter { it.isDigit() }

/** The whole number dialog of a settings field on the shared [NumPad]. */
@Composable
fun NumberKeypad(
    spec: NumberKeypadSpec,
    onConfirm: (String) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var digits by rememberSaveable(spec.tag) { mutableStateOf(wholePart(spec.initial).take(spec.maxDigits)) }
    val colors = PiliTheme.colors
    val tag = spec.tag
    Box(modifier = modifier.fillMaxSize().testTag("${tag}_dialog")) {
        PiliDialog(
            title = spec.title,
            onDismissRequest = onCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("${tag}_cancel"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = { onConfirm(NumberKeypadRules.result(digits, spec)) },
                    enabled = NumberKeypadRules.valid(digits, spec),
                    modifier = Modifier.testTag("${tag}_ok"),
                )
            },
        ) {
            Column(
                Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                Text(
                    text = digits.ifEmpty { "0" },
                    style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeXl),
                    color = colors.accentPrimary,
                    modifier = Modifier.testTag("${tag}_value"),
                )
                NumPad(
                    onKey = { digits = NumberKeypadRules.press(digits, it, spec.maxDigits) },
                    clearLabel = uiText(R.string.ui_common_numpad_clear),
                    backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
                )
            }
        }
    }
}
