package dev.nuagemagique.pilipos.feature.webshop

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.ConfirmDialog
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val REJECT_CHIP_HEIGHT = 64.dp
private const val REJECT_PER_ROW = 2

/**
 * The dialogs of the board, drawn in the overlay of the scaffold so they sit inside the window above the header
 * (PD214): the refusal with its reason, the refund by the manager and the minutes of a delay. A dialog whose order left the list is
 * not drawn. Test tags: `confirm_ok`, `confirm_cancel` (the refusal and the refund), `webshop_delay_<minutes>`,
 * `webshop_delay_cancel`.
 */
@Composable
internal fun WebshopDialogs(
    state: WebshopUiState,
    callbacks: WebshopCallbacks,
) {
    val dialog = state.dialog ?: return
    val order = state.orders.firstOrNull { it.id == dialog.orderId } ?: return
    when (dialog) {
        is WebshopDialog.RejectConfirm -> RejectDialog(dialog.reason, callbacks)
        is WebshopDialog.RefundConfirm ->
            ConfirmDialog(
                title = vueText("webshop_action_refund"),
                message = uiText(R.string.ui_webshop_refund_message, "amount" to formatEuro(order.totalCents), "name" to state.operator),
                confirmText = vueText("refund_confirm"),
                cancelText = vueText("back"),
                onConfirm = callbacks.onConfirmRefund,
                onCancel = callbacks.onDismissDialog,
                destructive = true,
            )
        is WebshopDialog.Delay -> DelayDialog(callbacks)
    }
}

/**
 * "Refuser" (UX audit O1): the reasons as chips (a tap picks one, a second tap picks another) and the red button,
 * enabled once a reason is picked. The reason stays on the till; HubRise has no reject reason field. Test tags:
 * `webshop_reject_reason_<out_of_stock|too_busy|closed|other>` and those of the confirm dialog (`confirm_ok`,
 * `confirm_cancel`).
 */
@Composable
private fun RejectDialog(
    picked: RejectReason?,
    callbacks: WebshopCallbacks,
) {
    val d = PiliTheme.dimens
    PiliDialog(
        title = vueText("webshop_reject_title"),
        onDismissRequest = callbacks.onDismissDialog,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onDismissDialog,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("confirm_cancel"),
            )
            PiliButton(
                text = vueText("webshop_reject_confirm"),
                onClick = callbacks.onConfirmReject,
                variant = PiliButtonVariant.Danger,
                enabled = picked != null,
                modifier = Modifier.testTag("confirm_ok"),
            )
        },
    ) {
        Text(vueText("webshop_reject_message"), style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
        Text(vueText("webshop_reject_reason_label"), style = PiliTheme.typography.label, color = PiliTheme.colors.textPrimary)
        Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            RejectReason.entries.chunked(REJECT_PER_ROW).forEach { row ->
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
                    row.forEach { reason ->
                        PiliButton(
                            text = vueText(reason.labelKey),
                            onClick = { callbacks.onPickRejectReason(reason) },
                            variant = if (reason == picked) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                            modifier =
                                Modifier.weight(
                                    1f,
                                ).height(REJECT_CHIP_HEIGHT).testTag("webshop_reject_reason_${reason.name.lowercase()}"),
                        )
                    }
                }
            }
        }
    }
}

/** The minutes to push the promised time forward by (+60 is two taps on +30). */
@Composable
private fun DelayDialog(callbacks: WebshopCallbacks) {
    PiliDialog(
        title = vueText("webshop_delay_label"),
        onDismissRequest = callbacks.onDismissDialog,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onDismissDialog,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("webshop_delay_cancel"),
            )
        },
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            DELAY_CHOICES.forEach { minutes ->
                PiliButton(
                    text = uiText(R.string.ui_webshop_delay_minutes, "minutes" to minutes),
                    onClick = { callbacks.onDelayMinutes(minutes) },
                    variant = PiliButtonVariant.Primary,
                    modifier = Modifier.testTag("webshop_delay_$minutes"),
                )
            }
        }
    }
}
