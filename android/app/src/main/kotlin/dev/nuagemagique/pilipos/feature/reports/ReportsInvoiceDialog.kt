@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.Customer
import uniffi.pos_ffi.InvoicePreview
import uniffi.pos_ffi.InvoiceableSale

private val InvoiceMaxWidth = 1500.dp
private val InvoiceBodyMaxHeight = 640.dp
private val PreviewListMaxHeight = 300.dp
private val ResultsMaxHeight = 232.dp
private val ResultRowHeight = 56.dp
private val ChipPadding = 14.dp
private val QuantityWidth = 64.dp
private val VatLabelWidth = 40.dp
private val AmountWidth = 120.dp
private val SmallActionWidth = 240.dp
private const val CHIP_SHAPE_PERCENT = 50

/**
 * The invoice dialog (`InvoiceDialog.vue`): the customer step, the preview, the signature in progress and the
 * result. Nothing is signed before the confirm button of the preview. A tap on the scrim leaves the customer step
 * only (the other steps have their own buttons). Test tags: `invoice_dialog`, plus those of each step.
 */
@Composable
internal fun InvoiceDialog(
    flow: InvoiceFlow,
    sales: List<InvoiceableSale>,
    callbacks: InvoiceCallbacks,
) {
    Box(Modifier.fillMaxSize().testTag("invoice_dialog")) {
        when (flow.step) {
            InvoiceStep.CUSTOMER -> CustomerStep(flow, sales, callbacks)
            InvoiceStep.PREVIEW -> PreviewStep(flow, sales, callbacks)
            InvoiceStep.PROCESSING -> ProcessingDialog("invoice_processing")
            InvoiceStep.RESULT -> ResultStep(flow, callbacks)
        }
    }
}

/** The three steps and the sales on the invoice (`inv-head` of `InvoiceDialog.vue`). */
@Composable
private fun InvoiceHead(
    flow: InvoiceFlow,
    sales: List<InvoiceableSale>,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val chosen = sales.filter { it.fiscalEventId in flow.saleEventIds }.map { it.totalCents }
    val index =
        when (flow.step) {
            InvoiceStep.CUSTOMER -> 0
            InvoiceStep.PREVIEW, InvoiceStep.PROCESSING -> 1
            InvoiceStep.RESULT -> 2
        }
    Row(
        modifier = Modifier.fillMaxWidth().testTag("invoice_steps"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        listOf("invoice_step_customer", "invoice_step_preview", "invoice_step_done").forEachIndexed { i, key ->
            val on = i == index
            Box(
                Modifier
                    .background(if (on) c.selectedBg else c.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                    .padding(horizontal = PiliTheme.dimens.spaceMd, vertical = PiliTheme.dimens.spaceSm)
                    .testTag(if (on) "invoice_step_on" else "invoice_step_off"),
            ) {
                Text(vueText(key), style = t.body.copy(fontWeight = t.weightBold), color = if (on) c.selectedFg else c.textSecondary)
            }
        }
        Box(Modifier.weight(1f))
        Text(
            vueText("invoice_sales_chosen", "n" to flow.saleEventIds.size, "total" to formatCents(CashDisplay.sumCents(chosen))),
            style = t.figure,
            color = c.textSecondary,
            modifier = Modifier.testTag("invoice_sales"),
        )
    }
}

// ── Customer step ──────────────────────────────────────────────────

@Composable
private fun CustomerStep(
    flow: InvoiceFlow,
    sales: List<InvoiceableSale>,
    callbacks: InvoiceCallbacks,
) {
    PiliDialog(
        title = vueText("invoice_title"),
        onDismissRequest = callbacks.onClose,
        maxWidth = InvoiceMaxWidth,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onClose,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.width(SmallActionWidth).testTag("invoice_cancel"),
            )
            PiliButton(
                text = vueText("invoice_continue"),
                onClick = callbacks.onContinue,
                enabled = flow.canContinue,
                modifier = Modifier.width(ActionWidth).testTag("invoice_continue"),
            )
        },
    ) {
        InvoiceHead(flow, sales)
        Column(
            modifier = Modifier.heightIn(max = InvoiceBodyMaxHeight).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        ) {
            SearchBlock(flow, callbacks)
            VatBlock(flow, callbacks)
            AddressBlock(flow.form, callbacks)
            flow.customerError?.let { ErrorLine(it, "invoice_error") }
        }
    }
}

