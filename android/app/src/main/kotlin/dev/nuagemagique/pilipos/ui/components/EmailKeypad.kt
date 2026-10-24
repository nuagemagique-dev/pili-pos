package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Backspace
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The longest address the keypad takes (Vue `MAX_LEN`). */
const val EMAIL_MAX_LENGTH: Int = 60

/** The rows of letters, as in `EmailKeypad.vue`. */
private val LetterRows: List<String> = listOf("qwertyuiop", "asdfghjkl", "zxcvbnm")

/** The digit row. */
private const val DIGITS = "1234567890"

/** The symbols of the last row (the backspace key follows them). */
private const val SYMBOLS = "@.-_+"

/** The chunks typed in one tap. */
private val QuickChunks: List<String> = listOf(".be", ".com", "@gmail.com")

/** Local part, an at sign, a domain with a dot: the shape of Vue `isValid` (`^[^\s@]+@[^\s@]+\.[^\s@]+$`). */
private val EmailShape = Regex("^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$")

private val KeypadMaxWidth = 820.dp
private val LetterKeyWidth = 64.dp
private val LetterKeyHeight = 56.dp
private val QuickKeyHeight = 48.dp
private val ActionHeight = 64.dp
private val DisplayMinHeight = 64.dp
private val BackspaceIconSize = 24.dp
private val InvalidBorder = 2.dp

/** The two steps of the keypad: typing the address, then confirming it in large type (cert requirement). */
enum class EmailKeypadStep { Type, Confirm }

/**
 * What the operator has typed and where the keypad stands (the logic of `EmailKeypad.vue`, pure so it is tested
 * without a screen). The address is checked locally only to save a round trip on an obvious typo: the engine
 * validates it again before signing.
 */
@Immutable
data class EmailKeypadState(
    val buffer: String = "",
    val step: EmailKeypadStep = EmailKeypadStep.Type,
) {
    /** True when [buffer] has the shape local@domain.tld without a blank. */
    val isValid: Boolean get() = EmailShape.matches(buffer)

    /** True when the error line and the red border are shown: something is typed and it is not valid. */
    val showsError: Boolean get() = buffer.isNotEmpty() && !isValid

    /** [chunk] appended; unchanged when the result would pass [EMAIL_MAX_LENGTH] characters. */
    fun append(chunk: String): EmailKeypadState =
        if (buffer.length + chunk.length > EMAIL_MAX_LENGTH) this else copy(buffer = buffer + chunk)

    /** The last character erased. */
    fun backspace(): EmailKeypadState = copy(buffer = buffer.dropLast(1))

    /** Everything erased. */
    fun clear(): EmailKeypadState = copy(buffer = "")

    /** The confirm step; unchanged while the address is not valid. */
    fun goConfirm(): EmailKeypadState = if (isValid) copy(step = EmailKeypadStep.Confirm) else this

    /** Back to typing, the buffer kept. */
    fun goBackToType(): EmailKeypadState = copy(step = EmailKeypadStep.Type)
}

/** The captions of an [EmailKeypad], resolved by the caller. */
@Immutable
data class EmailKeypadTexts(
    val title: String,
    val placeholder: String,
    val invalid: String,
    val confirmTitle: String,
    val confirmAction: String,
    val edit: String,
    val cancel: String,
    val clear: String,
    val validate: String,
    val backspaceLabel: String,
)

/** The Vue captions (`delivery_*`, `clear`, `validate`; the backspace key reads `delivery_edit`). Needs `ProvideUiText` above. */
@Composable
fun defaultEmailKeypadTexts(): EmailKeypadTexts =
    EmailKeypadTexts(
        title = vueText("delivery_enter_email_title"),
        placeholder = vueText("delivery_email_placeholder"),
        invalid = vueText("delivery_invalid_email"),
        confirmTitle = vueText("delivery_confirm_title"),
        confirmAction = vueText("delivery_confirm_btn"),
        edit = vueText("delivery_edit"),
        cancel = vueText("delivery_cancel"),
        clear = vueText("clear"),
        validate = vueText("validate"),
        backspaceLabel = vueText("delivery_edit"),
    )

