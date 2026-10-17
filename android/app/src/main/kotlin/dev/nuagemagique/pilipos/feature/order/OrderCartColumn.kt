package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.VatSummary
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.delay
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderType
import java.time.Instant

private val PayHeight = 80.dp
private val ActionHeight = 64.dp
private val UnpaidHeight = 48.dp
private val BadgeSize = 26.dp
private val Hairline = 1.dp
private val PendingDot = 10.dp

/** How often "opened at" is refreshed, in milliseconds (30 s in `CartSidebar.vue`). */
private const val OPENED_TICK_MS = 30_000L

/**
 * The cart column (`CartSidebar.vue`): the title and when the order was opened, the lines (a dine-in line not yet
 * sent goes last under an amber label), the engine total with the VAT included, "customer left without paying"
 * for a manager, send to kitchen with the unsent count, the provisional bill, and the pay button. The total is the
 * engine's `Order.totalCents` and a line's amount comes from `CartAmounts` (PD208); nothing is added here. The pay
 * and pre-bill buttons need the payment right; pay also needs at least one line; send to kitchen is disabled when
 * nothing is unsent or while it runs. Refusals are the screen's, shown by the view model through `ErrorPresenter`.
 *
 * Test tags: `cart_column`, `cart_title`, `cart_opened`, `cart_empty`, `cart_pending_label`, `cart_unpaid`,
 * `cart_transfer`, `cart_total`, `vat_summary`, `cart_send`, `cart_send_count`, `cart_prebill`, `cart_pay`, `cart_pay_amount`, and
 * those of [OrderLineItem].
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
    val dineIn = state.order?.orderType == OrderType.DINE_IN
    // Dine-in lines not yet sent go last under an amber label; a takeaway's lines all go out with the sale.
    val sent = if (dineIn) state.rows.filter { it.line.unsentQuantity <= 0 } else state.rows
    val pending = if (dineIn) state.rows.filter { it.line.unsentQuantity > 0 } else emptyList()
    Column(
        modifier =
            modifier
                .background(c.bgSecondary)
                .drawBehind { drawRect(c.border, size = Size(Hairline.toPx(), size.height)) }
                .testTag("cart_column"),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().drawBottomLine(c.border).padding(horizontal = d.spaceMd, vertical = d.spaceMd),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = cartTitle(state.order),
                style = t.title,
                color = c.textPrimary,
                modifier = Modifier.weight(1f).testTag("cart_title"),
            )
            if (state.canTransfer) {
                PiliButton(
                    text = vueText("transfer_table"),
                    onClick = actions.onTransfer,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.padding(horizontal = d.spaceSm).testTag("cart_transfer"),
                )
            }
            OpenedAt(state.order)
        }
        Column(
            modifier =
                Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = d.spaceMd),
        ) {
            sent.forEach { OrderLineItem(it, state, actions, pending = false) }
            if (pending.isNotEmpty()) {
                PendingLabel()
                pending.forEach { OrderLineItem(it, state, actions, pending = true) }
            }
            if (state.rows.isEmpty()) {
                Box(Modifier.fillMaxWidth().height(EmptyHeight), contentAlignment = Alignment.Center) {
                    Text(text = vueText("no_items"), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("cart_empty"))
                }
            }
        }
        Column(
            modifier = Modifier.fillMaxWidth().drawTopLine(c.border).padding(d.spaceMd),
            verticalArrangement = Arrangement.spacedBy(d.spaceMd),
        ) {
            TotalRow(state, actions)
            if (dineIn) KitchenRow(state, actions)
            if (state.canPay) PayButton(state, actions)
        }
    }
}

private val EmptyHeight = 120.dp

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

/** The clock of the cart's "opened at"; the real time unless a test or a golden provides a fixed one. */
internal val LocalOrderNow = staticCompositionLocalOf<() -> Instant> { Instant::now }

/** "19:05 · 37 min": when the order was opened, and for how long; refreshed every 30 s while composed. */
@Composable
private fun OpenedAt(order: Order?) {
    val clock = LocalOrderNow.current
    var now by remember { mutableStateOf(clock()) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(OPENED_TICK_MS)
            now = clock()
        }
    }
    val created = order?.createdAt?.takeIf { it.isNotEmpty() } ?: return
    val text = "${PosDateTime.formatTimestamp(created, PosDateTime::formatTime)} · ${PosDateTime.minutesSince(created, now)} min"
    Text(
        text = text,
        style = PiliTheme.typography.figure,
        color = PiliTheme.colors.textSecondary,
        maxLines = 1,
        modifier = Modifier.testTag("cart_opened"),
    )
}

