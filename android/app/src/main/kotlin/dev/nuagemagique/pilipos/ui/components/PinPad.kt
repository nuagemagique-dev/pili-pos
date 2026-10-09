package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** What the PIN pad collects. */
enum class PinMode {
    /** One PIN, typed once; ready to submit on the 4th digit (confirm an action, log in). */
    Verify,

    /** A new PIN, typed twice; ready to submit when the confirmation matches. */
    Choose,
}

/** Step of a [PinMode.Choose] pad. */
enum class PinStep {
    /** Typing the PIN. */
    Enter,

    /** Typing it again. */
    Confirm,
}

/**
 * State of a PIN pad, the logic of `PinDialog.vue` without the UI: a PIN is exactly [PIN_LENGTH] digits.
 *
 * [apply] is the `onKey` of the Vue dialog. It returns the PIN to submit when one is ready (the 4th digit in
 * [PinMode.Verify], a matching confirmation in [PinMode.Choose]) and then marks the pad [busy] so that every
 * further key is ignored until [succeeded] or [failed] is called; a PIN is therefore never submitted twice.
 * [mismatchText] is the text shown when a confirmation differs (the caller resolves `pin_mismatch`).
 *
 * The properties are Compose snapshot state (runtime only, no UI) so that a [PinPad] redraws on a change;
 * the class has no other dependency and runs in a plain JVM test.
 */
@Stable
class PinPadState(
    val mode: PinMode = PinMode.Verify,
    private val mismatchText: String = "",
) {
    /** The digits typed so far on the current step, at most [PIN_LENGTH]. */
    var digits: String by mutableStateOf("")
        private set

    /** The current step; always [PinStep.Enter] in [PinMode.Verify]. */
    var step: PinStep by mutableStateOf(PinStep.Enter)
        private set

    /** The error line, or null. */
    var error: String? by mutableStateOf(null)
        private set

    /** True while a submission is running; keys are ignored. */
    var busy: Boolean by mutableStateOf(false)
        private set

    private var first: String = ""

    /** True when [digits] is complete and a submission is due. */
    val isReady: Boolean
        get() = digits.length == PIN_LENGTH && (mode == PinMode.Verify || step == PinStep.Confirm)

    /** Applies [key]; returns the PIN to submit when one is ready, else null. */
    fun apply(key: NumPadKey): String? {
        if (busy) return null
        when (key) {
            NumPadKey.Backspace -> digits = digits.dropLast(1)
            NumPadKey.Clear -> digits = ""
            is NumPadKey.Digit -> if (digits.length < PIN_LENGTH) digits += key.digit
        }
        return if (digits.length < PIN_LENGTH) null else complete()
    }

    /** [digits] holds a full PIN: submit it, move on to the confirmation, or reject a mismatch. */
    private fun complete(): String? =
        when {
            mode == PinMode.Verify -> submit()
            step == PinStep.Enter -> {
                first = digits
                digits = ""
                error = null
                step = PinStep.Confirm
                null
            }
            digits != first -> {
                restart(mismatchText)
                null
            }
            else -> submit()
        }

    /** The submission succeeded: the pad is cleared and accepts keys again. */
    fun succeeded() {
        busy = false
        digits = ""
        first = ""
        step = PinStep.Enter
    }

    /** The submission failed with [message]: the pad is cleared ([PinMode.Choose] starts over at Enter). */
    fun failed(message: String) {
        busy = false
        restart(message)
    }

    private fun submit(): String {
        busy = true
        error = null
        return digits
    }

    private fun restart(message: String) {
        error = message
        first = ""
        digits = ""
        step = PinStep.Enter
    }

    companion object {
        /** Number of digits of a PIN. */
        const val PIN_LENGTH: Int = 4
    }
}

/**
 * The texts of a PIN pad, all resolved by the caller. Vue keys of `PinDialog.vue`: [title] is `pin_enter`
 * (verify), `pin_choose` (first login) or `pin_change`; [hint] is `pin_first_login_hint` or the caller's hint
 * (empty hides it); [enterPrompt] is `pin_enter` or `pin_choose`; [confirmPrompt] is `pin_confirm`;
 * [mismatch] is `pin_mismatch`; [cancel] is `back`. [clearLabel] and [backspaceLabel] describe the two glyph
 * keys of the [NumPad].
 */
data class PinPadTexts(
    val title: String,
    val hint: String,
    val enterPrompt: String,
    val confirmPrompt: String,
    val mismatch: String,
    val cancel: String,
    val clearLabel: String,
    val backspaceLabel: String,
)

private val DotSize = 20.dp
private val DotBorder = 2.dp
private val HintMaxWidth = 352.dp

/**
 * The PIN pad of `PinDialog.vue`: title, hint, the prompt of the step, the error line, four dots (one filled per
 * digit), the [NumPad] and the cancel button. [showTitle] false hides the title (a [PinDialog] shows it in its
 * frame). Test tags: `pin_pad`, `pin_dot_0` to `pin_dot_3` (selected when filled), `pin_error`, `pin_cancel`.
 */
@Composable
fun PinPad(
    state: PinPadState,
    texts: PinPadTexts,
    onKey: (NumPadKey) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
    showTitle: Boolean = true,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    val typography = PiliTheme.typography
    val prompt = if (state.mode == PinMode.Choose && state.step == PinStep.Confirm) texts.confirmPrompt else texts.enterPrompt
    Column(
        modifier = modifier.testTag("pin_pad"),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
    ) {
        if (showTitle) {
            Text(
                texts.title,
                style = typography.body.copy(fontSize = typography.sizeLg, fontWeight = typography.weightBold),
                color = colors.textPrimary,
            )
        }
        if (texts.hint.isNotBlank()) {
            Text(
                texts.hint,
                style = typography.body.copy(fontSize = typography.sizeSm),
                color = colors.textSecondary,
                textAlign = TextAlign.Center,
                modifier = Modifier.widthIn(max = HintMaxWidth),
            )
        }
        Text(prompt, style = typography.body, color = colors.textSecondary, textAlign = TextAlign.Center)
        state.error?.let { message ->
            Text(
                message,
                style = typography.body.copy(fontWeight = typography.weightBold),
                color = colors.accentDanger,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("pin_error"),
            )
        }
        PinDots(filledCount = state.digits.length)
        NumPad(
            onKey = onKey,
            clearLabel = texts.clearLabel,
            backspaceLabel = texts.backspaceLabel,
            enabled = !state.busy,
        )
        PiliButton(
            text = texts.cancel,
            onClick = onCancel,
            variant = PiliButtonVariant.Secondary,
            modifier = Modifier.fillMaxWidth().testTag("pin_cancel"),
        )
    }
}

/** The four dots of the PIN, one filled per digit (selected in the semantics tree). */
@Composable
private fun PinDots(filledCount: Int) {
    val colors = PiliTheme.colors
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
        repeat(PinPadState.PIN_LENGTH) { i ->
            val filled = i < filledCount
            Row(
                Modifier
                    .testTag("pin_dot_$i")
                    .semantics { selected = filled }
                    .size(DotSize)
                    .background(if (filled) colors.accentPrimary else colors.bgSecondary, CircleShape)
                    .border(DotBorder, if (filled) colors.accentPrimary else colors.textSecondary, CircleShape),
            ) {}
        }
    }
}
