package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.launch
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus

private val PrintButtonHeight = 64.dp
private const val PRINTING_ALPHA = 0.5f

/** What happened to the ticket, as far as the message line goes. */
enum class DeliveryOutcomeKind { Sent, Queued, Printed }

/**
 * The message of [outcome], or null when there is none: the e-mail was sent, or is queued for a new try, or the
 * paper ticket was printed (in that order, as in `DeliveryOutcomePanel.vue`).
 */
fun deliveryOutcomeKind(outcome: DeliveryOutcome?): DeliveryOutcomeKind? =
    when {
        outcome == null -> null
        outcome.email == EmailStatus.SENT -> DeliveryOutcomeKind.Sent
        outcome.email == EmailStatus.QUEUED -> DeliveryOutcomeKind.Queued
        outcome.paperPrinted -> DeliveryOutcomeKind.Printed
        else -> null
    }

/** The captions of a [DeliveryOutcomePanel]: the message line (null for none) and the print action. */
@Immutable
data class DeliveryOutcomeTexts(
    val message: String?,
    val printMissing: String,
)

/** The Vue captions for [outcome] (`delivery_outcome_*`, `delivery_print_missing`). Needs `ProvideUiText` above. */
@Composable
fun defaultDeliveryOutcomeTexts(outcome: DeliveryOutcome?): DeliveryOutcomeTexts =
    DeliveryOutcomeTexts(
        message =
            when (deliveryOutcomeKind(outcome)) {
                DeliveryOutcomeKind.Sent -> vueText("delivery_outcome_sent", "hint" to (outcome?.recipientHint ?: ""))
                DeliveryOutcomeKind.Queued -> vueText("delivery_outcome_queued")
                DeliveryOutcomeKind.Printed -> vueText("delivery_outcome_printed")
                null -> null
            },
        printMissing = vueText("delivery_print_missing"),
    )

/**
 * `DeliveryOutcomePanel.vue`: shown after a checkout, a refund or a copy. It says what happened to the ticket
 * and, when the customer still needs a paper original ([DeliveryOutcome.needsPaper]), offers the one print
 * action of the done screen. Nothing is drawn while [outcome] is null.
 *
 * The action is never offered twice: while the print runs the button is faded and a second tap does nothing, and
 * once [onPrintMissing] answered true the button is gone (the engine never prints a second original). A refusal
 * keeps the button, so the operator can try again; the caller has already shown it. The caller passes
 * `DeliveryStore.printMissingOriginal` and says through [canPrintMissing] whether the user may take payments
 * (cashier or admin), as the Vue panel hides the button from a waiter.
 *
 * Test tags: `delivery_outcome` (root), `delivery_message`, `delivery_print_missing`.
 */
@Composable
fun DeliveryOutcomePanel(
    outcome: DeliveryOutcome?,
    texts: DeliveryOutcomeTexts,
    canPrintMissing: Boolean,
    onPrintMissing: suspend (fiscalEventId: Long) -> Boolean,
    modifier: Modifier = Modifier,
    fiscalEventId: Long? = outcome?.fiscalEventId,
) {
    if (outcome == null) return
    var printing by remember(fiscalEventId) { mutableStateOf(false) }
    var printed by remember(fiscalEventId) { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    Column(
        modifier = modifier.fillMaxWidth().testTag("delivery_outcome"),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        texts.message?.let { message ->
            Text(
                text = message,
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.textSecondary,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("delivery_message"),
            )
        }
        if (outcome.needsPaper && !printed && canPrintMissing) {
            PrintMissingButton(texts.printMissing, printing) {
                val id = fiscalEventId
                if (id != null && !printing) {
                    printing = true
                    scope.launch {
                        try {
                            printed = onPrintMissing(id)
                        } finally {
                            printing = false
                        }
                    }
                }
            }
        }
    }
}

/** The warning-coloured action (Vue `delivery-print-btn`); a tap while [printing] does nothing. */
@Composable
private fun PrintMissingButton(
    text: String,
    printing: Boolean,
    onClick: () -> Unit,
) {
    val colors = PiliTheme.colors
    val interactionSource = remember { MutableInteractionSource() }
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(PrintButtonHeight)
                .tapScale(interactionSource)
                .alpha(if (printing) PRINTING_ALPHA else 1f)
                .background(colors.accentWarning, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    enabled = !printing,
                    role = Role.Button,
                    onClick = onClick,
                ).defaultMinSize(minHeight = PrintButtonHeight)
                .padding(horizontal = PiliTheme.dimens.spaceMd)
                .testTag("delivery_print_missing"),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = colors.textOnAccent,
            textAlign = TextAlign.Center,
        )
    }
}
