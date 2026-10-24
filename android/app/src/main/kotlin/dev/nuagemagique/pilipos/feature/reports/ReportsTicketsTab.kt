package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.CorrectionDone
import uniffi.pos_ffi.InvoiceableSale

private val RowMinHeight = 72.dp
private val CheckSize = 48.dp
private val CheckColumn = 56.dp
private val CheckBorder = 2.dp
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
    invoices: InvoicesState = InvoicesState(),
    invoiceCallbacks: InvoiceCallbacks = InvoiceCallbacks(),
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val rows = ticketRowsWith(tickets.rows, invoices.sales)
    val sales = invoices.sales.associateBy { it.fiscalEventId }
    Column(modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceLg)) {
        Panel(Modifier.weight(TICKETS_WEIGHT).fillMaxWidth()) {
            Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
                    Text(
                        vueText("reports_tickets_title"),
                        style = t.title,
                        color = c.textPrimary,
                        modifier = Modifier.testTag("tickets_title"),
                    )
                    Text(
                        vueText("reports_tickets_hint") + " " + vueText("invoice_select_hint"),
                        style = t.body,
                        color = c.textSecondary,
                        modifier = Modifier.testTag("tickets_hint"),
                    )
                }
                PiliButton(
                    text =
                        if (invoices.selected.isEmpty()) {
                            vueText("invoice_action")
                        } else {
                            vueText("invoice_action_count", "n" to invoices.selected.size)
                        },
                    onClick = invoiceCallbacks.onOpen,
                    enabled = invoices.selected.isNotEmpty(),
                    modifier = Modifier.width(InvoiceActionWidth).testTag("invoice_action"),
                )
            }
            if (tickets.loaded && rows.isEmpty()) {
                Text(vueText("copy_none"), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("tickets_empty"))
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize().testTag("ticket_list"),
                    verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
                ) {
                    items(rows, key = { it.fiscalEventId }) { row ->
                        TicketRowView(
                            row,
                            tickets,
                            sales[row.fiscalEventId],
                            row.fiscalEventId in invoices.selected,
                            callbacks,
                            invoiceCallbacks,
                        )
                    }
                }
            }
        }
        if (invoices.invoices.isNotEmpty()) {
            InvoiceListPanel(invoices, invoiceCallbacks, Modifier.weight(INVOICES_WEIGHT).fillMaxWidth())
        }
    }
}

private const val TICKETS_WEIGHT = 3f
private const val INVOICES_WEIGHT = 2f
private val InvoiceActionWidth = 240.dp

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
    sale: InvoiceableSale?,
    ticked: Boolean,
    callbacks: TicketsCallbacks,
    invoiceCallbacks: InvoiceCallbacks,
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
        TicketCheck(row.fiscalEventId, sale, ticked, invoiceCallbacks)
        TicketFacts(row, sale)
        Box(Modifier.weight(1f))
        TicketActions(row, tickets, callbacks)
    }
}

/** The box that ticks a sale for an invoice; none for a ticket that cannot be invoiced or already is. */
@Composable
private fun TicketCheck(
    fiscalEventId: Long,
    sale: InvoiceableSale?,
    ticked: Boolean,
    callbacks: InvoiceCallbacks,
) {
    val c = PiliTheme.colors
    Box(Modifier.width(CheckColumn), contentAlignment = Alignment.Center) {
        if (sale != null && sale.invoicedBy == null) {
            Box(
                modifier =
                    Modifier
                        .size(CheckSize)
                        .background(if (ticked) c.selectedBg else c.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                        .border(CheckBorder, if (ticked) c.selectedBg else c.textSecondary, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                        .toggleable(value = ticked, role = Role.Checkbox, onValueChange = { callbacks.onToggleSale(fiscalEventId) })
                        .testTag("ticket_check_$fiscalEventId"),
                contentAlignment = Alignment.Center,
            ) {
                if (ticked) Text("\u2713", style = PiliTheme.typography.title, color = c.selectedFg)
            }
        }
    }
}

/** What the ticket is: its number, kind (and the invoice that covers it), table, time and amount. */
@Composable
private fun TicketFacts(
    row: TicketRow,
    sale: InvoiceableSale?,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val id = row.fiscalEventId
    Text(
        "#${row.ticketNumber}",
        style = t.figure,
        color = c.textPrimary,
        modifier = Modifier.width(NumberWidth).testTag("ticket_number_$id"),
    )
    Column(Modifier.width(KindWidth)) {
        Text(kindText(row.kind), style = t.body, color = c.textPrimary, modifier = Modifier.testTag("ticket_kind_$id"))
        sale?.invoicedBy?.let {
            Text(
                vueText("invoice_invoiced_by", "no" to it),
                style = t.label,
                color = c.accentInfo,
                modifier = Modifier.testTag("ticket_invoiced_$id"),
            )
        }
    }
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
