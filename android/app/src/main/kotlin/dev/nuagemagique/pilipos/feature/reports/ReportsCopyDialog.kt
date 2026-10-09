package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import dev.nuagemagique.pilipos.ui.components.DeliveryOutcomePanel
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.TicketMediumKeypad
import dev.nuagemagique.pilipos.ui.components.TicketMediumPicker
import dev.nuagemagique.pilipos.ui.components.defaultDeliveryOutcomeTexts
import dev.nuagemagique.pilipos.ui.components.defaultEmailKeypadTexts
import dev.nuagemagique.pilipos.ui.components.defaultTicketMediumPickerTexts
import dev.nuagemagique.pilipos.ui.components.rememberTicketMediumPickerState
import dev.nuagemagique.pilipos.ui.text.vueText

/**
 * The copy dialog (event C): the medium picker first, nothing signed yet; then the signature in progress; then
 * what happened to the copy, with the one print action when the customer still lacks a paper original. A tap on
 * the scrim leaves the picker and the result, never the signature. Test tags: `copy_dialog`, `copy_confirm`,
 * `copy_back`, `copy_processing`, `copy_error`, `copy_done`, plus those of the picker and the outcome panel.
 */
@Composable
internal fun CopyDialog(
    flow: CopyFlow,
    delivery: ReportsDelivery,
    callbacks: ReportsCallbacks,
) {
    val picker = rememberTicketMediumPickerState()
    val title = vueText(if (flow.kind == CopyTargetKind.TICKET) "copy_ticket" else "report_reprint")
    Box(Modifier.fillMaxSize().testTag("copy_dialog")) {
        when (flow.step) {
            CopyStep.PICKER ->
                PiliDialog(
                    title = title,
                    onDismissRequest = callbacks.tickets.onCopyClose,
                    maxWidth = FlowMaxWidth,
                    actions = {
                        PiliButton(
                            text = vueText("back"),
                            onClick = callbacks.tickets.onCopyClose,
                            variant = PiliButtonVariant.Secondary,
                            modifier = Modifier.width(ActionWidth).testTag("copy_back"),
                        )
                        PiliButton(
                            text = vueText("validate"),
                            onClick = callbacks.tickets.onCopyConfirm,
                            modifier = Modifier.width(ActionWidth).testTag("copy_confirm"),
                        )
                    },
                ) {
                    TicketMediumPicker(
                        mailStatus = delivery.mailStatus,
                        request = delivery.request,
                        state = picker,
                        onPaper = callbacks.delivery.onPaper,
                        texts = defaultTicketMediumPickerTexts(),
                    )
                }
            CopyStep.PROCESSING -> ProcessingDialog("copy_processing")
            CopyStep.RESULT -> CopyResultDialog(flow, title, delivery, callbacks)
        }
        TicketMediumKeypad(picker, defaultEmailKeypadTexts(), callbacks.delivery.onEmail, callbacks.delivery.onPaper)
    }
}

@Composable
private fun CopyResultDialog(
    flow: CopyFlow,
    title: String,
    delivery: ReportsDelivery,
    callbacks: ReportsCallbacks,
) {
    val result = flow.result
    PiliDialog(
        title = title,
        onDismissRequest = callbacks.tickets.onCopyClose,
        maxWidth = FlowMaxWidth,
        actions = {
            PiliButton(
                text = vueText("finish"),
                onClick = callbacks.tickets.onCopyClose,
                modifier = Modifier.width(ActionWidth).testTag("copy_done"),
            )
        },
    ) {
        DeliveryOutcomePanel(
            outcome = result?.delivery,
            texts = defaultDeliveryOutcomeTexts(result?.delivery),
            canPrintMissing = delivery.canPrintMissing,
            onPrintMissing = callbacks.delivery.onPrintMissing,
            fiscalEventId = result?.fiscalEventId,
        )
        if (result != null && !result.success) ErrorLine(callbacks.tickets.describeError(result.error), "copy_error")
    }
}
