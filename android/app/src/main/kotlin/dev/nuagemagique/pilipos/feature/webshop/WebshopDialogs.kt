package dev.nuagemagique.pilipos.feature.webshop

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.ConfirmDialog
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * The dialogs of the board, drawn in the overlay of the scaffold so they sit inside the window above the header
 * (PD214): the refusal, the refund by the manager and the minutes of a delay. A dialog whose order left the list is
 * not drawn. Test tags: those of `ConfirmDialog` (`confirm_ok`, `confirm_cancel`), `webshop_delay_<minutes>`,
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
        is WebshopDialog.RejectConfirm ->
            ConfirmDialog(
                title = vueText("webshop_action_reject"),
                message = uiText(R.string.ui_webshop_reject_message),
                confirmText = vueText("webshop_action_reject"),
                cancelText = vueText("back"),
                onConfirm = callbacks.onConfirmReject,
                onCancel = callbacks.onDismissDialog,
                destructive = true,
            )
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