@Composable
private fun SearchBlock(
    flow: InvoiceFlow,
    callbacks: InvoiceCallbacks,
) {
    val c = PiliTheme.colors
    Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        FieldLabel("invoice_search_label")
        InvoiceTextField(flow.search, callbacks.onSearch, vueText("invoice_search_placeholder"), "invoice_search_field")
        if (flow.searchResults.isNotEmpty()) {
            Column(
                modifier =
                    Modifier.heightIn(
                        max = ResultsMaxHeight,
                    ).verticalScroll(rememberScrollState()).testTag("invoice_search_results"),
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
            ) {
                flow.searchResults.forEach { SearchResult(it, callbacks) }
            }
        } else if (flow.search.isNotBlank()) {
            Text(
                vueText("invoice_no_match"),
                style = PiliTheme.typography.label,
                color = c.textSecondary,
                modifier = Modifier.testTag("invoice_no_match"),
            )
        }
    }
}

@Composable
private fun SearchResult(
    customer: Customer,
    callbacks: InvoiceCallbacks,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = ResultRowHeight)
                .background(c.bgPrimary, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(role = Role.Button) { callbacks.onPickCustomer(customer) }
                .padding(horizontal = PiliTheme.dimens.spaceMd)
                .testTag("invoice_search_result_${customer.id}"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(customer.name, style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
        Text(customer.vatNumber, style = t.figure, color = c.textSecondary)
    }
}

@Composable
private fun VatBlock(
    flow: InvoiceFlow,
    callbacks: InvoiceCallbacks,
) {
    Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        FieldLabel("invoice_vat_label")
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm), verticalAlignment = Alignment.CenterVertically) {
            InvoiceTextField(
                value = flow.form.vatNumber,
                onValueChange = callbacks.onVatNumber,
                placeholder = "BE0123456789",
                tag = "invoice_vat_field",
                modifier = Modifier.weight(1f),
                mono = true,
            )
            PiliButton(
                text = vueText(if (flow.lookupBusy) "invoice_vat_checking" else "invoice_vat_check"),
                onClick = callbacks.onCheckVat,
                variant = PiliButtonVariant.Secondary,
                enabled = !flow.lookupBusy && flow.form.vatNumber.isNotBlank(),
                modifier = Modifier.width(ActionWidth).testTag("invoice_vat_check"),
            )
            if (flow.lookup != null) PeppolChip(flow.peppolStatus)
        }
        peppolNote(flow)?.let {
            Text(
                it,
                style = PiliTheme.typography.label,
                color = PiliTheme.colors.textSecondary,
                modifier = Modifier.testTag("invoice_chip_note"),
            )
        }
    }
}

/** The explanation under the chip, or null (`chipNote` of `InvoiceDialog.vue`). */
@Composable
private fun peppolNote(flow: InvoiceFlow): String? {
    val lookup = flow.lookup
    return when {
        lookup != null && !lookup.deliverable -> vueText("invoice_foreign_note")
        flow.peppolStatus == PEPPOL_NOT_REGISTERED -> vueText("invoice_peppol_not_registered_note")
        flow.peppolStatus == PEPPOL_UNKNOWN && lookup != null -> vueText("invoice_peppol_unknown_note")
        else -> null
    }
}

/** The Peppol chip: registered, not registered or unknown (offline). Test tag `invoice_chip`. */
@Composable
internal fun PeppolChip(status: String) {
    val c = PiliTheme.colors
    val fill: Color =
        when (status) {
            PEPPOL_REGISTERED -> c.accentSuccess
            PEPPOL_NOT_REGISTERED -> c.accentDanger
            else -> c.accentWarning
        }
    val key =
        when (status) {
            PEPPOL_REGISTERED -> "invoice_peppol_registered"
            PEPPOL_NOT_REGISTERED -> "invoice_peppol_not_registered"
            else -> "invoice_peppol_unknown"
        }
    Box(
        Modifier
            .background(fill, RoundedCornerShape(CHIP_SHAPE_PERCENT))
            .padding(horizontal = ChipPadding, vertical = PiliTheme.dimens.spaceSm)
            .semantics(mergeDescendants = true) {}
            .testTag("invoice_chip"),
    ) {
        Text(
            vueText(key),
            style = PiliTheme.typography.label.copy(fontWeight = PiliTheme.typography.weightBold),
            color = c.textOnAccent,
            maxLines = 1,
        )
    }
}

