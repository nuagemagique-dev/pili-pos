package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Tables per row of the picker (`repeat(5, ...)` in `TablePickerDialog.vue`). */
private const val PICKER_COLUMNS = 5

private val PickHeight = 72.dp
private val PickerMaxWidth = 720.dp

/**
 * `TablePickerDialog.vue`: the free tables of the plan, to move an order to one of them. Tapping a table selects it
 * (Volt fill); "Validate" asks for the move with the selected table and stays disabled until one is selected. A tap
 * outside the card or "Back" cancels. [from] is the table the order is on now; [tables] are the free ones.
 *
 * Test tags: `table_picker` (hint), `pick_table_<n>`, `transfer_confirm`, `transfer_cancel`.
 */
@Composable
internal fun TablePickerDialog(
    from: Int,
    tables: List<Int>,
    onConfirm: (Int) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    var selected by rememberSaveable { mutableStateOf<Int?>(null) }
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
                PiliButton(
                    text = vueText("validate"),
                    onClick = { selected?.let(onConfirm) },
                    enabled = selected != null,
                    modifier = Modifier.testTag("transfer_confirm"),
                )
            },
        ) {
            Text(
                text = vueText(if (tables.isEmpty()) "transfer_no_free" else "transfer_pick_target"),
                style = t.body,
                color = c.textSecondary,
            )
            Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                tables.chunked(PICKER_COLUMNS).forEach { row ->
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                        row.forEach { number ->
                            PiliButton(
                                onClick = { selected = number },
                                variant = if (selected == number) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                                modifier = Modifier.weight(1f).height(PickHeight).testTag("pick_table_$number"),
                            ) {
                                Text(
                                    text = number.toString(),
                                    style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
                                    color = if (selected == number) c.textOnAccent else c.textPrimary,
                                )
                            }
                        }
                        repeat(PICKER_COLUMNS - row.size) { Box(Modifier.weight(1f)) }
                    }
                }
            }
        }
    }
}
