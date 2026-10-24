@file:Suppress("TooManyFunctions") // One small composable per block of the tab.

package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.SalesDay
import uniffi.pos_ffi.SalesReport

private val LeftWidth = 560.dp
private val QuantityWidth = 110.dp
private val AmountWidth = 150.dp
private val HourLabelWidth = 64.dp
private val HourAmountWidth = 130.dp
private val HourTicketsWidth = 110.dp
private val BarHeight = 18.dp
private val DayMarkWidth = 6.dp
private val FieldHeight = 64.dp
private val SortHeight = 48.dp
private const val PERCENT = 100f
private const val MIN_BAR = 0.02f
private const val TABLE_WEIGHT = 3f
private const val HOURS_WEIGHT = 2f

/**
 * The sales tab (`SalesReportPanel.vue`): the non-fiscal sales report of a range of booking dates. On the left the
 * range choices, the concordance with the Z of every period and the totals; on the right the by-product table and
 * the by-hour bars. Test tags: `sales_notice`, `sales_print`, `sales_preset_<name>`, `sales_field_from`,
 * `sales_field_to`, `sales_apply`, `sales_range`, `sales_loading`, `sales_error`, `sales_day_<index>`, `sales_tickets`,
 * `sales_refunds`, `sales_rounding`, `sales_total`, `sales_sort_amount`, `sales_sort_quantity`,
 * `sales_product_<index>`, `sales_hour_<hour>`, `sales_hour_bar_<hour>`, `sales_empty`.
 */
@Composable
internal fun SalesTab(
    sales: SalesState,
    callbacks: SalesCallbacks,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    Row(modifier.fillMaxSize(), horizontalArrangement = Arrangement.spacedBy(d.spaceLg)) {
        SalesControls(sales, callbacks, Modifier.width(LeftWidth).fillMaxHeight())
        SalesTables(sales, callbacks, Modifier.weight(1f).fillMaxHeight())
    }
}

@Composable
private fun SalesControls(
    sales: SalesState,
    callbacks: SalesCallbacks,
    modifier: Modifier,
) {
    val d = PiliTheme.dimens
    val c = PiliTheme.colors
    Panel(modifier) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(d.spaceMd)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(vueText("sales_title"), style = PiliTheme.typography.title, color = c.textPrimary)
                PiliButton(
                    text = vueText("sales_print"),
                    onClick = callbacks.onPrint,
                    enabled = !sales.printing && sales.report != null,
                    modifier = Modifier.testTag("sales_print"),
                )
            }
            Text(
                vueText("sales_not_fiscal"),
                style = PiliTheme.typography.label,
                color = c.textSecondary,
                modifier = Modifier.testTag("sales_notice"),
            )
            Presets(sales.preset, callbacks.onPreset)
            if (sales.preset == SalesPreset.CUSTOM) CustomRange(sales, callbacks)
            if (sales.from.isNotEmpty()) {
                Text(
                    "${SalesRange.showDate(sales.from)} - ${SalesRange.showDate(sales.to)}",
                    style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
                    color = c.textPrimary,
                    modifier = Modifier.testTag("sales_range"),
                )
            }
            when {
                sales.loading -> Text(vueText("sales_loading"), color = c.textSecondary, modifier = Modifier.testTag("sales_loading"))
                sales.error != null -> Text(sales.error, color = c.accentDanger, modifier = Modifier.testTag("sales_error"))
            }
            sales.report?.let { report ->
                Text(vueText("sales_concordance"), style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold))
                if (report.days.isEmpty()) {
                    Text(vueText("sales_empty"), color = c.textSecondary, modifier = Modifier.testTag("sales_empty"))
                }
                Column(verticalArrangement = Arrangement.spacedBy(d.spaceXs)) {
                    report.days.forEachIndexed { index, day -> DayRow(index, day) }
                }
                Totals(report)
            }
        }
    }
}

@Composable
private fun Presets(
    selected: SalesPreset,
    onPreset: (SalesPreset) -> Unit,
) {
    val d = PiliTheme.dimens
    val rows = listOf(listOf(SalesPreset.TODAY, SalesPreset.YESTERDAY, SalesPreset.WEEK), listOf(SalesPreset.MONTH, SalesPreset.CUSTOM))
    Column(verticalArrangement = Arrangement.spacedBy(d.spaceXs)) {
        rows.forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceXs)) {
                row.forEach { preset ->
                    ChoiceButton(
                        text = vueText(presetKey(preset)),
                        selected = preset == selected,
                        onClick = { onPreset(preset) },
                        modifier = Modifier.weight(1f).testTag("sales_preset_${preset.name.lowercase()}"),
                    )
                }
                if (row.size < PRESET_COLUMNS) Box(Modifier.weight((PRESET_COLUMNS - row.size).toFloat()))
            }
        }
    }
}

