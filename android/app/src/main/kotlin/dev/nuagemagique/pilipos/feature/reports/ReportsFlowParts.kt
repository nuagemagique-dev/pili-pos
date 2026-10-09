package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

internal val FlowMaxWidth = 900.dp
internal val FlowListMaxHeight = 480.dp
internal val ActionWidth = 200.dp

/** The amounts of the refund screen: each line of the sale for its full quantity, and the preview of the ticked ones. */
internal data class RefundUi(
    /** The engine amount of every line of the sale for its full quantity, by line id. */
    val amounts: Map<Long, LineAmount> = emptyMap(),
    /** The preview of the ticked lines (UI-PD307). */
    val preview: RefundPreview = RefundPreview(),
)

/** The signature in progress: a spinner and the FDM text; nothing dismisses it. */
@Composable
internal fun ProcessingDialog(tag: String) {
    PiliDialog(title = "", onDismissRequest = null, modifier = Modifier.testTag(tag), maxWidth = FlowMaxWidth) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd, Alignment.CenterHorizontally),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Spinner()
            Text(vueText("processing_fdm"), style = PiliTheme.typography.body, color = PiliTheme.colors.textPrimary)
        }
    }
}

@Composable
internal fun ErrorLine(
    text: String,
    tag: String,
) {
    Text(text, style = PiliTheme.typography.body, color = PiliTheme.colors.accentDanger, modifier = Modifier.testTag(tag))
}
