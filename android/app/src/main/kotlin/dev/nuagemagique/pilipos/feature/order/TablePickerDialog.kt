package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.TableInfo

/** Tables per row of the picker (`repeat(5, ...)` in `TablePickerDialog.vue`). */
private const val PICKER_COLUMNS = 5

private const val STATUS_FREE = "free"

private val PickHeight = 72.dp
private val PickerMaxWidth = 720.dp

/**
 * `TablePickerDialog.vue` (UX audit C8): every other table of the plan, to move the open order. A free table is one
 * tap and [onPick] moves the order at once (no "Validate"); a busy table is dimmed (a disabled button), cannot be
 * tapped and shows its total.
 * A tap outside the card or "Back" cancels. [from] is the table the order is on now; [tables] are the others.
 *
 * Test tags: `table_picker` (root), `pick_table_<n>`, `transfer_cancel`.
 */
@Composable
internal fun TablePickerDialog(
    from: Int,
    tables: List<TableInfo>,
    onPick: (Int) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Box(modifier = modifier.testTag("table_picker")) {
        PiliDialog(
            title = vueText("transfer_table") + " · " + vueText("table") + " " + from,
            onDismissRequest = onCancel,
            maxWidth = PickerMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("transfer_cancel"),
                )
            },
        ) {
            Text(
                text = vueText(if (tables.any { it.status == STATUS_FREE }) "transfer_pick_target" else "transfer_no_free"),
                style = t.body,
                color = c.textSecondary,
            )
            Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                tables.chunked(PICKER_COLUMNS).forEach { row ->
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                        row.forEach { table ->
                            val free = table.status == STATUS_FREE
                            PiliButton(
                                onClick = { onPick(table.tableNumber) },
                                variant = PiliButtonVariant.Secondary,
                                enabled = free,
                                modifier = Modifier.weight(1f).height(PickHeight).testTag("pick_table_${table.tableNumber}"),
                            ) {
                                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                    Text(
                                        text = table.tableNumber.toString(),
                                        style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
                                        color = c.textPrimary,
                                    )
                                    if (!free) {
                                        Text(
                                            text = formatEuro(table.totalCents),
                                            style = t.body.copy(fontSize = t.sizeXs),
                                            color = c.textSecondary,
                                            modifier = Modifier.testTag("pick_total_${table.tableNumber}"),
                                        )
                                    }
                                }
                            }
                        }
                        repeat(PICKER_COLUMNS - row.size) { Box(Modifier.weight(1f)) }
                    }
                }
            }
        }
    }
}