private const val PRESET_COLUMNS = 3

private fun presetKey(preset: SalesPreset): String =
    when (preset) {
        SalesPreset.TODAY -> "sales_today"
        SalesPreset.YESTERDAY -> "sales_yesterday"
        SalesPreset.WEEK -> "sales_week"
        SalesPreset.MONTH -> "sales_month"
        SalesPreset.CUSTOM -> "sales_custom"
    }

@Composable
private fun CustomRange(
    sales: SalesState,
    callbacks: SalesCallbacks,
) {
    val d = PiliTheme.dimens
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            DateField(
                "sales_from",
                sales.fromDigits,
                sales.field == SalesField.FROM,
                { callbacks.onField(SalesField.FROM) },
                "sales_field_from",
                Modifier.weight(1f),
            )
            DateField(
                "sales_to",
                sales.toDigits,
                sales.field == SalesField.TO,
                { callbacks.onField(SalesField.TO) },
                "sales_field_to",
                Modifier.weight(1f),
            )
        }
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(d.spaceLg),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            NumPad(
                onKey = callbacks.onKey,
                clearLabel = uiText(R.string.ui_common_numpad_clear),
                backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
            )
            PiliButton(
                text = vueText("sales_apply"),
                onClick = callbacks.onApply,
                enabled = sales.canApply,
                modifier = Modifier.weight(1f).height(FieldHeight).testTag("sales_apply"),
            )
        }
    }
}

@Composable
private fun DateField(
    labelKey: String,
    digits: String,
    active: Boolean,
    onClick: () -> Unit,
    tag: String,
    modifier: Modifier,
) {
    ChoiceButton(
        text = vueText(labelKey) + " " + SalesRange.showTyped(digits),
        selected = active,
        onClick = onClick,
        modifier = modifier.height(FieldHeight).testTag(tag),
    )
}

@Composable
private fun DayRow(
    index: Int,
    day: SalesDay,
) {
    val d = PiliTheme.dimens
    val c = PiliTheme.colors
    val mark =
        when {
            day.open -> c.accentWarning
            day.matchesZ -> c.accentSuccess
            else -> c.accentDanger
        }
    Row(
        Modifier
            .fillMaxWidth()
            .background(c.bgSurface, RoundedCornerShape(d.radiusMd))
            .padding(end = d.spaceSm)
            .semantics(mergeDescendants = true) {}
            .testTag("sales_day_$index"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(DayMarkWidth).height(FieldHeight).background(mark))
        Text(SalesRange.showDate(day.bookingDate), fontFamily = FontFamily.Monospace, color = c.textPrimary)
        Text(dayText(day), color = c.textSecondary, modifier = Modifier.weight(1f))
        Text(formatEuro(day.totalCents), fontFamily = FontFamily.Monospace, color = c.textPrimary)
    }
}

@Composable
private fun dayText(day: SalesDay): String {
    val number = day.zNumber
    val z = day.zTotalCents
    val gap = day.gapCents
    return when {
        day.open -> vueText("sales_open")
        number == null || z == null -> vueText("sales_z_missing")
        day.matchesZ -> vueText("sales_z_equal", "no" to number)
        else -> vueText("sales_z_gap", "no" to number, "gap" to dayAmount(gap ?: 0L), "z" to dayAmount(z))
    }
}

/** An amount without the euro sign, as `formatPrice(c)` is used inside a sentence of the Vue strings. */
private fun dayAmount(cents: Long): String = formatEuro(cents).removeSuffix(" €")

@Composable
private fun Totals(report: SalesReport) {
    val c = PiliTheme.colors
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        TotalRow("sales_tickets", report.ticketCount.toString(), "sales_tickets")
        if (report.refundsCents != 0L) TotalRow("sales_refunds", formatEuro(report.refundsCents), "sales_refunds")
        if (report.priceChangesCents != 0L) TotalRow("sales_price_changes", formatEuro(report.priceChangesCents), "sales_price_changes")
        if (report.roundingCents != 0L) TotalRow("sales_rounding", formatEuro(report.roundingCents), "sales_rounding")
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(vueText("sales_total"), style = PiliTheme.typography.title, color = c.textPrimary)
            Text(
                formatEuro(report.totalCents),
                style = PiliTheme.typography.title,
                fontFamily = FontFamily.Monospace,
                color = c.textPrimary,
                modifier = Modifier.testTag("sales_total"),
            )
        }
    }
}