/**
 * `EmailKeypad.vue`: the on-screen keyboard that takes a customer's e-mail address (the till has no physical
 * keyboard). Two steps: type, with live validation, then confirm in large type. [onConfirm] gets the address
 * after the confirm step; [onCancel] is called by the cancel button and by a tap outside the card.
 *
 * Drawn in the window like every dialog ([PiliDialog]): put it last in a `Box` that fills the screen.
 * Test tags: `email_keypad` (root), `email_display`, `email_buffer`, `email_placeholder`, `email_error`,
 * `email_key_<char>`, `email_quick_<chunk>`, `email_backspace`, `email_cancel`, `email_clear`, `email_validate`,
 * `email_confirm_display`, `email_edit`, `email_confirm`.
 */
@Composable
fun EmailKeypad(
    texts: EmailKeypadTexts,
    onConfirm: (email: String) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var buffer by rememberSaveable { mutableStateOf("") }
    var confirming by rememberSaveable { mutableStateOf(false) }
    val state = EmailKeypadState(buffer, if (confirming) EmailKeypadStep.Confirm else EmailKeypadStep.Type)

    fun apply(next: EmailKeypadState) {
        buffer = next.buffer
        confirming = next.step == EmailKeypadStep.Confirm
    }
    Box(modifier = modifier.fillMaxSize().testTag("email_keypad")) {
        PiliDialog(
            title = if (state.step == EmailKeypadStep.Type) texts.title else texts.confirmTitle,
            onDismissRequest = onCancel,
            maxWidth = KeypadMaxWidth,
        ) {
            when (state.step) {
                EmailKeypadStep.Type -> TypeStep(state, texts, ::apply, onCancel)
                EmailKeypadStep.Confirm -> ConfirmStep(state, texts, ::apply, onConfirm)
            }
        }
    }
}

@Composable
private fun TypeStep(
    state: EmailKeypadState,
    texts: EmailKeypadTexts,
    apply: (EmailKeypadState) -> Unit,
    onCancel: () -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val dimens = PiliTheme.dimens
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
    ) {
        AddressDisplay(state, texts)
        if (state.showsError) {
            Text(texts.invalid, style = typography.label, color = colors.accentDanger, modifier = Modifier.testTag("email_error"))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(dimens.spaceSm)) {
            QuickChunks.forEach { chunk ->
                KeypadKey(
                    modifier = Modifier.height(QuickKeyHeight).testTag("email_quick_$chunk"),
                    onClick = { apply(state.append(chunk)) },
                    wide = true,
                ) {
                    Text(chunk, style = typography.label.copy(fontWeight = typography.weightBold), color = colors.textSecondary)
                }
            }
        }
        KeyGrid(state, texts, apply)
        ActionRow {
            PiliButton(
                text = texts.cancel,
                onClick = onCancel,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("email_cancel"),
            )
            PiliButton(
                text = texts.clear,
                onClick = { apply(state.clear()) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("email_clear"),
            )
            PiliButton(
                text = texts.validate,
                onClick = { apply(state.goConfirm()) },
                enabled = state.isValid,
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("email_validate"),
            )
        }
    }
}

/** The address typed so far (or the placeholder), with a red border while it is not valid. */
@Composable
private fun AddressDisplay(
    state: EmailKeypadState,
    texts: EmailKeypadTexts,
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
                    BorderStroke(InvalidBorder, if (state.showsError) colors.accentDanger else Color.Transparent),
                    RoundedCornerShape(dimens.radiusMd),
                ).padding(dimens.spaceMd)
                .testTag("email_display"),
        contentAlignment = Alignment.Center,
    ) {
        if (state.buffer.isEmpty()) {
            Text(
                texts.placeholder,
                style = typography.body,
                color = colors.textSecondary,
                modifier = Modifier.testTag("email_placeholder"),
            )
        } else {
            Text(
                state.buffer,
                style = typography.body.copy(fontSize = typography.sizeXl, fontWeight = typography.weightBold),
                color = colors.textPrimary,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("email_buffer"),
            )
        }
    }
}

