package dev.nuagemagique.pilipos.feature.settings.vouchers

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import dev.nuagemagique.pilipos.feature.settings.ActionButton
import dev.nuagemagique.pilipos.feature.settings.Panel
import dev.nuagemagique.pilipos.feature.settings.PanelText
import dev.nuagemagique.pilipos.feature.settings.PanelTitle
import dev.nuagemagique.pilipos.feature.settings.SectionLabel
import dev.nuagemagique.pilipos.feature.settings.SettingsTextField
import dev.nuagemagique.pilipos.feature.settings.alerts.SwitchRow
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * The gift voucher panel of the settings screen (`GiftVoucherSettings.vue`, "Bons cadeaux"): a switch, the validity
 * in months (12 to 36) and the largest voucher sold, in euros. Test tags: `gv_settings`, `gv_enabled`,
 * `gv_validity`, `gv_max`, `gv_settings_save`, `gv_settings_saved`, `gv_settings_error`.
 */
@Composable
internal fun GiftVoucherSection(
    state: GiftVoucherUiState,
    callbacks: GiftVoucherCallbacks,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Panel(modifier = modifier.testTag("gv_settings")) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            PanelTitle(vueText("gv_settings_title"), Modifier.weight(1f))
            if (state.stored != null) {
                SwitchRow(
                    on = state.enabled,
                    text = vueText(if (state.enabled) "gv_settings_on" else "gv_settings_off"),
                    onClick = callbacks.onToggle,
                    tag = "gv_enabled",
                    fill = false,
                )
            }
        }
        state.error?.let {
            Text(it, style = PiliTheme.typography.body, color = colors.accentDanger, modifier = Modifier.testTag("gv_settings_error"))
        }
        PanelText(vueText("gv_settings_note"))
        if (state.stored != null) {
            VoucherFields(state, callbacks)
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                ActionButton(
                    text = vueText("gv_settings_save"),
                    onClick = callbacks.onSave,
                    enabled = !state.busy && state.dirty,
                    variant = PiliButtonVariant.Primary,
                    modifier = Modifier.testTag("gv_settings_save"),
                )
                if (state.saved && !state.dirty) {
                    Text(
                        text = vueText("gv_settings_saved"),
                        style = PiliTheme.typography.body,
                        color = colors.accentSuccess,
                        modifier = Modifier.testTag("gv_settings_saved"),
                    )
                }
            }
        }
    }
}

@Composable
private fun VoucherFields(
    state: GiftVoucherUiState,
    callbacks: GiftVoucherCallbacks,
) {
    SectionLabel(vueText("gv_settings_validity"))
    SettingsTextField(
        value = state.validity,
        onValueChange = callbacks.onValidity,
        placeholder = "12",
        modifier = Modifier.testTag("gv_validity"),
        mono = true,
        numeric = true,
    )
    SectionLabel(vueText("gv_settings_max"))
    SettingsTextField(
        value = state.maxEuros,
        onValueChange = callbacks.onMax,
        placeholder = "500",
        modifier = Modifier.testTag("gv_max"),
        mono = true,
        numeric = true,
    )
}
