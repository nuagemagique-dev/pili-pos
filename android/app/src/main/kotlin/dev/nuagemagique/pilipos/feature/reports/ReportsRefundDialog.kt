package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
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
import androidx.compose.ui.semantics.semantics
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.DeliveryOutcomePanel
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.TicketMediumKeypad
import dev.nuagemagique.pilipos.ui.components.TicketMediumPicker
import dev.nuagemagique.pilipos.ui.components.TicketMediumPickerState
import dev.nuagemagique.pilipos.ui.components.VatSummary
import dev.nuagemagique.pilipos.ui.components.defaultDeliveryOutcomeTexts
import dev.nuagemagique.pilipos.ui.components.defaultEmailKeypadTexts
import dev.nuagemagique.pilipos.ui.components.defaultTicketMediumPickerTexts
import dev.nuagemagique.pilipos.ui.components.rememberTicketMediumPickerState
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.format.formatWeight
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.RefundableEvent

/**
 * The refund dialog, step by step: the sale to refund (only when the flow starts from the list), the lines to give
 * back with their quantity, the delivery choice with the confirmation (selected lines, VAT summary and the preview
 * total, UI-PD307), the signature in progress, and the result. Test tags: `refund_dialog`, `refund_event_<id>`,
 * `refund_line_<id>`, `refund_orig_<id>`, `refund_amount_<id>`, `refund_qty_<id>`, `refund_minus_<id>`,
 * `refund_plus_<id>`, `refund_vat`, `refund_next`, `refund_close`, `refund_back`, `refund_confirm`, `refund_total`,
 * `refund_total_unavailable`, `refund_recap_<id>`, `refund_processing`, `refund_error`, `refund_done`.
 */
@Composable
internal fun RefundDialog(
    flow: RefundFlow,
    refundable: List<RefundableEvent>,
    delivery: ReportsDelivery,
    refund: RefundUi,
    callbacks: ReportsCallbacks,
) {
    val picker = rememberTicketMediumPickerState()
    Box(Modifier.fillMaxSize().testTag("refund_dialog")) {
        when (flow.step) {
            RefundStep.CHOOSE_EVENT -> ChooseEventStep(refundable, callbacks.tickets)
            RefundStep.CHOOSE_LINES -> LinesStep(flow, refund, callbacks.tickets)
            RefundStep.DELIVERY -> DeliveryStep(refund, delivery, picker, callbacks)
            RefundStep.PROCESSING -> ProcessingDialog("refund_processing")
            RefundStep.RESULT -> ResultStep(flow, delivery, callbacks)
        }
        TicketMediumKeypad(picker, defaultEmailKeypadTexts(), callbacks.delivery.onEmail, callbacks.delivery.onPaper)
    }
}

@Composable
private fun ChooseEventStep(
    refundable: List<RefundableEvent>,
    callbacks: TicketsCallbacks,
) {
    PiliDialog(
        title = vueText("refund_select_items"),
        onDismissRequest = callbacks.onRefundClose,
        maxWidth = FlowMaxWidth,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onRefundClose,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.width(ActionWidth).testTag("refund_close"),
            )
        },
    ) {
        if (refundable.isEmpty()) {
            Text(vueText("copy_none"), style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
        }
        Column(
            modifier = Modifier.fillMaxWidth().heightIn(max = FlowListMaxHeight).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        ) {
            refundable.forEach { event -> EventRow(event, callbacks.onRefund) }
        }
    }
}

@Composable
private fun EventRow(
    event: RefundableEvent,
    onRefund: (RefundableEvent) -> Unit,
) {
    val t = PiliTheme.typography
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "#${event.ticketNumber} · " + PosDateTime.formatTimestamp(event.createdAt, PosDateTime::formatTime),
            style = t.figure,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.weight(1f),
        )
        Text(formatEuro(event.totalCents), style = t.figure.copy(fontWeight = t.weightBold), color = PiliTheme.colors.textPrimary)
        PiliButton(
            text = vueText("refund_confirm"),
            onClick = { onRefund(event) },
            variant = PiliButtonVariant.Secondary,
            modifier = Modifier.width(ActionWidth).testTag("refund_event_${event.fiscalEventId}"),
        )
    }
}

/** The caption of the refund button: the Vue text and, when the engine gave one, the cash to pay back. */
@Composable
private fun refundLabel(preview: RefundPreview): String {
    val cash = preview.cashCents ?: return vueText("refund_confirm")
    return "${vueText("refund_confirm")} ${formatEuro(cash)}"
}

