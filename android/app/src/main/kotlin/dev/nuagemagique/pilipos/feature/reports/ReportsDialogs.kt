package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.AmountDialog
import dev.nuagemagique.pilipos.ui.components.NumberKeypad
import dev.nuagemagique.pilipos.ui.components.NumberKeypadSpec
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.defaultAmountDialogTexts
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.ZReportSummary

private val ListMaxHeight = 560.dp
private val RowActionWidth = 200.dp

/**
 * The Z history (`z-list` of `ReportsView.vue`): every Z with a reprint button, which opens the copy flow of that
 * Z. A Z without a fiscal event has no button. Test tags: `z_list`, `z_list_empty`, `z_row_<id>`,
 * `z_label_<id>`, `z_reprint_<id>`, `z_list_close`.
 */
@Composable
internal fun ZListDialog(
    history: List<ZReportSummary>,
    callbacks: DayCallbacks,
) {
    PiliDialog(
        title = vueText("report_history_title"),
        onDismissRequest = callbacks.onCloseZList,
        modifier = Modifier.testTag("z_list"),
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onCloseZList,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("z_list_close"),
            )
        },
    ) {
        if (history.isEmpty()) {
            Text(
                vueText("report_no_history"),
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.textSecondary,
                modifier = Modifier.testTag("z_list_empty"),
            )
        } else {
            Column(
                modifier = Modifier.fillMaxWidth().heightIn(max = ListMaxHeight).verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
            ) {
                history.forEach { z -> ZRow(z, callbacks.onReprintZ) }
            }
        }
    }
}

@Composable
private fun ZRow(
    z: ZReportSummary,
    onReprint: (Long?) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().testTag("z_row_${z.id}"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "Z #${z.sequenceNumber} · ${dateOf(z.createdAt)}",
            style = PiliTheme.typography.body,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.weight(1f).testTag("z_label_${z.id}"),
        )
        if (z.fiscalEventId != null) {
            PiliButton(
                text = vueText("report_reprint"),
                onClick = { onReprint(z.fiscalEventId) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.width(RowActionWidth).testTag("z_reprint_${z.id}"),
            )
        }
    }
}

/**
 * The two amount keypads of the cash tab: "Other" for the amount of the form and the drawer count (0 allowed, an
 * empty drawer). The amount is read as cents from the keypad and sent as typed. Test tags: those of
 * `AmountDialog` (`amount_dialog`, `amount_value`, `amount_confirm`, `amount_cancel`).
 */
@Composable
internal fun CashDialogs(
    cash: CashState,
    callbacks: CashCallbacks,
) {
    if (cash.declarationOpen) {
        AmountDialog(
            title = vueText("cash_declaration"),
            itemName = vueText("cash_declaration_hint"),
            texts = defaultAmountDialogTexts(),
            onConfirm = callbacks.onDeclare,
            onCancel = callbacks.onCloseDeclaration,
            allowZero = true,
        )
    }
    if (cash.otherAmountOpen) {
        AmountDialog(
            title = vueText("reports_other_amount"),
            itemName = cashSummaryText(cash),
            texts = defaultAmountDialogTexts(),
            onConfirm = callbacks.onPickOtherAmount,
            onCancel = callbacks.onCloseOtherAmount,
        )
    }
}

/**
 * The pad of the ticket number field (`NumberEntryDialog` behind `ticket-find`, UX audit R2): a whole number from 1,
 * up to [TICKET_NUMBER_MAX_DIGITS] digits. Test tags: `ticket_find_dialog`, `ticket_find_value`, `ticket_find_ok`,
 * `ticket_find_cancel`.
 */
@Composable
internal fun TicketNumberDialog(
    initial: Long?,
    callbacks: TicketsCallbacks,
) {
    NumberKeypad(
        spec =
            NumberKeypadSpec(
                tag = "ticket_find",
                title = vueText("tickets_find_label"),
                initial = initial?.toString().orEmpty(),
                maxDigits = TICKET_NUMBER_MAX_DIGITS,
                min = 1L,
            ),
        onConfirm = { callbacks.onFind(it.toLongOrNull() ?: 0L) },
        onCancel = callbacks.onCloseFind,
    )
}