/** The letter rows, the digit row, and the symbols with the backspace key. */
@Composable
private fun KeyGrid(
    state: EmailKeypadState,
    texts: EmailKeypadTexts,
    apply: (EmailKeypadState) -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Column(verticalArrangement = Arrangement.spacedBy(dimens.spaceSm), horizontalAlignment = Alignment.CenterHorizontally) {
        LetterRows.forEach { row -> CharRow(row, state, apply, colors.textPrimary) }
        CharRow(DIGITS, state, apply, colors.textPrimary)
        Row(horizontalArrangement = Arrangement.spacedBy(dimens.spaceSm)) {
            SYMBOLS.forEach { ch -> CharKey(ch, state, apply, colors.textSecondary) }
            KeypadKey(
                modifier = Modifier.size(LetterKeyWidth, LetterKeyHeight).testTag("email_backspace"),
                onClick = { apply(state.backspace()) },
                description = texts.backspaceLabel,
            ) {
                Icon(
                    imageVector = Icons.AutoMirrored.Outlined.Backspace,
                    contentDescription = null,
                    tint = colors.accentDanger,
                    modifier = Modifier.size(BackspaceIconSize),
                )
            }
        }
    }
}

@Composable
private fun ConfirmStep(
    state: EmailKeypadState,
    texts: EmailKeypadTexts,
    apply: (EmailKeypadState) -> Unit,
    onConfirm: (String) -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val dimens = PiliTheme.dimens
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
    ) {
        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .background(colors.bgPrimary, RoundedCornerShape(dimens.radiusMd))
                    .padding(vertical = dimens.spaceXl, horizontal = dimens.spaceMd),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                state.buffer,
                style = typography.body.copy(fontSize = typography.sizeXl, fontWeight = typography.weightHeavy),
                color = colors.textPrimary,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("email_confirm_display"),
            )
        }
        ActionRow {
            PiliButton(
                text = texts.edit,
                onClick = { apply(state.goBackToType()) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("email_edit"),
            )
            PiliButton(
                text = texts.confirmAction,
                onClick = { onConfirm(state.buffer) },
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("email_confirm"),
            )
        }
    }
}

@Composable
private fun ActionRow(content: @Composable RowScope.() -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        content = content,
    )
}

@Composable
private fun CharRow(
    chars: String,
    state: EmailKeypadState,
    apply: (EmailKeypadState) -> Unit,
    color: Color,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        chars.forEach { ch -> CharKey(ch, state, apply, color) }
    }
}

@Composable
private fun CharKey(
    ch: Char,
    state: EmailKeypadState,
    apply: (EmailKeypadState) -> Unit,
    color: Color,
) {
    KeypadKey(
        modifier = Modifier.size(LetterKeyWidth, LetterKeyHeight).testTag("email_key_$ch"),
        onClick = { apply(state.append(ch.toString())) },
    ) {
        val typography = PiliTheme.typography
        Text(
            ch.toString(),
            style = typography.body.copy(fontSize = typography.sizeLg, fontWeight = typography.weightBold),
            color = color,
        )
    }
}

/** A key: surface fill, [tapScale] and a darker fill while pressed. [wide] pads the sides instead of fixing the width. */
@Composable
private fun KeypadKey(
    modifier: Modifier,
    onClick: () -> Unit,
    description: String? = null,
    wide: Boolean = false,
    content: @Composable () -> Unit,
) {
    val colors = PiliTheme.colors
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    Box(
        modifier =
            modifier
                .tapScale(interactionSource)
                .background(if (pressed) colors.bgPressed else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    role = Role.Button,
                    onClick = onClick,
                ).then(if (wide) Modifier.padding(horizontal = PiliTheme.dimens.spaceMd) else Modifier)
                .then(if (description != null) Modifier.semantics { contentDescription = description } else Modifier),
        contentAlignment = Alignment.Center,
    ) {
        content()
    }
}
