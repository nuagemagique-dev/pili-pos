package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val PayHeight = 80.dp
private val ActionHeight = 64.dp
private val OutlineWidth = 2.dp
private const val DISABLED_ALPHA = 0.4f

/** The pill on the Volt button: black at 20 % (`rgba(0, 0, 0, 0.2)` of `.kitchen-pill`). */
private val PillFill = Color.Black.copy(alpha = 0.2f)

/** PAYER as an outlined secondary while lines wait for the kitchen (C4): it still shows the total. */
@Composable
internal fun PaySecondaryButton(
    state: OrderCartState,
    actions: OrderCartActions,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val enabled = state.rows.isNotEmpty()
    Row(
        modifier =
            modifier
                .height(ActionHeight)
                .alpha(if (enabled) 1f else DISABLED_ALPHA)
                .border(OutlineWidth, c.textSecondary, RoundedCornerShape(d.radiusMd))
                .clickable(enabled = enabled, role = Role.Button, onClick = actions.onPay)
                .padding(horizontal = d.spaceMd)
                .testTag("cart_pay_secondary"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(vueText("pay"), style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightBold), color = c.textPrimary)
        Text(
            text = formatEuro(state.order?.totalCents ?: 0L),
            style = t.figure.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
            color = c.textPrimary,
            modifier = Modifier.testTag("cart_pay_secondary_amount"),
        )
    }
}

/** "Send to kitchen" as the Volt primary while lines wait (C4), with a pill counting them ("2 lignes"). */
@Composable
internal fun KitchenPrimaryButton(
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val lines = state.sendableLineCount
    PiliButton(
        onClick = actions.onSend,
        enabled = state.unsentCount > 0 && !state.sending,
        modifier = Modifier.fillMaxWidth().height(PayHeight).testTag("cart_send_primary"),
    ) {
        Row(Modifier.weight(1f), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(vueText("send_to_kitchen"), style = t.body.copy(fontSize = t.sizeXl, fontWeight = t.weightHeavy), color = c.textOnAccent)
            if (lines > 0) {
                Box(
                    modifier = Modifier.background(PillFill, CircleShape).padding(horizontal = d.spaceMd, vertical = d.spaceXs),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = if (lines == 1) vueText("send_lines_one") else vueText("send_lines_other", "n" to lines),
                        style = t.figure.copy(fontSize = t.sizeLg),
                        color = c.textOnAccent,
                        modifier = Modifier.testTag("cart_send_pill"),
                    )
                }
            }
        }
    }
}
