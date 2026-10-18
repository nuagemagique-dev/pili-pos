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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.InvoiceSummary

private val InvoiceRowMinHeight = 72.dp
private val InvoiceNoWidth = 180.dp
private val InvoiceDateWidth = 140.dp
private val InvoiceAmountWidth = 160.dp
private val InvoiceStateWidth = 240.dp
private val InvoiceCopyWidth = 160.dp

/**
 * The issued invoices (`InvoiceList.vue`): number, customer, date, total, the state of the Peppol delivery and the
 * Copie button (managers only). Drawn under the tickets list, only when there is an invoice. Test tags:
 * `invoice_list`, `invoice_row_<id>`, `invoice_no_<id>`, `invoice_customer_<id>`, `invoice_date_<id>`,
 * `invoice_amount_<id>`, `invoice_status_<id>`, `invoice_copy_<id>`.
 */
@Composable
internal fun InvoiceListPanel(
    state: InvoicesState,
    callbacks: InvoiceCallbacks,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Panel(modifier.testTag("invoice_list")) {
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceLg), verticalAlignment = Alignment.Bottom) {
            Text(vueText("invoices_title"), style = t.title, color = c.textPrimary)
            Text(vueText("invoices_hint"), style = t.body, color = c.textSecondary)
        }
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        ) {
            items(state.invoices, key = { it.id }) { invoice -> InvoiceRow(invoice, state, callbacks) }
        }
    }
}

@Composable
private fun InvoiceRow(
    invoice: InvoiceSummary,
    state: InvoicesState,
    callbacks: InvoiceCallbacks,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val id = invoice.id
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = InvoiceRowMinHeight)
                .background(c.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .padding(horizontal = PiliTheme.dimens.spaceMd, vertical = PiliTheme.dimens.spaceSm)
                .testTag("invoice_row_$id"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            invoice.invoiceNo,
            style = t.figure.copy(fontWeight = t.weightBold),
            color = c.textPrimary,
            modifier = Modifier.width(InvoiceNoWidth).testTag("invoice_no_$id"),
        )
        InvoiceCustomerCell(invoice, Modifier.weight(1f))
        Text(
            issueDay(invoice.issueDate),
            style = t.figure,
            color = c.textSecondary,
            modifier = Modifier.width(InvoiceDateWidth).testTag("invoice_date_$id"),
        )
        Text(
            formatEuro(invoice.totalCents),
            style = t.figure.copy(fontWeight = t.weightBold),
            color = c.textPrimary,
            textAlign = TextAlign.End,
            modifier = Modifier.width(InvoiceAmountWidth).testTag("invoice_amount_$id"),
        )
        Text(
            vueText(invoiceStatusKey(invoice.deliveryStatus)),
            style = t.label.copy(fontWeight = t.weightBold),
            color = statusColor(invoice.deliveryStatus),
            modifier = Modifier.width(InvoiceStateWidth).testTag("invoice_status_$id"),
        )
        Box(Modifier.width(InvoiceCopyWidth)) { InvoiceCopyButton(invoice, state, callbacks) }
    }
}

@Composable
private fun statusColor(status: String?): Color {
    val c = PiliTheme.colors
    return when (status) {
        "queued" -> c.accentWarning
        "accepted" -> c.accentSuccess
        "failed" -> c.accentDanger
        else -> c.textSecondary
    }
}

@Composable
private fun InvoiceCustomerCell(
    invoice: InvoiceSummary,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(modifier.semantics(mergeDescendants = true) {}.testTag("invoice_customer_${invoice.id}")) {
        Text(
            invoice.customerName,
            style = t.body.copy(fontWeight = t.weightBold),
            color = c.textPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        Text(invoice.customerVatNumber, style = t.label, color = c.textSecondary)
    }
}

@Composable
private fun InvoiceCopyButton(
    invoice: InvoiceSummary,
    state: InvoicesState,
    callbacks: InvoiceCallbacks,
) {
    if (!state.canCopy) return
    PiliButton(
        text = vueText("invoice_copy"),
        onClick = { callbacks.onCopy(invoice.fiscalEventId) },
        variant = PiliButtonVariant.Secondary,
        enabled = state.copying == null,
        modifier = Modifier.fillMaxWidth().testTag("invoice_copy_${invoice.id}"),
    )
}
