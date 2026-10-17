package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.PinDialog
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadTexts
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.SalePaymentLine

/**
 * The payment correction dialog (`PaymentCorrectionDialog.vue`, event F): the payment lines of a sale, the line
 * and the method picked, the summary, and the manager PIN that confirms it. A card the linked terminal took is
 * shown with a terminal mark and cannot be picked; a sale the engine says cannot be corrected shows why and
 * nothing to confirm. Test tags: `correction_dialog`, `correction_loading`, `correction_refusal`,
 * `correction_corrected`, `correction_line_<index>`, `correction_terminal_<index>`, `correction_method_<method>`,
 * `correction_summary`, `correction_confirm`, `correction_back`, plus those of the PIN dialog.
 */
@Composable
internal fun CorrectionDialog(
    flow: CorrectionFlow,
    userName: String,
    callbacks: CorrectionCallbacks,
) {
    Box(Modifier.fillMaxSize().testTag("correction_dialog")) {
        PiliDialog(
            title = vueText("reports_pc_title") + " · #" + flow.ticketNumber,
            onDismissRequest = callbacks.onClose,
            maxWidth = FlowMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = callbacks.onClose,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.width(ActionWidth).testTag("correction_back"),
                )
                if (flow.payments?.correctable == true) {
                    PiliButton(
                        text = vueText("reports_pc_confirm"),
                        onClick = callbacks.onConfirm,
                        enabled = flow.canConfirm,
                        modifier = Modifier.width(ActionWidth).testTag("correction_confirm"),
                    )
                }
            },
        ) {
            CorrectionBody(flow, callbacks)
        }
        if (flow.askPin) CorrectionPin(userName, callbacks)
    }
}

@Composable
private fun CorrectionBody(
    flow: CorrectionFlow,
    callbacks: CorrectionCallbacks,
) {
    val payments = flow.payments
    when {
        flow.loadFailure != null -> ErrorLine(flow.loadFailure, "correction_refusal")
        payments == null ->
            Text(
                "…",
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.textSecondary,
                modifier = Modifier.testTag("correction_loading"),
            )
        !payments.correctable -> NotCorrectable(payments.refusal, payments.corrected, callbacks)
        else -> CorrectionChoices(flow, callbacks)
    }
}

/** Why the sale cannot be corrected, and the correction it already has. */
@Composable
private fun NotCorrectable(
    refusal: String?,
    corrected: uniffi.pos_ffi.CorrectionDone?,
    callbacks: CorrectionCallbacks,
) {
    val text = if (refusal != null) callbacks.describeRaw(refusal) else vueText("reports_pc_not_possible")
    ErrorLine(text, "correction_refusal")
    if (corrected != null) {
        Text(
            correctionMark(corrected),
            style = PiliTheme.typography.body,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.testTag("correction_corrected"),
        )
    }
}

@Composable
private fun CorrectionChoices(
    flow: CorrectionFlow,
    callbacks: CorrectionCallbacks,
) {
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val c = PiliTheme.colors
    Column(verticalArrangement = Arrangement.spacedBy(d.spaceMd)) {
        Text(vueText("reports_pc_pick_line"), style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
        Column(verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            flow.lines.forEach { line -> PaymentLineChoice(line, selected = flow.lineIndex == line.index, onPick = callbacks.onLine) }
        }
        if (flow.lines.any { it.inputMethod == CorrectionFlow.INPUT_AUTOMATIC }) {
            Text(vueText("reports_pc_terminal_note"), style = t.body, color = c.textSecondary)
        }
        Text(vueText("reports_pc_pick_method"), style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            val current = flow.picked?.let { methodOf(it.paymentType) }
            CORRECTION_METHODS.forEach { method ->
                ChoiceButton(
                    text = vueText(methodKey(method).orEmpty()),
                    selected = flow.method == method,
                    onClick = { callbacks.onMethod(method) },
                    enabled = flow.picked != null && method != current,
                    modifier = Modifier.weight(1f).testTag("correction_method_${method.name.lowercase()}"),
                )
            }
        }
        correctionSummary(flow)?.let {
            Text(
                it,
                style = t.title,
                color = c.textPrimary,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth().testTag("correction_summary"),
            )
        }
    }
}

@Composable
private fun PaymentLineChoice(
    line: SalePaymentLine,
    selected: Boolean,
    onPick: (Int) -> Unit,
) {
    val locked = line.inputMethod == CorrectionFlow.INPUT_AUTOMATIC
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ChoiceButton(
            text = lineCaption(line),
            selected = selected,
            onClick = { onPick(line.index) },
            enabled = !locked,
            modifier = Modifier.weight(1f).testTag("correction_line_${line.index}"),
        )
        if (locked) {
            Text(
                vueText("reports_pc_terminal"),
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.textSecondary,
                modifier = Modifier.testTag("correction_terminal_${line.index}"),
            )
        }
    }
}

/** "Espèces · 52,00 €" (and the card brand when there is one). */
@Composable
private fun lineCaption(line: SalePaymentLine): String {
    val base = vueText("reports_pc_line", "method" to nameOf(line.paymentType), "amount" to formatCents(line.amountCents))
    return if (line.brand.isNullOrEmpty()) base else base + " · " + line.brand
}

/** "Espèces 52,00 € → Carte 52,00 €" once a line and a method are picked. */
@Composable
private fun correctionSummary(flow: CorrectionFlow): String? {
    val line = flow.picked
    val to = flow.method
    return if (line == null || to == null) {
        null
    } else {
        vueText(
            "reports_pc_summary",
            "from" to nameOf(line.paymentType),
            "to" to vueText(methodKey(to).orEmpty()),
            "amount" to formatCents(line.amountCents),
        )
    }
}

/** "Paiement corrigé : Espèces → Carte". */
@Composable
internal fun correctionMark(done: uniffi.pos_ffi.CorrectionDone): String =
    vueText("reports_pc_corrected", "from" to nameOf(done.fromType), "to" to nameOf(done.toType))

@Composable
private fun nameOf(type: String): String = methodOf(type)?.let { methodKey(it) }?.let { vueText(it) } ?: type

/** The manager PIN (`PinDialog.vue` with the hint `reports_pc_pin_hint`). */
@Composable
private fun CorrectionPin(
    userName: String,
    callbacks: CorrectionCallbacks,
) {
    val hint = vueText("reports_pc_pin_hint")
    val texts =
        PinPadTexts(
            title = vueText("pin_enter"),
            hint = if (userName.isBlank()) hint else userName + "\n" + hint,
            enterPrompt = vueText("pin_enter"),
            confirmPrompt = vueText("pin_confirm"),
            mismatch = vueText("pin_mismatch"),
            cancel = vueText("back"),
            clearLabel = uiText(R.string.ui_common_numpad_clear),
            backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
        )
    PinDialog(
        mode = PinMode.Verify,
        texts = texts,
        onSubmit = callbacks.onPin,
        describeError = callbacks.describeThrowable,
        onDone = callbacks.onDone,
        onCancel = callbacks.onPinCancel,
    )
}
