package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.VatSplitRow

/** Between two rows of the summary, as `VatSummary.vue` writes it. */
private const val ROW_SEPARATOR = " · "

/**
 * `VatSummary.vue`: the VAT included in the total, per rate, on one line, for example
 * `TVA 21% 1,23 · 6% 0,34`. The [rows] come from the engine (`CartAmounts.vatRows`), the amounts are only
 * formatted here. Nothing is drawn when [rows] is empty. Test tag: `vat_summary`.
 */
@Composable
fun VatSummary(
    rows: List<VatSplitRow>,
    modifier: Modifier = Modifier,
) {
    if (rows.isEmpty()) return
    val label = vueText("vat")
    val line = rows.joinToString(ROW_SEPARATOR) { "${it.ratePercent}% ${formatCents(it.vatCents)}" }
    Text(
        text = "$label $line",
        style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeSm),
        color = PiliTheme.colors.textSecondary,
        textAlign = TextAlign.End,
        modifier = modifier.fillMaxWidth().testTag("vat_summary"),
    )
}
