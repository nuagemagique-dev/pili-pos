package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val RowHeight = 64.dp
private val Hairline = 1.dp

/**
 * The "Plus" menu of the cart header (`CartMoreMenu.vue`, UX audit C3): change table, the order discount (or its
 * removal) and "customer left without paying", out of the always-visible footer. A row shows only for the roles and
 * orders that have the action; the dialog closes when a row opens its own dialog (the view model replaces it).
 *
 * Test tags: `cart_more_menu` (root), `cart_transfer`, `cart_discount`, `cart_discount_remove`, `cart_unpaid`,
 * `cart_more_close`.
 */
@Composable
internal fun CartMoreDialog(
    dialog: OrderDialog.CartMore,
    callbacks: OrderCallbacks,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    PiliDialog(
        title = vueText("cart_more_title"),
        onDismissRequest = callbacks.onDismiss,
        modifier = modifier.testTag("cart_more_menu"),
        actions = {
            PiliButton(
                text = vueText("cart_more_close"),
                onClick = callbacks.onDismiss,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("cart_more_close"),
            )
        },
    ) {
        Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            if (dialog.canTransfer) MoreRow("transfer_table", "cart_transfer", callbacks.onMoreTransfer)
            if (dialog.canDiscount && !dialog.orderDiscounted) MoreRow("discount_button", "cart_discount", callbacks.onMoreDiscount)
            if (dialog.canDiscount && dialog.orderDiscounted) {
                MoreRow("discount_remove", "cart_discount_remove", callbacks.onMoreDiscountRemove)
            }
            if (dialog.canCloseUnpaid) UnpaidRow(callbacks.onMoreUnpaid)
        }
    }
}

@Composable
private fun MoreRow(
    key: String,
    tag: String,
    onClick: () -> Unit,
) {
    val t = PiliTheme.typography
    PiliButton(
        onClick = onClick,
        variant = PiliButtonVariant.Secondary,
        modifier = Modifier.fillMaxWidth().height(RowHeight).testTag(tag),
    ) {
        Text(vueText(key), style = t.body.copy(fontWeight = t.weightBold))
    }
}

/** Rare and destructive: outlined red, apart from the plain rows (the old footer button, moved here). */
@Composable
private fun UnpaidRow(onClick: () -> Unit) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(RowHeight)
                .border(Hairline, c.accentDanger, RoundedCornerShape(d.radiusMd))
                .clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = d.spaceMd)
                .testTag("cart_unpaid"),
        contentAlignment = Alignment.Center,
    ) {
        Text(text = vueText("unpaid_close"), style = t.body.copy(fontWeight = t.weightBold), color = c.accentDanger)
    }
}