@Composable
private fun AddressBlock(
    form: CustomerForm,
    callbacks: InvoiceCallbacks,
) {
    val gap = PiliTheme.dimens.spaceMd
    Column(verticalArrangement = Arrangement.spacedBy(gap)) {
        LabeledField("invoice_field_name", form.name, callbacks.onName, "invoice_field_name_input")
        LabeledField("invoice_field_street", form.street, callbacks.onStreet, "invoice_field_street_input")
        Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
            LabeledField(
                "invoice_field_postal_code",
                form.postalCode,
                callbacks.onPostalCode,
                "invoice_field_postalCode_input",
                Modifier.weight(1f),
            )
            LabeledField("invoice_field_city", form.city, callbacks.onCity, "invoice_field_city_input", Modifier.weight(1f))
            LabeledField(
                "invoice_field_country",
                form.countryCode,
                callbacks.onCountryCode,
                "invoice_field_countryCode_input",
                Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun LabeledField(
    labelKey: String,
    value: String,
    onChange: (String) -> Unit,
    tag: String,
    modifier: Modifier = Modifier,
) {
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        FieldLabel(labelKey)
        InvoiceTextField(value, onChange, "", tag)
    }
}

@Composable
private fun FieldLabel(key: String) {
    Text(
        vueText(key),
        style = PiliTheme.typography.label.copy(fontWeight = PiliTheme.typography.weightBold),
        color = PiliTheme.colors.textSecondary,
    )
}

// ── Preview step ───────────────────────────────────────────────────

@Composable
private fun PreviewStep(
    flow: InvoiceFlow,
    sales: List<InvoiceableSale>,
    callbacks: InvoiceCallbacks,
) {
    val preview = flow.preview ?: return
    PiliDialog(
        title = vueText("invoice_title"),
        onDismissRequest = null,
        maxWidth = InvoiceMaxWidth,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onBack,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.width(SmallActionWidth).testTag("invoice_back"),
            )
            PiliButton(
                text = vueText("invoice_confirm"),
                onClick = callbacks.onConfirm,
                enabled = flow.canConfirm,
                modifier = Modifier.testTag("invoice_confirm"),
            )
        },
    ) {
        InvoiceHead(flow, sales)
        Row(
            modifier =
                Modifier.fillMaxWidth().heightIn(
                    max = InvoiceBodyMaxHeight,
                ).verticalScroll(rememberScrollState()).testTag("invoice_preview"),
            horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXl),
        ) {
            PreviewCustomer(flow, Modifier.weight(1f))
            PreviewBody(flow, preview, Modifier.weight(2f))
        }
    }
}

@Composable
private fun PreviewCustomer(
    flow: InvoiceFlow,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val form = flow.form
    Column(modifier, verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        PreviewTitle("invoice_preview_customer")
        Column(Modifier.semantics(mergeDescendants = true) {}.testTag("invoice_customer")) {
            Text(form.name, style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
            Text(form.street, style = t.body, color = c.textPrimary)
            Text("${form.postalCode} ${form.city} (${form.countryCode})", style = t.body, color = c.textPrimary)
            Text(form.vatNumber, style = t.figure, color = c.textPrimary)
        }
        PeppolChip(flow.peppolStatus)
        peppolNote(flow)?.let { Text(it, style = t.label, color = c.textSecondary) }
        PreviewTitle("invoice_preview_tickets")
        Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
            flow.preview?.sales?.forEach { sale ->
                Row(
                    Modifier.fillMaxWidth().semantics(mergeDescendants = true) {
                    }.testTag("invoice_sale_${sale.fiscalEventId}"),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("#${sale.ticketNumber}", style = t.figure, color = c.textPrimary)
                    Text("${sale.fdmId} ${sale.eventCounter}/${sale.totalCounter}", style = t.figure, color = c.textSecondary)
                    Text(formatEuro(sale.totalCents), style = t.figure, color = if (sale.totalCents < 0L) c.accentDanger else c.textPrimary)
                }
            }
        }
    }
}

