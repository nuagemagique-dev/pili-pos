package dev.nuagemagique.pilipos.feature.settings.alerts

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The widest the number dialog grows. */
private val DialogMaxWidth = 420.dp

/** What the number dialog calls. */
internal data class NumberEntryActions(
    val onKey: (NumPadKey) -> Unit = {},
    val onMinus: () -> Unit = {},
    val onConfirm: () -> Unit = {},
    val onCancel: () -> Unit = {},
)

/**
 * The number dialog of the alerts panel (`NumberEntryDialog.vue`): a Telegram chat id (optional minus sign, up
 * to 20 digits) or a small setting (hours, minutes) on the shared [NumPad]. Test tags: `number_entry`,
 * `number_entry_value`, `number_entry_minus`, `number_entry_ok`, `number_entry_cancel`, plus the pad keys.
 */
@Composable
internal fun NumberEntryDialog(
    entry: NumberEntry,
    actions: NumberEntryActions,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Box(modifier.fillMaxSize().testTag("number_entry")) {
        PiliDialog(
            title = vueText(entry.target.titleKey),
            onDismissRequest = actions.onCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = actions.onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("number_entry_cancel"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = actions.onConfirm,
                    enabled = entry.valid,
                    modifier = Modifier.testTag("number_entry_ok"),
                )
            },
        ) {
            Column(
                Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                Text(
                    text = entry.text.ifEmpty { "0" },
                    style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeXl),
                    color = colors.accentPrimary,
                    modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite }.testTag("number_entry_value"),
                )
                if (entry.target.allowMinus) MinusToggle(entry.minus, actions.onMinus)
                NumPad(
                    onKey = actions.onKey,
                    clearLabel = uiText(R.string.ui_common_numpad_clear),
                    backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
                )
            }
        }
    }
}

/** The minus toggle of a group chat id (`.minus`). */
@Composable
private fun MinusToggle(
    on: Boolean,
    onClick: () -> Unit,
) {
    val colors = PiliTheme.colors
    Box(
        Modifier
            .heightIn(min = 48.dp)
            .background(if (on) colors.selectedBg else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
            .clickable(role = Role.Switch, onClick = onClick)
            .padding(horizontal = PiliTheme.dimens.spaceMd)
            .testTag("number_entry_minus"),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            vueText("alerts_minus"),
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = if (on) colors.selectedFg else colors.textPrimary,
        )
    }
}
