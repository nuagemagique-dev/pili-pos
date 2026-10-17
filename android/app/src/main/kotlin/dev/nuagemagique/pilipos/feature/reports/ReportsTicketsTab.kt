package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.CorrectionDone

private val RowMinHeight = 72.dp
private val NumberWidth = 120.dp
private val KindWidth = 200.dp
private val TableWidth = 90.dp
private val TimeWidth = 110.dp
private val TotalWidth = 160.dp
private val CopyWidth = 160.dp
private val RefundWidth = 200.dp
private val CorrectWidth = 220.dp

/**
 * The tickets tab (`tickets` section of `ReportsView.vue`): one list of recent tickets, newest first, each with a
 * copy button (when it can be copied) and a refund button (when it can be refunded). The amounts are the engine's
 * own, only formatted. Test tags: `tickets_title`, `tickets_hint`, `tickets_empty`, `ticket_list`,
 * `ticket_row_<id>`, `ticket_number_<id>`, `ticket_kind_<id>`, `ticket_table_<id>`, `ticket_time_<id>`,
 * `ticket_total_<id>`, `ticket_copy_<id>`, `ticket_refund_<id>`, `ticket_correct_<id>` and `ticket_corrected_<id>`
 * (a manager only).
 */
@Composable
internal fun TicketsTab(
    tickets: TicketsState,
    callbacks: TicketsCallbacks,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Panel(modifier.fillMaxSize()) {
        Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
            Text(vueText("reports_tickets_title"), style = t.title, color = c.textPrimary, modifier = Modifier.testTag("tickets_title"))
            Text(vueText("reports_tickets_hint"), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("tickets_hint"))
        }
        if (tickets.loaded && tickets.rows.isEmpty()) {
            Text(vueText("copy_none"), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("tickets_empty"))
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize().testTag("ticket_list"),
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
            ) {
                items(tickets.rows, key = { it.fiscalEventId }) { row -> TicketRowView(row, tickets, callbacks) }
            }
        }
    }
}

/** The caption of the kind of a ticket (`copyKind` of `ReportsView.vue`). */
@Composable
internal fun kindText(kind: TicketKind): String =
    when (kind) {
        TicketKind.TRAINING -> vueText("copy_kind_training")
        TicketKind.PRE_BILL -> vueText("copy_kind_pre_bill")
        TicketKind.REFUND -> vueText("copy_kind_refund")
        TicketKind.SALE -> vueText("copy_kind_sale")
    }

@Composable
private fun TicketRowView(
    row: TicketRow,
    tickets: TicketsState,
    callbacks: TicketsCallbacks,
) {
    val d = PiliTheme.dimens
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = RowMinHeight)
                .background(PiliTheme.colors.bgSurface, RoundedCornerShape(d.radiusMd))
                .padding(horizontal = d.spaceMd, vertical = d.spaceSm)
                .testTag("ticket_row_${row.fiscalEventId}"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TicketFacts(row)
        Box(Modifier.weight(1f))
        TicketActions(row, tickets, callbacks)
    }
}

/** What the ticket is: its number, kind, table, time and amount. */
@Composable
private fun TicketFacts(row: TicketRow) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val id = row.fiscalEventId
    Text(
        "#${row.ticketNumber}",
        style = t.figure,
        color = c.textPrimary,
        modifier = Modifier.width(NumberWidth).testTag("ticket_number_$id"),
    )
    Text(kindText(row.kind), style = t.body, color = c.textPrimary, modifier = Modifier.width(KindWidth).testTag("ticket_kind_$id"))
    Text(
        text = row.tableNumber?.let { "T$it" } ?: "",
        style = t.figure,
        color = c.textSecondary,
        modifier = Modifier.width(TableWidth).testTag("ticket_table_$id"),
    )
    Text(
        text = PosDateTime.formatTimestamp(row.createdAt, PosDateTime::formatTime),
        style = t.figure,
        color = c.textSecondary,
        modifier = Modifier.width(TimeWidth).testTag("ticket_time_$id"),
    )
    Text(
        text = formatEuro(row.totalCents),
        style = t.figure.copy(fontWeight = t.weightBold),
        color = if (row.totalCents < 0L) c.accentDanger else c.textPrimary,
        textAlign = TextAlign.End,
        modifier = Modifier.width(TotalWidth).testTag("ticket_total_$id"),
    )
}

/** What can be done with the ticket: copy it when it is copyable, refund it when it is refundable. */
@Composable
private fun TicketActions(
    row: TicketRow,
    tickets: TicketsState,
    callbacks: TicketsCallbacks,
) {
    val id = row.fiscalEventId
    Box(Modifier.width(CopyWidth)) {
        if (row.copyable != null) {
            PiliButton(
                text = vueText("reports_copy"),
                onClick = { callbacks.onCopy(row.copyable.fiscalEventId) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.fillMaxWidth().testTag("ticket_copy_$id"),
            )
        }
    }
    Box(Modifier.width(RefundWidth)) {
        if (row.refundable != null) {
            PiliButton(
                text = vueText("refund_confirm"),
                onClick = { callbacks.onRefund(row.refundable) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.fillMaxWidth().testTag("ticket_refund_$id"),
            )
        }
    }
    if (tickets.canCorrect) CorrectionSlot(row, tickets.corrections[id], callbacks)
}

/** The payment correction of a sale: the button, or the mark of the correction it already has. */
@Composable
private fun CorrectionSlot(
    row: TicketRow,
    done: CorrectionDone?,
    callbacks: TicketsCallbacks,
) {
    val id = row.fiscalEventId
    Box(Modifier.width(CorrectWidth), contentAlignment = Alignment.CenterEnd) {
        if (done != null) {
            Text(
                correctionMark(done),
                style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
                color = PiliTheme.colors.textSecondary,
                textAlign = TextAlign.End,
                modifier = Modifier.testTag("ticket_corrected_$id"),
            )
        } else if (row.isSale) {
            PiliButton(
                text = vueText("reports_pc_action"),
                onClick = { callbacks.onCorrect(row) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.fillMaxWidth().testTag("ticket_correct_$id"),
            )
        }
    }
}
