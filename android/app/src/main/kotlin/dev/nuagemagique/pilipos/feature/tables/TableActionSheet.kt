package dev.nuagemagique.pilipos.feature.tables

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val SheetMaxWidth = 520.dp
private val ActionHeight = 80.dp
private val CloseHeight = 56.dp

/**
 * `TableActionSheet.vue` (UX audit T7): the sheet a long press (500 ms) opens on an occupied table card. "Ouvrir"
 * is the usual tap, "Addition" prints the provisional bill, "Déplacer" starts the move mode with this table chosen,
 * "Payer" opens its checkout. "Addition" and "Payer" are only offered to the roles that take payments. A tap outside
 * the card or "Fermer" closes it.
 *
 * Test tags: `table_sheet` (root), `sheet_open`, `sheet_bill`, `sheet_move`, `sheet_pay`, `sheet_close`.
 */
@Composable
internal fun TableActionSheet(
    sheet: TableSheet,
    callbacks: TablesCallbacks,
    modifier: Modifier = Modifier,
) {
    val table = sheet.table
    Box(modifier = modifier.testTag("table_sheet")) {
        PiliDialog(
            title = vueText("table") + " " + table.label,
            onDismissRequest = callbacks.onSheetClose,
            maxWidth = SheetMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("table_sheet_close"),
                    onClick = callbacks.onSheetClose,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.height(CloseHeight).testTag("sheet_close"),
                )
            },
        ) {
            Text(
                text = formatEuro(table.totalCents),
                style = PiliTheme.typography.figure,
                color = PiliTheme.colors.textSecondary,
                modifier = Modifier.testTag("sheet_total"),
            )
            val actions =
                buildList {
                    add(SheetAction("sheet_open", vueText("table_sheet_open"), callbacks.onSheetOpen))
                    if (sheet.canTakePayment) add(SheetAction("sheet_bill", vueText("pre_bill"), callbacks.onSheetBill))
                    add(SheetAction("sheet_move", vueText("transfer_mode"), callbacks.onSheetMove))
                    if (sheet.canTakePayment) add(SheetAction("sheet_pay", vueText("table_sheet_pay"), callbacks.onSheetPay))
                }
            // Two columns, as the Vue grid.
            actions.chunked(2).forEach { row ->
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                    row.forEach { action ->
                        PiliButton(
                            text = action.label,
                            onClick = action.onClick,
                            variant = PiliButtonVariant.Secondary,
                            modifier = Modifier.weight(1f).height(ActionHeight).testTag(action.tag),
                        )
                    }
                    if (row.size == 1) Box(Modifier.weight(1f))
                }
            }
        }
    }
}

private data class SheetAction(
    val tag: String,
    val label: String,
    val onClick: () -> Unit,
)
