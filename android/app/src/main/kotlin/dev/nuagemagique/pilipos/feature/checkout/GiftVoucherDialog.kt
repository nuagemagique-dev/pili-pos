package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.GIFT_VOUCHER_CODE_LENGTH
import dev.nuagemagique.pilipos.ui.components.GiftVoucherSummary
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.spacedVoucherCode
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val DialogMaxWidth = 440.dp

/**
 * Pay with a gift voucher (`GiftVoucherDialog.vue`): the 12-digit code typed on the pad, "Chercher" asks the engine
 * (5 failed tries a minute, then it makes the operator wait), and the screen shows the balance, the expiry and the
 * status. The amount used is the smaller of the balance and [restCents]. Test tags: `gv_dialog`, `gv_code`,
 * `gv_error`, `gv_found`, `gv_status`, `gv_balance`, `gv_amount`, `gv_lookup`, `gv_apply`, `gv_cancel`.
 */
@Composable
internal fun GiftVoucherDialog(
    state: VoucherDialogState,
    restCents: Long,
    callbacks: CheckoutCallbacks,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val found = state.found
    Box(Modifier.fillMaxSize().testTag("gv_dialog")) {
        PiliDialog(
            title = vueText("gv_dialog_title"),
            onDismissRequest = callbacks.onVoucherCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = callbacks.onVoucherCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("gv_cancel"),
                )
                if (found == null || found.status != "active") {
                    PiliButton(
                        text = vueText("gv_lookup"),
                        onClick = callbacks.onVoucherLookup,
                        enabled = state.digits.length == GIFT_VOUCHER_CODE_LENGTH && !state.busy,
                        modifier = Modifier.testTag("gv_lookup"),
                    )
                } else {
                    PiliButton(
                        text = vueText("gv_apply"),
                        onClick = callbacks.onVoucherApply,
                        enabled = minOf(found.balanceCents, restCents) > 0L,
                        modifier = Modifier.testTag("gv_apply"),
                    )
                }
            },
        ) {
            Column(
                modifier = Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                Text(vueText("gv_code_label"), style = t.body, color = c.textSecondary)
                Text(
                    text = spacedVoucherCode(state.digits).ifEmpty { " " },
                    style = t.figure.copy(fontSize = t.sizeXl),
                    color = c.accentPrimary,
                    modifier = Modifier.testTag("gv_code"),
                )
                NumPad(
                    onKey = callbacks.onVoucherKey,
                    clearLabel = uiText(R.string.ui_common_numpad_clear),
                    backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
                )
                state.error?.let {
                    Text(it, style = t.body, color = c.accentDanger, textAlign = TextAlign.Center, modifier = Modifier.testTag("gv_error"))
                }
                if (found != null) GiftVoucherSummary(found, useCents = minOf(found.balanceCents, restCents))
            }
        }
    }
}