/** The amber label over the lines that were not sent to the bar or kitchen yet. */
@Composable
private fun PendingLabel() {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = PiliTheme.dimens.spaceMd, bottom = PiliTheme.dimens.spaceXs)
                .semantics(mergeDescendants = true) {}
                .testTag("cart_pending_label"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(PendingDot).background(c.accentWarning, CircleShape))
        Text(
            text = vueText("not_sent_to_kitchen").uppercase(),
            style = t.figure.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
            color = c.accentWarning,
        )
    }
}

/** The unpaid button on the left (a manager only), the engine total and the VAT included on the right. */
@Composable
private fun TotalRow(
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.Bottom) {
        if (state.canCloseUnpaid) {
            Box(
                modifier =
                    Modifier
                        .defaultMinSize(minHeight = UnpaidHeight)
                        .border(Hairline, c.accentDanger, RoundedCornerShape(d.radiusMd))
                        .clickable(role = Role.Button, onClick = actions.onCloseUnpaid)
                        .padding(horizontal = d.spaceMd)
                        .testTag("cart_unpaid"),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = vueText("unpaid_close"),
                    style = t.body.copy(fontWeight = t.weightBold),
                    color = c.accentDanger,
                    textAlign = TextAlign.Center,
                )
            }
        }
        Column(Modifier.weight(1f), horizontalAlignment = Alignment.End) {
            Text(
                text = formatEuro(state.order?.totalCents ?: 0L),
                style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
                color = c.textPrimary,
                modifier = Modifier.testTag("cart_total"),
            )
            VatSummary(state.vatRows)
        }
    }
}

/** Send to kitchen with the unsent count, and the provisional bill (dine-in only, `cart-actions`). */
@Composable
private fun KitchenRow(
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val canPreBill = state.rows.isNotEmpty() && !state.preBilling
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
        PiliButton(
            onClick = actions.onSend,
            variant = PiliButtonVariant.Secondary,
            enabled = state.unsentCount > 0 && !state.sending,
            modifier = Modifier.weight(1f).height(ActionHeight).testTag("cart_send"),
        ) {
            Text(vueText("send_to_kitchen"), style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightBold))
            if (state.unsentCount > 0) {
                Box(
                    modifier =
                        Modifier
                            .defaultMinSize(minWidth = BadgeSize, minHeight = BadgeSize)
                            .background(c.accentWarning, CircleShape)
                            .padding(horizontal = d.spaceSm),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = "${state.unsentCount}",
                        style = t.figure.copy(fontSize = t.sizeSm),
                        color = c.textOnAccent,
                        modifier = Modifier.testTag("cart_send_count"),
                    )
                }
            }
        }
        if (state.canPay) {
            PiliButton(
                onClick = actions.onPreBill,
                variant = PiliButtonVariant.Secondary,
                enabled = canPreBill,
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("cart_prebill"),
            ) {
                Text(vueText("pre_bill"), style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightBold))
            }
        } else {
            Box(Modifier.weight(1f))
        }
    }
}

/** The main action, always bottom right: "PAY" and the engine total; it needs at least one line. */
@Composable
private fun PayButton(
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    PiliButton(
        onClick = actions.onPay,
        enabled = state.rows.isNotEmpty(),
        modifier = Modifier.fillMaxWidth().height(PayHeight).testTag("cart_pay"),
    ) {
        Row(Modifier.weight(1f), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(vueText("pay"), style = t.body.copy(fontSize = t.sizeXl, fontWeight = t.weightHeavy), color = c.textOnAccent)
            Text(
                text = formatEuro(state.order?.totalCents ?: 0L),
                style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
                color = c.textOnAccent,
                modifier = Modifier.testTag("cart_pay_amount"),
            )
        }
    }
}

private fun Modifier.drawBottomLine(color: Color): Modifier =
    drawBehind { drawRect(color, topLeft = Offset(0f, size.height - Hairline.toPx()), size = Size(size.width, Hairline.toPx())) }

private fun Modifier.drawTopLine(color: Color): Modifier = drawBehind { drawRect(color, size = Size(size.width, Hairline.toPx())) }