@Composable
private fun LinesStep(
    flow: RefundFlow,
    refund: RefundUi,
    callbacks: TicketsCallbacks,
) {
    val ticket = flow.event?.let { " · #${it.ticketNumber}" } ?: ""
    PiliDialog(
        title = vueText("refund_select_items") + ticket,
        onDismissRequest = callbacks.onRefundClose,
        maxWidth = FlowMaxWidth,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onRefundClose,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.width(ActionWidth).testTag("refund_close"),
            )
            PiliButton(
                text = refundLabel(refund.preview),
                onClick = callbacks.onRefundNext,
                enabled = flow.hasSelection,
                modifier = Modifier.testTag("refund_next"),
            )
        },
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().heightIn(max = FlowListMaxHeight).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        ) {
            flow.lines.forEach { line ->
                RefundLineRow(
                    // A weighing is refunded whole: its weight tells it from another line of the same product.
                    name = line.weightGrams?.let { "${line.name} (${formatWeight(it)})" } ?: line.name,
                    lineId = line.id,
                    quantity = line.quantity,
                    ticked = flow.checked[line.id],
                    amount = refund.amounts[line.id],
                    callbacks = callbacks,
                )
            }
        }
        VatSummary(refund.preview.vatRows, Modifier.testTag("refund_vat"))
    }
}

@Composable
private fun DeliveryStep(
    refund: RefundUi,
    delivery: ReportsDelivery,
    picker: TicketMediumPickerState,
    callbacks: ReportsCallbacks,
) {
    PiliDialog(
        title = vueText("refund_confirm"),
        onDismissRequest = callbacks.tickets.onRefundBack,
        maxWidth = FlowMaxWidth,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.tickets.onRefundBack,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.width(ActionWidth).testTag("refund_back"),
            )
            PiliButton(
                text = refundLabel(refund.preview),
                onClick = callbacks.tickets.onRefundConfirm,
                modifier = Modifier.testTag("refund_confirm"),
            )
        },
    ) {
        Recap(refund.preview)
        TicketMediumPicker(
            mailStatus = delivery.mailStatus,
            request = delivery.request,
            state = picker,
            onPaper = callbacks.delivery.onPaper,
            texts = defaultTicketMediumPickerTexts(),
        )
    }
}

/** The confirmation of the refund (UI-PD307): the ticked lines, their VAT summary and the cash to pay back. */
@Composable
private fun Recap(preview: RefundPreview) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(
        modifier = Modifier.fillMaxWidth().heightIn(max = FlowListMaxHeight).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
    ) {
        preview.lines.forEach { line ->
            val id = line.selection.line.id
            Row(
                modifier = Modifier.fillMaxWidth().semantics(mergeDescendants = true) {}.testTag("refund_recap_$id"),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    text = "${line.selection.quantity} × ${line.selection.line.name}",
                    style = t.body,
                    color = c.textPrimary,
                    modifier = Modifier.weight(1f),
                )
                Text(amountText(line.amount), style = t.figure, color = c.textPrimary)
            }
        }
    }
    VatSummary(preview.vatRows, Modifier.testTag("refund_vat"))
    val cash = preview.cashCents
    if (cash != null) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(uiText(R.string.ui_reports_refund_cash_total), style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
            Text(
                text = formatEuro(cash),
                style = t.figure.copy(fontWeight = t.weightBold),
                color = c.textPrimary,
                modifier = Modifier.testTag("refund_total"),
            )
        }
    } else if (preview.totalUnavailable) {
        Text(
            text = uiText(R.string.ui_reports_total_unavailable),
            style = t.body,
            color = c.textSecondary,
            modifier = Modifier.testTag("refund_total_unavailable"),
        )
    }
}

@Composable
private fun ResultStep(
    flow: RefundFlow,
    delivery: ReportsDelivery,
    callbacks: ReportsCallbacks,
) {
    val result = flow.result
    PiliDialog(
        title = vueText("refund_confirm"),
        onDismissRequest = callbacks.tickets.onRefundClose,
        maxWidth = FlowMaxWidth,
        actions = {
            PiliButton(
                text = vueText("finish"),
                onClick = callbacks.tickets.onRefundClose,
                modifier = Modifier.width(ActionWidth).testTag("refund_done"),
            )
        },
    ) {
        DeliveryOutcomePanel(
            outcome = result?.delivery,
            texts = defaultDeliveryOutcomeTexts(result?.delivery),
            canPrintMissing = delivery.canPrintMissing,
            onPrintMissing = callbacks.delivery.onPrintMissing,
            fiscalEventId = result?.fiscalEventId,
        )
        if (result != null && !result.success) ErrorLine(callbacks.tickets.describeError(result.error), "refund_error")
    }
}
