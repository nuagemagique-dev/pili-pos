package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.GIFT_VOUCHER_CODE_LENGTH
import dev.nuagemagique.pilipos.ui.components.GiftVoucherSummary
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.spacedVoucherCode
import dev.nuagemagique.pilipos.ui.components.voucherDate
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.GiftVoucherView

private val LookupWidth = 440.dp

/**
 * The gift vouchers tab (`GiftVouchersReport.vue`): the lookup of a code on a pad (cashiers and managers; the engine
 * refuses a waiter), and for a manager the list of the vouchers issued with a status filter, their movements and
 * the DUPLICATA reprint. Test tags:
 * `gv_report`, `gv_report_code`, `gv_report_lookup`, `gv_report_error`, `gv_report_found`, `gv_filter_<status|all>`,
 * `gv_list`, `gv_empty`, `gv_row_<id>`, `gv_head_<id>`, `gv_reprint_<id>`.
 */
@Composable
internal fun VouchersTab(
    vouchers: VouchersState,
    callbacks: VouchersCallbacks,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier.fillMaxSize().testTag("gv_report"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceLg),
    ) {
        LookupPanel(vouchers, callbacks, Modifier.width(LookupWidth))
        if (vouchers.manager) ListPanel(vouchers, callbacks, Modifier.weight(1f))
    }
}

@Composable
private fun LookupPanel(
    vouchers: VouchersState,
    callbacks: VouchersCallbacks,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Panel(modifier) {
        Text(vueText("gv_lookup_title"), style = t.title, color = c.textPrimary)
        Text(
            text = spacedVoucherCode(vouchers.digits).ifEmpty { " " },
            style = t.figure.copy(fontSize = t.sizeXl),
            color = c.accentPrimary,
            modifier = Modifier.testTag("gv_report_code"),
        )
        NumPad(
            onKey = callbacks.onKey,
            clearLabel = uiText(R.string.ui_common_numpad_clear),
            backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
        )
        PiliButton(
            text = vueText("gv_lookup"),
            onClick = callbacks.onLookup,
            enabled = vouchers.digits.length == GIFT_VOUCHER_CODE_LENGTH && !vouchers.busy,
            modifier = Modifier.fillMaxWidth().testTag("gv_report_lookup"),
        )
        vouchers.lookupError?.let {
            Text(it, style = t.body, color = c.accentDanger, modifier = Modifier.testTag("gv_report_error"))
        }
        vouchers.found?.let { GiftVoucherSummary(it, tag = "gv_report") }
    }
}

@Composable
private fun ListPanel(
    vouchers: VouchersState,
    callbacks: VouchersCallbacks,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Panel(modifier.fillMaxSize()) {
        Text(vueText("gv_list_title"), style = t.title, color = c.textPrimary)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            VOUCHER_FILTERS.forEach { status ->
                ChoiceButton(
                    text = if (status == null) vueText("gv_filter_all") else vueText("gv_status_$status"),
                    selected = vouchers.filter == status,
                    onClick = { callbacks.onFilter(status) },
                    modifier = Modifier.weight(1f).testTag("gv_filter_${status ?: "all"}"),
                )
            }
        }
        vouchers.listError?.let { Text(it, style = t.body, color = c.accentDanger, modifier = Modifier.testTag("gv_list_error")) }
        if (vouchers.listError == null && vouchers.list.isEmpty()) {
            Text(vueText("gv_empty"), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("gv_empty"))
        } else {
            LazyColumn(Modifier.fillMaxSize().testTag("gv_list"), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
                items(vouchers.list, key = { it.id }) { voucher ->
                    VoucherRow(voucher, vouchers.open == voucher.id, callbacks)
                }
            }
        }
    }
}

@Composable
private fun VoucherRow(
    voucher: GiftVoucherView,
    open: Boolean,
    callbacks: VouchersCallbacks,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val statusColor = if (voucher.status == "active") c.accentSuccess else c.accentDanger
    Column(Modifier.fillMaxWidth().testTag("gv_row_${voucher.id}")) {
        Row(
            Modifier.fillMaxWidth().testTag("gv_head_${voucher.id}").clickable { callbacks.onToggle(voucher.id) }
                .padding(vertical = PiliTheme.dimens.spaceSm),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(voucher.codeMasked, style = t.figure, color = c.textPrimary, modifier = Modifier.weight(1f))
            Text(
                vueText("gv_status_${voucher.status}"),
                style = t.body.copy(fontWeight = t.weightBold),
                color = statusColor,
                modifier = Modifier.padding(end = PiliTheme.dimens.spaceMd),
            )
            Text(
                text = formatEuro(voucher.balanceCents) + " / " + formatEuro(voucher.faceValueCents),
                style = t.body.copy(fontWeight = t.weightBold),
                color = c.textPrimary,
                modifier = Modifier.width(MoneyWidth),
            )
        }
        if (open) {
            Text(
                vueText("gv_issued_on", "date" to voucherDate(voucher.issuedOn)) + " · " +
                    vueText("gv_expires", "date" to voucherDate(voucher.expiresOn)),
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
            )
            Text(vueText("gv_movements"), style = t.body.copy(fontSize = t.sizeSm), color = c.textSecondary)
            voucher.movements.forEach { m ->
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(
                        vueText("gv_movement_${m.kind}") + " · " + voucherDate(m.date) + " · " + m.ticket,
                        style = t.body.copy(fontSize = t.sizeSm),
                        color = c.textSecondary,
                    )
                    Text(formatEuro(m.amountCents), style = t.body.copy(fontSize = t.sizeSm), color = c.textSecondary)
                }
            }
            PiliButton(
                text = vueText("gv_reprint"),
                onClick = { callbacks.onReprint(voucher.id) },
                modifier = Modifier.testTag("gv_reprint_${voucher.id}"),
            )
        }
    }
}

private val MoneyWidth = 240.dp
