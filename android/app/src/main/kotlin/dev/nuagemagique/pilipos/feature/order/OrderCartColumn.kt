@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MoreHoriz
import androidx.compose.material3.Icon
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
private val BadgeSize = 26.dp
private val Hairline = 1.dp

/** How often "opened at" is refreshed, in milliseconds (30 s in `CartSidebar.vue`). */
private const val OPENED_TICK_MS = 30_000L

/**
 * The cart column (`CartSidebar.vue`): the title, the "Plus" button (change table, discount, customer left, opened as
 * [CartMoreDialog]) and when the order was opened, the lines (a dine-in line not yet sent goes last under an amber
 * label), the engine total with the VAT included, and the footer actions. While a dine-in order has lines the
 * kitchen has not received ([OrderCartState.kitchenPrimary]) "Send to kitchen" is the Volt primary with a pill
 * ("2 lignes") and PAYER an outlined secondary showing the total; otherwise PAYER is the Volt primary. The total is
 * the engine's `Order.totalCents` and a line's amount comes from `CartAmounts` (PD208); nothing is added here. The pay
 * and pre-bill buttons need the payment right; pay also needs at least one line; send to kitchen is disabled when
 * nothing is unsent or while it runs. Refusals are the screen's, shown by the view model through `ErrorPresenter`.
 *
 * Test tags: `cart_column`, `cart_title`, `cart_opened`, `cart_empty`, `cart_pending_label`, `cart_more`,
 * `cart_total`, `vat_summary`, `cart_send`, `cart_send_count`, `cart_send_primary`, `cart_send_pill`,
 * `cart_prebill`, `cart_pay`, `cart_pay_amount`, `cart_pay_secondary`, `cart_pay_secondary_amount`, and those of
 * [OrderLineItem].
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
    Column(
        modifier =
            modifier
                .background(c.bgSecondary)
                .drawBehind { drawRect(c.border, size = Size(Hairline.toPx(), size.height)) }
                .testTag("cart_column"),
    ) {
        CartHeader(state, actions)
        CartLines(state, actions, dineIn, Modifier.weight(1f))
        Column(
            modifier = Modifier.fillMaxWidth().drawTopLine(c.border).padding(d.spaceMd),
            verticalArrangement = Arrangement.spacedBy(d.spaceMd),
        ) {
            TotalRow(state)
            if (state.heldPartCount > 0) {
                Text(
                    text = vueText("parts_kept", "n" to state.heldPartCount),
                    style = t.body.copy(fontWeight = t.weightBold),
                    color = c.accentWarning,
                    modifier = Modifier.testTag("cart_held_parts"),
                )
            }
            // C4: the secondary row is empty for a role that cannot pay while the kitchen button is the primary.
            if (dineIn && !(state.kitchenPrimary && !state.canPay)) ActionRow(state, actions)
            val next = state.nextHeldCourse
            if (dineIn && next != null) NextCourseButton(next, state.sendingNext, actions)
            when {
                state.kitchenPrimary -> KitchenPrimaryButton(state, actions)
                state.canPay -> PayButton(state, actions)
            }
        }
    }
}

/** "Order · Table 4" ("Table A-4" in a zone) or "Order · Takeaway" (`cartTitle` of `CartSidebar.vue`). */
@Composable
private fun cartTitle(
    order: Order?,
    tableLabel: String?,
): String {
    val name = vueText("order")
    return when {
        order?.tableNumber != null -> "$name · ${vueText("table")} ${tableLabel ?: order.tableNumber}"
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

/** The engine total and the VAT included, on the right. */
@Composable
private fun TotalRow(state: OrderCartState) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.Bottom) {
        Column(horizontalAlignment = Alignment.End) {
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

/**
 * The secondary row of a dine-in cart (`cart-actions`): "Send to kitchen" with the unit count, or the outlined PAYER
 * while lines wait for the kitchen (C4), and the provisional bill.
 */
@Composable
private fun ActionRow(
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val canPreBill = state.rows.isNotEmpty() && !state.preBilling
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
        if (state.kitchenPrimary) {
            PaySecondaryButton(state, actions, Modifier.weight(1f))
        } else {
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

/** The title of the order, the covers of a table, "Change table" when the order can move, and the opening time. */
@Composable
private fun CartHeader(
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    Column(
        modifier = Modifier.fillMaxWidth().drawBottomLine(c.border).padding(horizontal = d.spaceMd, vertical = d.spaceMd),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        CartTitleRow(state, actions)
        if (state.canSetCovers) {
            // UX audit T5: the guests of the table, optional; they show on the table card and default the equal parts.
            PiliButton(
                text = coversText(state.covers),
                onClick = actions.onCovers,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("cart_covers"),
            )
        }
    }
}

/** "Couverts", "1 couvert" or "4 couverts" (`coversText` of `CartSidebar.vue`). */
@Composable
private fun coversText(covers: Int?): String =
    when (covers) {
        null -> vueText("covers_add")
        1 -> vueText("covers_one")
        else -> vueText("covers_n", "n" to covers)
    }

@Composable
private fun CartTitleRow(
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = cartTitle(state.order, state.tableLabel),
            style = PiliTheme.typography.title,
            color = c.textPrimary,
            modifier = Modifier.weight(1f).testTag("cart_title"),
        )
        if (state.hasMoreActions) {
            PiliButton(
                onClick = actions.onMore,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.padding(horizontal = d.spaceSm).testTag("cart_more"),
            ) {
                Icon(Icons.Filled.MoreHoriz, contentDescription = vueText("cart_more"), tint = c.textPrimary)
                Text(vueText("cart_more"), style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold))
            }
        }
        OpenedAt(state.order)
    }
}
