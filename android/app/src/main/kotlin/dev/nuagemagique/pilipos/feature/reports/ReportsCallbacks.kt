package dev.nuagemagique.pilipos.feature.reports

import dev.nuagemagique.pilipos.ui.state.DeliveryStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.RefundableEvent
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest

/** What the operator can do on the day tab. */
internal data class DayCallbacks(
    val onX: () -> Unit = {},
    val onUserX: () -> Unit = {},
    val onZ: () -> Unit = {},
    val onOpenZList: () -> Unit = {},
    val onCloseZList: () -> Unit = {},
    val onReprintZ: (fiscalEventId: Long?) -> Unit = {},
)

/** What the operator can do on the cash tab. */
internal data class CashCallbacks(
    val onDirection: (CashDirection) -> Unit = {},
    val onReason: (String) -> Unit = {},
    val onAmount: (cents: Long) -> Unit = {},
    val onOpenOtherAmount: () -> Unit = {},
    val onCloseOtherAmount: () -> Unit = {},
    val onPickOtherAmount: (cents: Long) -> Unit = {},
    val onOpenDeclaration: () -> Unit = {},
    val onCloseDeclaration: () -> Unit = {},
    val onDeclare: (cents: Long) -> Unit = {},
    val onOpenDrawer: () -> Unit = {},
    val onSubmit: () -> Unit = {},
)

/** What the operator can do on the tickets tab: the copy flow and the refund flow. */
internal data class TicketsCallbacks(
    val onCopy: (fiscalEventId: Long?) -> Unit = {},
    val onCopyConfirm: () -> Unit = {},
    val onCopyClose: () -> Unit = {},
    val onRefund: (RefundableEvent) -> Unit = {},
    /** The "Corriger le paiement" button of a sale row. */
    val onCorrect: (TicketRow) -> Unit = {},
    val onToggleLine: (lineId: Long) -> Unit = {},
    val onQuantity: (lineId: Long, delta: Int) -> Unit = { _, _ -> },
    val onRefundNext: () -> Unit = {},
    val onRefundBack: () -> Unit = {},
    val onRefundConfirm: () -> Unit = {},
    val onRefundClose: () -> Unit = {},
    /** The screen text of the raw error of a copy or a refund the engine did not sign. */
    val describeError: (String?) -> String = { it.orEmpty() },
)

/** What the operator can do in the payment correction dialog. */
internal data class CorrectionCallbacks(
    val onLine: (index: Int) -> Unit = {},
    val onMethod: (PaymentMethod) -> Unit = {},
    val onConfirm: () -> Unit = {},
    val onClose: () -> Unit = {},
    val onPinCancel: () -> Unit = {},
    /** The PIN typed on the pad: the engine corrects the payment, or the refusal is thrown to the pad. */
    val onPin: suspend (String) -> Unit = {},
    val onDone: () -> Unit = {},
    /** The screen text of a refusal thrown by [onPin], and of a refusal code of the engine. */
    val describeThrowable: (Throwable) -> String = { it.message.orEmpty() },
    val describeRaw: (String?) -> String = { it.orEmpty() },
)

/** What the delivery picker of the copy and refund dialogs does. */
internal data class DeliveryCallbacks(
    val onPaper: () -> Unit = {},
    val onEmail: (RequestedMedium, String) -> Unit = { _, _ -> },
    val onPrintMissing: suspend (fiscalEventId: Long) -> Boolean = { false },
)

/** Every action of the reports screen. */
internal data class ReportsCallbacks(
    val onTab: (ReportsTab) -> Unit = {},
    val day: DayCallbacks = DayCallbacks(),
    val cash: CashCallbacks = CashCallbacks(),
    val tickets: TicketsCallbacks = TicketsCallbacks(),
    val correction: CorrectionCallbacks = CorrectionCallbacks(),
    val delivery: DeliveryCallbacks = DeliveryCallbacks(),
)

