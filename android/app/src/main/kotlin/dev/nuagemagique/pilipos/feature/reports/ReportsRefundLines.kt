package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Remove
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val LineMinHeight = 72.dp
private val StepperSize = 56.dp
private val CheckSize = 32.dp
private val AmountWidth = 170.dp

@Composable
@Suppress("LongParameterList") // One row: the line, what is ticked of it, its engine amount and the actions.
internal fun RefundLineRow(
    name: String,
    lineId: Long,
    quantity: Int,
    ticked: Int?,
    amount: LineAmount?,
    callbacks: TicketsCallbacks,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val fg = if (ticked != null) c.selectedFg else c.textPrimary
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = LineMinHeight)
                .background(if (ticked != null) c.selectedBg else c.bgSurface, RoundedCornerShape(d.radiusMd))
                .clickable(role = Role.Checkbox) { callbacks.onToggleLine(lineId) }
                .padding(horizontal = d.spaceMd, vertical = d.spaceSm)
                .testTag("refund_line_$lineId"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(CheckSize), contentAlignment = Alignment.Center) {
            if (ticked != null) Icon(Icons.Filled.Check, contentDescription = null, tint = fg)
        }
        Text(name, style = t.body.copy(fontWeight = t.weightBold), color = fg, modifier = Modifier.weight(1f))
        if (ticked != null) {
            Stepper(lineId, ticked, quantity, fg, callbacks.onQuantity)
        } else {
            Text("${quantity}x", style = t.figure, color = c.textSecondary, modifier = Modifier.testTag("refund_orig_$lineId"))
        }
        Text(
            text = amountText(amount),
            style = t.figure.copy(fontWeight = t.weightBold),
            color = fg,
            textAlign = TextAlign.End,
            modifier = Modifier.width(AmountWidth).testTag("refund_amount_$lineId"),
        )
    }
}

/** The price of a line as the engine gave it: the total, or `quantity x unit price` for a line the split leaves out. */
@Composable
internal fun amountText(amount: LineAmount?): String =
    when (amount) {
        is LineAmount.Total -> formatEuro(amount.cents)
        is LineAmount.PerUnit ->
            uiText(R.string.ui_reports_line_per_unit, "quantity" to amount.quantity, "price" to formatEuro(amount.unitCents))
        null -> ""
    }

@Composable
private fun Stepper(
    lineId: Long,
    ticked: Int,
    max: Int,
    tint: Color,
    onQuantity: (Long, Int) -> Unit,
) {
    val t = PiliTheme.typography
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm), verticalAlignment = Alignment.CenterVertically) {
        StepperButton(Icons.Filled.Remove, uiText(R.string.ui_reports_quantity_less), Modifier.testTag("refund_minus_$lineId")) {
            onQuantity(lineId, -1)
        }
        Text("$ticked", style = t.figure.copy(fontWeight = t.weightBold), color = tint, modifier = Modifier.testTag("refund_qty_$lineId"))
        StepperButton(Icons.Filled.Add, uiText(R.string.ui_reports_quantity_more), Modifier.testTag("refund_plus_$lineId")) {
            onQuantity(lineId, 1)
        }
        Text("/ $max", style = t.figure, color = tint)
    }
}

@Composable
private fun StepperButton(
    icon: ImageVector,
    description: String,
    modifier: Modifier,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    Box(
        modifier =
            modifier
                .size(StepperSize)
                .background(c.bgSecondary, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(role = Role.Button, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, contentDescription = description, tint = c.textPrimary)
    }
}