@Composable
private fun TotalRow(
    labelKey: String,
    value: String,
    tag: String,
) {
    val c = PiliTheme.colors
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(vueText(labelKey), color = c.textSecondary)
        Text(value, fontFamily = FontFamily.Monospace, color = c.textPrimary, modifier = Modifier.testTag(tag + "_value"))
    }
}

@Composable
private fun SalesTables(
    sales: SalesState,
    callbacks: SalesCallbacks,
    modifier: Modifier,
) {
    val d = PiliTheme.dimens
    Panel(modifier) {
        Column(Modifier.weight(TABLE_WEIGHT), verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            ProductsHeader(sales.sort, callbacks.onSort)
            LazyColumn(Modifier.weight(1f)) {
                val rows = sales.products
                items(rows.size) { index ->
                    val p = rows[index]
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .padding(vertical = d.spaceXs)
                            .semantics(mergeDescendants = true) {}
                            .testTag("sales_product_$index"),
                        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
                    ) {
                        Text(
                            p.name,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.weight(1f),
                            color = PiliTheme.colors.textPrimary,
                        )
                        Text(
                            SalesRange.showQuantity(p.quantityMilli),
                            fontFamily = FontFamily.Monospace,
                            textAlign = TextAlign.End,
                            color = if (p.quantityMilli < 0L) PiliTheme.colors.accentDanger else PiliTheme.colors.textPrimary,
                            modifier = Modifier.width(QuantityWidth),
                        )
                        Text(
                            formatEuro(p.amountCents),
                            fontFamily = FontFamily.Monospace,
                            textAlign = TextAlign.End,
                            color = if (p.amountCents < 0L) PiliTheme.colors.accentDanger else PiliTheme.colors.textPrimary,
                            modifier = Modifier.width(AmountWidth),
                        )
                    }
                }
            }
        }
        Column(Modifier.weight(HOURS_WEIGHT), verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            Text(vueText("sales_by_hour"), style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold))
            LazyColumn(Modifier.weight(1f)) {
                val rows = sales.report?.byHour.orEmpty()
                items(rows.size) { index -> HourRow(rows[index]) }
            }
        }
    }
}

@Composable
private fun ProductsHeader(
    sort: SalesSort,
    onSort: (SalesSort) -> Unit,
) {
    val d = PiliTheme.dimens
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Text(vueText("sales_by_product"), style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold))
        Row(horizontalArrangement = Arrangement.spacedBy(d.spaceXs)) {
            ChoiceButton(
                vueText("sales_sort_amount"),
                sort == SalesSort.AMOUNT,
                { onSort(SalesSort.AMOUNT) },
                Modifier.height(SortHeight).testTag("sales_sort_amount"),
            )
            ChoiceButton(
                vueText("sales_sort_quantity"),
                sort == SalesSort.QUANTITY,
                { onSort(SalesSort.QUANTITY) },
                Modifier.height(SortHeight).testTag("sales_sort_quantity"),
            )
        }
    }
}

@Composable
private fun HourRow(hour: uniffi.pos_ffi.SalesHour) {
    val d = PiliTheme.dimens
    val c = PiliTheme.colors
    Row(
        Modifier.fillMaxWidth().padding(vertical = d.spaceXs).semantics(mergeDescendants = true) {}.testTag("sales_hour_${hour.hour}"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            SalesRange.showHour(hour.hour.toInt()),
            fontFamily = FontFamily.Monospace,
            color = c.textPrimary,
            modifier = Modifier.width(HourLabelWidth),
        )
        Box(Modifier.weight(1f).height(BarHeight).background(c.bgSurface, RoundedCornerShape(BarHeight))) {
            Box(
                Modifier
                    .fillMaxWidth((hour.barPercent.toFloat() / PERCENT).coerceIn(MIN_BAR, 1f))
                    .fillMaxHeight()
                    .background(c.selectedBg, RoundedCornerShape(BarHeight))
                    .testTag("sales_hour_bar_${hour.hour}"),
            )
        }
        Text(
            formatEuro(hour.amountCents),
            fontFamily = FontFamily.Monospace,
            textAlign = TextAlign.End,
            color = c.textPrimary,
            modifier = Modifier.width(HourAmountWidth),
        )
        Text(
            vueText("sales_hour_tickets", "n" to hour.tickets),
            color = c.textSecondary,
            textAlign = TextAlign.End,
            modifier = Modifier.width(HourTicketsWidth),
        )
    }
}