@Composable
private fun PreviewBody(
    flow: InvoiceFlow,
    preview: InvoicePreview,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(modifier, verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        PreviewTitle("invoice_preview_lines")
        Column(Modifier.heightIn(max = PreviewListMaxHeight).verticalScroll(rememberScrollState()).testTag("invoice_lines")) {
            preview.lines.forEachIndexed { i, line ->
                Row(
                    Modifier.fillMaxWidth().padding(
                        vertical = PiliTheme.dimens.spaceXs,
                    ).semantics(mergeDescendants = true) {}.testTag("invoice_line_$i"),
                    horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        "${quantityText(line.quantityMilli)}x",
                        style = t.figure,
                        color = c.textPrimary,
                        modifier = Modifier.width(QuantityWidth),
                    )
                    Text(line.description, style = t.body, color = c.textPrimary, modifier = Modifier.weight(1f))
                    Text(line.vatLabel, style = t.figure, color = c.textSecondary, modifier = Modifier.width(VatLabelWidth))
                    Text(
                        formatCents(line.totalCents),
                        style = t.figure,
                        color = if (line.totalCents < 0L) c.accentDanger else c.textPrimary,
                        textAlign = TextAlign.End,
                        modifier = Modifier.width(AmountWidth),
                    )
                }
            }
        }
        PreviewTitle("invoice_preview_vat")
        VatTable(preview)
        Row(
            Modifier.fillMaxWidth().semantics(mergeDescendants = true) {
            }.testTag("invoice_total"),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(vueText("invoice_total"), style = t.title, color = c.textPrimary)
            Text(formatEuro(preview.totalCents), style = t.title, color = c.textPrimary)
        }
        Text(
            vueText("invoice_already_paid", "total" to formatCents(preview.totalCents)),
            style = t.label,
            color = c.textSecondary,
            modifier = Modifier.testTag("invoice_already_paid"),
        )
        flow.problems.forEachIndexed { i, problem -> ErrorLine(problem, "invoice_problem_$i") }
    }
}

@Composable
private fun VatTable(preview: InvoicePreview) {
    val c = PiliTheme.colors
    Column(Modifier.fillMaxWidth().testTag("invoice_vat"), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        VatRow(
            "",
            vueText("invoice_vat_base"),
            vueText("invoice_vat_amount"),
            vueText("invoice_vat_total"),
            c.textSecondary,
            "invoice_vat_head",
        )
        preview.vat.forEach { v ->
            VatRow(
                "${v.label} · ${rateText(v.rateHundredths)}",
                formatCents(v.baseCents),
                formatCents(v.vatCents),
                formatCents(v.totalCents),
                c.textPrimary,
                "invoice_vat_${v.label}",
            )
        }
    }
}

@Composable
private fun VatRow(
    label: String,
    base: String,
    vat: String,
    total: String,
    color: Color,
    tag: String,
) {
    val t = PiliTheme.typography
    Row(
        Modifier.fillMaxWidth().semantics(mergeDescendants = true) {
        }.testTag(tag),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        Text(label, style = t.figure, color = color, modifier = Modifier.weight(1.4f))
        listOf(base, vat, total).forEach {
            Text(it, style = t.figure, color = color, textAlign = TextAlign.End, modifier = Modifier.weight(1f))
        }
    }
}

@Composable
private fun PreviewTitle(key: String) {
    Text(vueText(key), style = PiliTheme.typography.title, color = PiliTheme.colors.textPrimary)
}

// ── Result step ────────────────────────────────────────────────────

@Composable
private fun ResultStep(
    flow: InvoiceFlow,
    callbacks: InvoiceCallbacks,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val result = flow.result
    PiliDialog(
        title = vueText("invoice_title"),
        onDismissRequest = callbacks.onClose,
        maxWidth = InvoiceMaxWidth,
        actions = {
            PiliButton(
                text = vueText("finish"),
                onClick = callbacks.onClose,
                modifier = Modifier.width(ActionWidth).testTag("invoice_finish"),
            )
        },
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().testTag("invoice_result"),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            if (result != null && result.success) {
                Text(
                    vueText("invoice_done_title", "no" to result.invoiceNo.orEmpty()),
                    style = t.title,
                    color = c.accentSuccess,
                    modifier = Modifier.testTag("invoice_result_title"),
                )
                Text(
                    vueText(if (result.printed) "invoice_printed" else "invoice_not_printed"),
                    style = t.body,
                    color = c.textPrimary,
                    modifier = Modifier.testTag("invoice_result_printed"),
                )
                Text(
                    vueText(if (result.deliveryStatus == "queued") "invoice_delivery_queued" else "invoice_delivery_none"),
                    style = t.body,
                    color = c.textSecondary,
                    modifier = Modifier.testTag("invoice_result_delivery"),
                )
            } else {
                Text(
                    vueText("invoice_failed"),
                    style = t.title,
                    color = c.accentDanger,
                    modifier = Modifier.testTag("invoice_result_title"),
                )
                flow.resultError?.let { ErrorLine(it, "invoice_error") }
            }
        }
    }
}
