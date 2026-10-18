package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.Delivery

private val RetryWidth = 200.dp

/** What the operator can do in the Peppol panel. */
internal data class PeppolCallbacks(
    val onRetry: (deliveryId: Long) -> Unit = {},
)

/**
 * The Peppol panel of the settings screen (`PeppolPanel.vue`): the provider and the sender id, the queue counts
 * and the parked deliveries with their reason and a Réessayer button. Drawn for a manager only, and only when the
 * installation sends invoices over Peppol. Test tags: `peppol_section`, `peppol_provider`, `peppol_sender`,
 * `peppol_queued`, `peppol_accepted`, `peppol_failed`, `peppol_none_parked`, `peppol_parked_<id>`,
 * `peppol_retry_<id>`, `peppol_error`.
 */
@Composable
internal fun PeppolSection(
    state: PeppolUiState,
    callbacks: PeppolCallbacks,
    modifier: Modifier = Modifier,
) {
    val panel = state.panel
    if (!state.visible || panel == null) return
    val colors = PiliTheme.colors
    Panel(modifier = modifier.testTag("peppol_section")) {
        PanelTitle(vueText("peppol_title"))
        if (state.error != null) {
            Text(state.error, style = PiliTheme.typography.body, color = colors.accentDanger, modifier = Modifier.testTag("peppol_error"))
        }
        InfoRow(vueText("peppol_provider"), panel.provider.orEmpty(), Modifier.testTag("peppol_provider"))
        InfoRow(vueText("peppol_sender"), panel.senderId.orEmpty(), Modifier.testTag("peppol_sender"))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            Count(
                panel.queued,
                "peppol_queued",
                colors.textPrimary,
                Modifier.weight(1f).semantics(mergeDescendants = true) {}.testTag("peppol_queued"),
            )
            Count(
                panel.accepted,
                "peppol_accepted",
                colors.accentSuccess,
                Modifier.weight(1f).semantics(mergeDescendants = true) {
                }.testTag("peppol_accepted"),
            )
            Count(
                panel.failed,
                "peppol_failed",
                if (panel.failed > 0L) colors.accentDanger else colors.textPrimary,
                Modifier.weight(
                    1f,
                ).semantics(mergeDescendants = true) {
                }.testTag("peppol_failed"),
            )
        }
        PanelTitle(vueText("peppol_parked_title"))
        val parked = state.parked
        if (parked.isEmpty()) {
            PanelText(vueText("peppol_none_parked"), Modifier.testTag("peppol_none_parked"))
        } else {
            Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                parked.forEach { ParkedRow(it, state.retrying == null, callbacks) }
            }
        }
    }
}

@Composable
private fun Count(
    value: Long,
    labelKey: String,
    color: Color,
    modifier: Modifier,
) {
    Column(
        modifier =
            modifier.background(
                PiliTheme.colors.bgSurface,
                RoundedCornerShape(PiliTheme.dimens.radiusMd),
            ).padding(PiliTheme.dimens.spaceMd),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(value.toString(), style = PiliTheme.typography.title, color = color, textAlign = TextAlign.Center)
        Text(vueText(labelKey), style = PiliTheme.typography.label, color = PiliTheme.colors.textSecondary, textAlign = TextAlign.Center)
    }
}

@Composable
private fun ParkedRow(
    delivery: Delivery,
    canRetry: Boolean,
    callbacks: PeppolCallbacks,
) {
    val colors = PiliTheme.colors
    val t = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .padding(horizontal = PiliTheme.dimens.spaceMd, vertical = PiliTheme.dimens.spaceSm)
                .semantics(mergeDescendants = true) {}.testTag("peppol_parked_${delivery.id}"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(
                "${delivery.invoiceNo}  ${delivery.customerEndpointId}",
                style = t.figure.copy(fontWeight = t.weightBold),
                color = colors.textPrimary,
            )
            Text(vueText("peppol_reason", "reason" to (delivery.errorMsg ?: "-")), style = t.body, color = colors.accentDanger)
            val parkedAt = delivery.parkedAt?.let { " · " + PosDateTime.formatTimestamp(it, PosDateTime::formatDayTime) }.orEmpty()
            Text(vueText("peppol_attempts", "n" to delivery.attempts) + parkedAt, style = t.label, color = colors.textSecondary)
        }
        ActionButton(
            vueText("peppol_retry"),
            { callbacks.onRetry(delivery.id) },
            Modifier.width(RetryWidth).testTag("peppol_retry_${delivery.id}"),
            enabled = canRetry,
            variant = PiliButtonVariant.Secondary,
        )
    }
}
