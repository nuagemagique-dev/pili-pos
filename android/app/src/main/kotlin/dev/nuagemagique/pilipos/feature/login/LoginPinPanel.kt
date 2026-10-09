package dev.nuagemagique.pilipos.feature.login

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PinPad
import dev.nuagemagique.pilipos.ui.components.PinPadTexts
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val ActionHeight = 56.dp
private val LoginButtonWidth = 400.dp
private val LoginButtonHeight = 80.dp
private val ActiveRing = 2.dp

/**
 * The PIN panel beside the tiles (`pin-overlay`): who, the pad that logs in on the 4th digit (or ends the
 * shift in end-shift mode), and the back and end-shift buttons. An operator on shift without a PIN gets a
 * login button instead of the pad.
 */
@Composable
internal fun LoginPinPanel(
    panel: PinPanel,
    state: LoginUiState,
    callbacks: PinCallbacks,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val onShift = state.shiftStart(panel.user.insz) != null
    Column(
        modifier = modifier.testTag("login_pin_panel"),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceLg + PiliTheme.dimens.spaceXs, Alignment.CenterVertically),
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text = vueText(if (panel.endShift) "end_shift" else "pin_code"),
                style = typography.body.copy(fontSize = typography.sizePrice),
                color = colors.textSecondary,
                modifier = Modifier.testTag("login_pin_title"),
            )
            Text(
                text = panel.user.name,
                style = typography.title.copy(fontSize = typography.sizeXl),
                color = colors.textPrimary,
                modifier = Modifier.testTag("login_pin_user"),
            )
        }
        if (panel.user.hasPin) {
            PinPad(
                state = panel.pad,
                texts = padTexts(panel),
                onKey = callbacks.onKey,
                onCancel = callbacks.onBack,
                showTitle = false,
            )
            if (onShift) EndShiftToggle(panel.endShift, callbacks.onToggleEndShift)
        } else {
            NoPinActions(panel, onShift, callbacks)
        }
    }
}

@Composable
private fun padTexts(panel: PinPanel): PinPadTexts =
    PinPadTexts(
        title = "",
        hint = vueText(if (panel.endShift) "end_shift_hint" else "pin_auto_hint"),
        enterPrompt = vueText("pin_enter"),
        confirmPrompt = vueText("pin_confirm"),
        mismatch = vueText("pin_mismatch"),
        cancel = vueText("back"),
        clearLabel = uiText(R.string.ui_common_numpad_clear),
        backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
    )

/** The end-shift button of an operator with a PIN: lit while the PIN ends the shift. */
@Composable
private fun EndShiftToggle(
    active: Boolean,
    onToggle: () -> Unit,
) {
    val ring =
        if (active) {
            Modifier.border(
                ActiveRing,
                PiliTheme.colors.selectedBg,
                RoundedCornerShape(PiliTheme.dimens.radiusMd),
            )
        } else {
            Modifier
        }
    PiliButton(
        text = vueText("end_shift"),
        onClick = onToggle,
        variant = PiliButtonVariant.Secondary,
        modifier = ring.width(LoginButtonWidth / 2).height(ActionHeight).testTag("login_end_shift"),
    )
}

/** An operator without a PIN who is on shift: log in, or end the shift, or go back. */
@Composable
private fun NoPinActions(
    panel: PinPanel,
    onShift: Boolean,
    callbacks: PinCallbacks,
) {
    panel.error?.let { ErrorLine(it, Modifier.testTag("login_pin_error")) }
    PiliButton(
        text = vueText("login"),
        onClick = callbacks.onLogin,
        modifier = Modifier.width(LoginButtonWidth).height(LoginButtonHeight).testTag("login_pin_login"),
    )
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        PiliButton(
            text = vueText("back"),
            onClick = callbacks.onBack,
            variant = PiliButtonVariant.Secondary,
            modifier = Modifier.height(ActionHeight).testTag("login_pin_back"),
        )
        if (onShift) {
            PiliButton(
                text = vueText("end_shift"),
                onClick = callbacks.onEndShift,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.height(ActionHeight).testTag("login_end_shift"),
            )
        }
    }
}
