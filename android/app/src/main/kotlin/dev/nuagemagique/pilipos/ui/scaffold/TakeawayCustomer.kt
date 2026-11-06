package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The longest customer name (`TAKEAWAY_NAME_MAX` of `utils/takeaway.ts`; the engine refuses more). */
internal const val TAKEAWAY_NAME_MAX: Int = 30

/** The most digits a customer phone takes on the numeric pad (`TAKEAWAY_PHONE_MAX_DIGITS`). */
internal const val TAKEAWAY_PHONE_MAX_DIGITS: Int = 15

/** The two steps of adding a customer to a takeaway (UX audit T2): the name, then the optional phone. */
internal enum class CustomerStep { NAME, PHONE }

/** The words of the takeaway chip, as in `utils/takeaway.ts`. */
internal object TakeawayText {
    /** "À emporter E-61 · Dupont": the badge, the number of the day and the customer name, as far as known. */
    fun title(
        badge: String,
        label: String?,
        name: String?,
    ): String {
        if (label == null) return badge
        val who = name?.trim().orEmpty()
        return if (who.isEmpty()) "$badge $label" else "$badge $label · $who"
    }
}

/** The phone typed on the numeric pad (`phoneKey`, `phoneDisplay` and `phoneValue` of `utils/takeaway.ts`). */
internal object PhoneEntry {
    private const val HEAD = 4
    private const val GROUP = 2

    /** The digits after [key]: a digit that would pass the limit leaves them as they are. */
    fun press(
        digits: String,
        key: NumPadKey,
    ): String =
        when (key) {
            NumPadKey.Backspace -> digits.dropLast(1)
            NumPadKey.Clear -> ""
            is NumPadKey.Digit -> if (digits.length >= TAKEAWAY_PHONE_MAX_DIGITS) digits else digits + key.digit
        }

    /** The digits in groups, "0470 12 34 56". */
    fun display(digits: String): String =
        if (digits.length <= HEAD) {
            digits
        } else {
            (listOf(digits.take(HEAD)) + digits.drop(HEAD).chunked(GROUP)).joinToString(" ")
        }

    /** What is sent for a phone: null when empty. */
    fun value(digits: String): String? = if (digits.isEmpty()) null else display(digits)
}

private val DialogMaxWidth = 720.dp
private val PhoneDialogMaxWidth = 400.dp
private val FieldHeight = 64.dp
private val Hairline = 1.dp
private val ButtonHeight = 48.dp

/** The customer name typed in the dialog: no control character, at most [TAKEAWAY_NAME_MAX] characters. */
internal fun customerNameText(typed: String): String = typed.filterNot { it.isISOControl() }.take(TAKEAWAY_NAME_MAX)

/**
 * The name of the customer of a takeaway (`TextKeypad` of `TakeawayCustomer.vue`). Typed on the system keyboard like
 * the kitchen note until the in-app keyboard lands (UX audit C13). An empty text clears the name; the engine validates
 * (`takeaway_name_invalid`). Test tags: `customer_name_dialog`, `customer_name_text`, `customer_name_counter`,
 * `customer_name_confirm`, `customer_name_cancel`.
 */
@Composable
internal fun CustomerNameDialog(
    initial: String,
    onConfirm: (String) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val start = customerNameText(initial)
    // The cursor starts after the current name, so a correction types at its end.
    var field by rememberSaveable(stateSaver = TextFieldValue.Saver) { mutableStateOf(TextFieldValue(start, TextRange(start.length))) }
    val text = field.text
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Box(modifier = modifier.fillMaxSize().testTag("customer_name_dialog")) {
        PiliDialog(
            title = vueText("takeaway_name_title"),
            onDismissRequest = onCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("delivery_cancel"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("customer_name_cancel"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = { onConfirm(text.trim()) },
                    modifier = Modifier.testTag("customer_name_confirm"),
                )
            },
        ) {
            NameField(field, { field = it }, focus, onDone = { onConfirm(text.trim()) })
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(
                    text = vueText("takeaway_name_hint"),
                    style = t.body.copy(fontSize = t.sizeSm),
                    color = c.textSecondary,
                    modifier = Modifier.weight(1f),
                )
                Text(
                    text = "${text.length}/$TAKEAWAY_NAME_MAX",
                    style = t.figure.copy(fontSize = t.sizeSm),
                    color = c.textSecondary,
                    modifier = Modifier.testTag("customer_name_counter"),
                )
            }
        }
    }
}

/**
 * The phone of the customer of a takeaway (`PhoneDialog.vue`), digits only on the shared pad, shown in groups, optional:
 * "Valider" with nothing typed clears it, "Passer" leaves it as it was. Test tags: `customer_phone_dialog`,
 * `customer_phone_value`, `customer_phone_confirm`, `customer_phone_skip`, plus the pad keys.
 */
@Composable
internal fun CustomerPhoneDialog(
    initial: String?,
    onConfirm: (digits: String) -> Unit,
    onSkip: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var digits by rememberSaveable { mutableStateOf(initial.orEmpty().filter { it.isDigit() }) }
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Box(modifier = modifier.fillMaxSize().testTag("customer_phone_dialog")) {
        PiliDialog(
            title = vueText("takeaway_phone_title"),
            onDismissRequest = onSkip,
            maxWidth = PhoneDialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("takeaway_phone_skip"),
                    onClick = onSkip,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("customer_phone_skip"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = { onConfirm(digits) },
                    modifier = Modifier.testTag("customer_phone_confirm"),
                )
            },
        ) {
            Column(
                modifier = Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                Text(
                    text = PhoneEntry.display(digits).ifEmpty { vueText("takeaway_phone_none") },
                    style = t.figure.copy(fontSize = t.sizeXl),
                    color = c.accentPrimary,
                    modifier = Modifier.testTag("customer_phone_value"),
                )
                NumPad(
                    onKey = { key -> digits = PhoneEntry.press(digits, key) },
                    clearLabel = uiText(R.string.ui_common_numpad_clear),
                    backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
                )
            }
        }
    }
}

/** "Ajouter un nom" / "Modifier le client": a 48 dp button next to the takeaway title of the header. Tag `header_takeaway_customer`. */
@Composable
internal fun CustomerButton(
    text: String,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    Box(
        modifier =
            Modifier
                .height(ButtonHeight)
                .tapScale(source)
                .background(c.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .padding(horizontal = 16.dp)
                .testTag("header_takeaway_customer"),
        contentAlignment = Alignment.Center,
    ) {
        Text(text, style = t.body.copy(fontSize = t.sizeSm, fontWeight = t.weightBold), color = c.textPrimary, maxLines = 1)
    }
}

/** The single line of the name dialog, with its placeholder (`customer_name_text`). */
@Composable
private fun NameField(
    value: TextFieldValue,
    onValueChange: (TextFieldValue) -> Unit,
    focus: FocusRequester,
    onDone: () -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    BasicTextField(
        value = value,
        onValueChange = { onValueChange(it.copy(text = customerNameText(it.text))) },
        singleLine = true,
        textStyle = t.body.copy(color = c.textPrimary, fontSize = t.sizeLg),
        cursorBrush = SolidColor(c.textPrimary),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
        modifier = Modifier.fillMaxWidth().focusRequester(focus).testTag("customer_name_text"),
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = FieldHeight)
                    .border(BorderStroke(Hairline, c.border), RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .padding(horizontal = PiliTheme.dimens.spaceMd),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.text.isEmpty()) {
                    Text(
                        vueText("takeaway_name_placeholder"),
                        style = t.body,
                        color = c.textSecondary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                inner()
            }
        },
    )
}