/** What the delivery picker shows: the mail probe, the choice made and whether a paper original may be printed. */
internal data class ReportsDelivery(
    val mailStatus: MailStatus? = null,
    val request: TicketDeliveryRequest = TicketDeliveryRequest(RequestedMedium.PAPER, null),
    val canPrintMissing: Boolean = false,
)

/**
 * The actions of the screen over [model] and [delivery]. Every copy and refund flow starts from paper and ends
 * back on paper, as `ReportsView.vue` does: [DeliveryStore.reset] when a flow opens and when it closes, and the
 * mail probe is read again when a picker is about to be shown ([DeliveryStore.refreshMailStatus], which the engine
 * caches for a minute).
 */
internal fun reportsCallbacks(
    model: ReportsViewModel,
    delivery: DeliveryStore,
    scope: CoroutineScope,
): ReportsCallbacks {
    val openPicker = {
        delivery.reset()
        scope.launch { delivery.refreshMailStatus() }
        Unit
    }
    return ReportsCallbacks(
        onTab = model::selectTab,
        day = dayCallbacks(model, openPicker),
        cash = cashCallbacks(model),
        tickets = ticketsCallbacks(model, delivery, openPicker),
        correction = correctionCallbacks(model),
        delivery =
            DeliveryCallbacks(
                onPaper = delivery::setPaper,
                onEmail = delivery::setEmail,
                onPrintMissing = delivery::printMissingOriginal,
            ),
    )
}

private fun dayCallbacks(
    model: ReportsViewModel,
    openPicker: () -> Unit,
): DayCallbacks =
    DayCallbacks(
        onX = model::generateX,
        onUserX = model::generateUserX,
        onZ = model::generateZ,
        onOpenZList = model::openZList,
        onCloseZList = model::closeZList,
        onReprintZ = { id ->
            openPicker()
            model.reprintZ(id)
        },
    )

private fun cashCallbacks(model: ReportsViewModel): CashCallbacks =
    CashCallbacks(
        onDirection = model::setCashDirection,
        onReason = model::setCashReason,
        onAmount = model::setCashAmount,
        onOpenOtherAmount = model::openOtherAmount,
        onCloseOtherAmount = model::closeOtherAmount,
        onPickOtherAmount = model::pickOtherAmount,
        onOpenDeclaration = model::openDeclaration,
        onCloseDeclaration = model::closeDeclaration,
        onDeclare = model::declareDrawer,
        onOpenDrawer = model::openDrawer,
        onSubmit = model::submitCash,
    )

private fun ticketsCallbacks(
    model: ReportsViewModel,
    delivery: DeliveryStore,
    openPicker: () -> Unit,
): TicketsCallbacks =
    TicketsCallbacks(
        onCopy = { id ->
            openPicker()
            model.openCopy(id, CopyTargetKind.TICKET)
        },
        onCopyConfirm = { model.confirmCopy(delivery.request.value) },
        onCopyClose = {
            model.closeCopy()
            delivery.reset()
        },
        onRefund = model::openRefund,
        onCorrect = model::openCorrection,
        onToggleLine = model::toggleRefundLine,
        onQuantity = model::adjustRefundQuantity,
        onRefundNext = {
            openPicker()
            model.goToRefundDelivery()
        },
        onRefundBack = model::backToRefundLines,
        onRefundConfirm = { model.refund(delivery.request.value) },
        onRefundClose = {
            model.closeRefund()
            delivery.reset()
        },
        describeError = model::describeRaw,
    )

private fun correctionCallbacks(model: ReportsViewModel): CorrectionCallbacks =
    CorrectionCallbacks(
        onLine = model::pickCorrectionLine,
        onMethod = model::pickCorrectionMethod,
        onConfirm = model::askCorrectionPin,
        onClose = model::closeCorrection,
        onPinCancel = model::cancelCorrectionPin,
        onPin = model::correctPayment,
        onDone = model::correctionDone,
        describeThrowable = model::describeError,
        describeRaw = model::describeRaw,
    )
