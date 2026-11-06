package dev.nuagemagique.pilipos.feature.login

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PinPad
import dev.nuagemagique.pilipos.ui.components.PinPadTexts
import dev.nuagemagique.pilipos.ui.state.LanguageChoices
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val ActionHeight = 56.dp
private val LoginButtonWidth = 400.dp
private val LoginButtonHeight = 80.dp
private val LanguageHeight = 48.dp
private val LanguageMinWidth = 72.dp

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
                text = vueText(pinTitleKey(panel, onShift)),
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
        UserLanguageChoice(state.languageOf(panel.user.insz), callbacks.onUserLanguage)
        if (panel.user.hasPin) {
            if (onShift) ShiftChoice(panel.endShift, callbacks.onShiftMode)
            PinPad(
                state = panel.pad,
                texts = padTexts(panel),
                onKey = callbacks.onKey,
                onCancel = callbacks.onBack,
                showTitle = false,
            )
            if (panel.endShift && state.badgeReaderOn) BadgeHint(vueText("end_shift_badge_hint"), "login_end_shift_badge_hint")
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

/** The title of the pad says which of the two it does: an operator on shift can log in or end the shift. */
private fun pinTitleKey(
    panel: PinPanel,
    onShift: Boolean,
): String =
    when {
        panel.endShift -> "end_shift"
        onShift -> "login_action"
        else -> "pin_code"
    }

/**
 * The two buttons of an operator on shift (UX audit L1): "Se connecter" and "Terminer le service", the chosen one in
 * Brume (a selection, never Volt). Test tags: `login_mode_login`, `login_end_shift`.
 */
@Composable
private fun ShiftChoice(
    endShift: Boolean,
    onMode: (Boolean) -> Unit,
) {
    Row(
        modifier = Modifier.width(LoginButtonWidth),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        ChoiceButton(vueText("login_action"), !endShift, "login_mode_login", Modifier.weight(1f)) { onMode(false) }
        ChoiceButton(vueText("end_shift_action"), endShift, "login_end_shift", Modifier.weight(1f)) { onMode(true) }
    }
}

@Composable
private fun ChoiceButton(
    text: String,
    chosen: Boolean,
    tag: String,
    modifier: Modifier,
    onClick: () -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Box(
        modifier =
            modifier
                .height(ActionHeight)
                .background(if (chosen) colors.selectedBg else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(role = Role.RadioButton, onClick = onClick)
                .semantics { selected = chosen }
                .testTag(tag),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = typography.body.copy(fontWeight = typography.weightBold),
            color = if (chosen) colors.selectedFg else colors.textPrimary,
        )
    }
}

/**
 * "FR · 中文 · EN" of the selected tile (UX audit R5): the language that person's screens use after the login. The login
 * screen itself stays in the terminal language. Test tags: `login_user_lang_<code>`.
 */
@Composable
private fun UserLanguageChoice(
    current: AppLanguage,
    onLanguage: (AppLanguage) -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        LanguageChoices.forEach { language ->
            val chosen = language == current
            Box(
                modifier =
                    Modifier
                        .height(LanguageHeight)
                        .defaultMinSize(minWidth = LanguageMinWidth)
                        .background(if (chosen) colors.selectedBg else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                        .clickable(role = Role.RadioButton) { onLanguage(language) }
                        .semantics { selected = chosen }
                        .testTag("login_user_lang_${language.code}"),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = vueText("lang_short_${language.code}"),
                    style = typography.body.copy(fontWeight = typography.weightBold),
                    color = if (chosen) colors.selectedFg else colors.textPrimary,
                    modifier = Modifier.padding(horizontal = PiliTheme.dimens.spaceMd),
                )
            }
        }
    }
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
        text = vueText(if (onShift) "login_action" else "login"),
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
                text = vueText("end_shift_action"),
                onClick = callbacks.onEndShift,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.height(ActionHeight).testTag("login_end_shift"),
            )
        }
    }
}
