package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.bilingualName
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderType

private val PayHeight = 80.dp

/**
 * The cart column, minimal (P5b-T7): the lines with their quantity and their amount, the engine total and the pay
 * button. P5b-T12 replaces this composable with the full cart (`CartSidebar.vue` and `OrderLineItem.vue`) and keeps
 * its signature. The total is the engine's `Order.totalCents` and a line's amount comes from `CartAmounts` (PD208);
 * nothing is added here. The pay button needs the payment right and at least one line.
 *
 * Test tags: `cart_column`, `cart_title`, `cart_empty`, `cart_line_<lineId>`, `cart_line_<lineId>_amount`,
 * `cart_total`, `cart_pay`.
 */
@Composable
internal fun OrderCartColumn(
    state: OrderCartState,
    actions: OrderCartActions,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Column(
        modifier =
            modifier
                .background(c.bgSecondary)
                .drawBehind { drawRect(c.border, size = Size(1.dp.toPx(), size.height)) }
                .testTag("cart_column"),
    ) {
        Text(
            text = cartTitle(state.order),
            style = t.title,
            color = c.textPrimary,
            modifier = Modifier.fillMaxWidth().drawBottomLine(c.border).padding(d.spaceMd).testTag("cart_title"),
        )
        Column(
            modifier =
                Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = d.spaceMd, vertical = d.spaceSm),
            verticalArrangement = Arrangement.spacedBy(d.spaceSm),
        ) {
            if (state.rows.isEmpty()) {
                Text(
                    text = uiText(R.string.ui_order_cart_empty),
                    style = t.body,
                    color = c.textSecondary,
                    modifier = Modifier.testTag("cart_empty"),
                )
            }
            state.rows.forEach { CartLineRow(it, state) }
        }
        Column(Modifier.fillMaxWidth().drawTopLine(c.border).padding(d.spaceMd), verticalArrangement = Arrangement.spacedBy(d.spaceMd)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(vueText("total"), style = t.title, color = c.textPrimary)
                Text(
                    text = formatEuro(state.order?.totalCents ?: 0L),
                    style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
                    color = c.textPrimary,
                    modifier = Modifier.testTag("cart_total"),
                )
            }
            if (state.canPay) {
                PiliButton(
                    onClick = actions.onPay,
                    enabled = state.rows.isNotEmpty(),
                    modifier = Modifier.fillMaxWidth().height(PayHeight).testTag("cart_pay"),
                ) {
                    Text(vueText("pay"), style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightHeavy), color = c.textOnAccent)
                }
            }
        }
    }
}

/** "Order · Table 4" or "Order · Takeaway" (`cartTitle` of `CartSidebar.vue`). */
@Composable
private fun cartTitle(order: Order?): String {
    val name = vueText("order")
    return when {
        order?.tableNumber != null -> "$name · ${vueText("table")} ${order.tableNumber}"
        order?.orderType == OrderType.TAKEAWAY -> "$name · ${vueText("takeaway_badge")}"
        else -> name
    }
}

@Composable
private fun CartLineRow(
    row: CartRow,
    state: OrderCartState,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val names = bilingualName(row.line, state.language, state.labelMode)
    val quantity = "${row.line.quantity}×"
    Row(
        modifier = Modifier.fillMaxWidth().semantics(mergeDescendants = true) {}.testTag("cart_line_${row.line.id}"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
        verticalAlignment = Alignment.Top,
    ) {
        Text(quantity, style = t.figure.copy(fontWeight = t.weightBold), color = c.textPrimary)
        Column(Modifier.weight(1f)) {
            Text(
                text = names.primary,
                style = t.body.copy(fontWeight = t.weightBold),
                color = c.textPrimary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            names.secondary?.let {
                Text(
                    text = it,
                    style = t.body.copy(fontSize = t.sizeSm),
                    color = c.textSecondary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Text(
            text = amountText(row.amount),
            style = t.figure.copy(fontWeight = t.weightBold),
            color = c.textPrimary,
            maxLines = 1,
            modifier = Modifier.testTag("cart_line_${row.line.id}_amount"),
        )
    }
}

/** The amount of a line: its total, or quantity and unit price for a line the VAT split leaves out (PD208). */
private fun amountText(amount: LineAmount?): String =
    when (amount) {
        is LineAmount.Total -> formatEuro(amount.cents)
        is LineAmount.PerUnit -> "${amount.quantity} × ${formatEuro(amount.unitCents)}"
        null -> ""
    }

private fun Modifier.drawBottomLine(color: Color): Modifier =
    drawBehind { drawRect(color, topLeft = Offset(0f, size.height - 1.dp.toPx()), size = Size(size.width, 1.dp.toPx())) }

private fun Modifier.drawTopLine(color: Color): Modifier = drawBehind { drawRect(color, size = Size(size.width, 1.dp.toPx())) }
