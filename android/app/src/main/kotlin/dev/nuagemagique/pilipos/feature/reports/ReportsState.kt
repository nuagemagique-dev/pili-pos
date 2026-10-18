package dev.nuagemagique.pilipos.feature.reports

import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.CopyResult
import uniffi.pos_ffi.CopyableTicket
import uniffi.pos_ffi.CorrectionDone
import uniffi.pos_ffi.GiftVoucherView
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.RefundableEvent
import uniffi.pos_ffi.ReportResult
import uniffi.pos_ffi.SalePaymentLine
import uniffi.pos_ffi.SalePayments
import uniffi.pos_ffi.UnsettledCardPayment
import uniffi.pos_ffi.ZReportSummary

/** The tabs of the reports screen (`Tab` of `ReportsView.vue`, UX guide 2.1): four, five with gift vouchers on. */
internal enum class ReportsTab {
    /** X, user X and Z reports and the Z history. */
    DAY,

    /** Cash in and out, the drawer count and the drawer opening. */
    CASH,

    /** Copy and refund from one list of tickets. */
    TICKETS,

    /** The non-fiscal sales report (by product, by hour, concordance with the Z). */
    SALES,

    /** Gift vouchers: lookup by code, and for a manager the list with the duplicate. */
    VOUCHERS,
}

/** The statuses the manager's list filters on, `null` being all of them (`FILTERS` of `GiftVouchersReport.vue`). */
internal val VOUCHER_FILTERS: List<String?> = listOf(null, "active", "used", "expired", "cancelled")

/**
 * The gift vouchers tab (`GiftVouchersReport.vue`): the lookup of a code (any role) and, for a manager, the list.
 * [enabled] is the setting; the tab is offered only then. Codes are masked everywhere.
 */
internal data class VouchersState(
    val enabled: Boolean = false,
    val manager: Boolean = false,
    /** The digits of the code typed for the lookup. */
    val digits: String = "",
    val found: GiftVoucherView? = null,
    val lookupError: String? = null,
    val busy: Boolean = false,
    /** The status filter of the list; null is all. */
    val filter: String? = null,
    val list: List<GiftVoucherView> = emptyList(),
    val listError: String? = null,
    /** The voucher whose movements are open. */
    val open: Long? = null,
)

/** The report a button of the day tab generates. */
internal enum class ReportKind {
    /** The mid-day X report. */
    X,

    /** The X report of the operator. */
    USER_X,

    /** The end-of-day Z report. */
    Z,
}

/**
 * The answer of the last report, kept for the result panel.
 *
 * [failure] is the screen text of a refusal (the engine answered `success = false`, or the call failed); it is
 * null when the report was generated.
 */
internal data class ReportOutcome(
    val kind: ReportKind,
    val result: ReportResult?,
    val failure: String? = null,
) {
    /** True when the report was generated. */
    val generated: Boolean get() = failure == null && result?.success == true
}

/**
 * The label of a report in the toast and the result panel: `X`, `X util.` or `Z #<no>` (`Z` without a number).
 * Text only, no amount.
 */
internal fun reportLabel(
    kind: ReportKind,
    reportNo: Long?,
): String =
    when (kind) {
        ReportKind.X -> "X"
        ReportKind.USER_X -> "X util."
        ReportKind.Z -> if (reportNo != null && reportNo != 0L) "Z #$reportNo" else "Z"
    }

/** The day tab. */
internal data class DayState(
    /** The Z reports, newest first (the history). */
    val zHistory: List<ZReportSummary> = emptyList(),
    /** True when the end-of-day Z is due (`needs_z_report`). */
    val zPending: Boolean = false,
    /** Card payments taken on the terminal that no sale records yet: told at the Z, which they never block. */
    val unsettled: List<UnsettledCardPayment> = emptyList(),
    /** The report being generated, or null: while set, every report button is disabled. */
    val loadingKind: ReportKind? = null,
    /** True after the first tap on the Z button; the second tap within [Z_ARM_WINDOW_MS] generates it. */
    val zArmed: Boolean = false,
    /** True while the Z history dialog is open. */
    val zListOpen: Boolean = false,
    /** The last report answered, or null. */
    val lastReport: ReportOutcome? = null,
) {
    /** The newest Z report, or null. */
    val lastZ: ZReportSummary? get() = zHistory.firstOrNull()

    /** True while a report is being generated. */
    val busy: Boolean get() = loadingKind != null

    companion object {
        /** How long the Z button stays armed, in milliseconds (5 s in `ReportsView.vue`). */
        const val Z_ARM_WINDOW_MS: Long = 5000L
    }
}

/** Direction of the cash form: money put in the drawer or taken out of it. */
internal enum class CashDirection {
    IN,
    OUT,
}

/** The answer of the last cash command, kept for the screen. */
internal data class CashOutcome(
    val success: Boolean,
    /** The screen text of the refusal, null when the command was recorded. */
    val failure: String? = null,
)

/** The cash tab. */
internal data class CashState(
    val direction: CashDirection = CashDirection.IN,
    /** The amount as entered (preset or keypad), in cents; 0 means nothing entered. */
    val amountCents: Long = 0L,
    /** The reason key (`fond_de_caisse`, `retrait`, `pourboire`). */
    val reason: String = CashRules.reasonsOf(CashDirection.IN).first(),
    /** True while a cash movement or a declaration is being recorded. */
    val loading: Boolean = false,
    /** The last answer, or null. */
    val lastOutcome: CashOutcome? = null,
    /** True while the "other amount" keypad is open (`otherAmountOpen` of `ReportsView.vue`). */
    val otherAmountOpen: Boolean = false,
    /** True while the drawer count keypad is open (`declarationOpen` of `ReportsView.vue`). */
    val declarationOpen: Boolean = false,
) {
    /** The reasons that make sense for the chosen direction. */
    val reasons: List<String> get() = CashRules.reasonsOf(direction)

    /** True when the amount is set and is not one of the preset buttons ("Autre" is shown as selected). */
    val custom: Boolean get() = amountCents > 0L && amountCents !in CashRules.PRESETS_CENTS

    /** True when the in/out form can be sent: an amount above zero and a reason. */
    val canSubmit: Boolean get() = amountCents > 0L && reason.isNotEmpty() && !loading
}

/** What the cash tab knows by heart (`ReportsView.vue`). */
internal object CashRules {
    /** The amount buttons: 10, 20, 50 and 100 euros. */
    val PRESETS_CENTS: List<Long> = listOf(PRESET_10_CENTS, PRESET_20_CENTS, PRESET_50_CENTS, PRESET_100_CENTS)

    private const val PRESET_10_CENTS: Long = 1000L
    private const val PRESET_20_CENTS: Long = 2000L
    private const val PRESET_50_CENTS: Long = 5000L
    private const val PRESET_100_CENTS: Long = 10000L

    /** The reason of a drawer count (Basic Set M133). */
    const val DECLARATION_REASON: String = "declaration_tiroir"

    /** The reasons offered for [direction]. */
    fun reasonsOf(direction: CashDirection): List<String> =
        when (direction) {
            CashDirection.IN -> listOf("fond_de_caisse", "pourboire")
            CashDirection.OUT -> listOf("retrait", "pourboire")
        }
}

/** The kind label of a ticket row (`copyKind` of `ReportsView.vue`). */
internal enum class TicketKind {
    TRAINING,
    PRE_BILL,
    REFUND,
    SALE,
}

/**
 * One line of the tickets list: a ticket that can be copied, refunded or both.
 *
 * [copyable] is set when the ticket is in `list_copyable_tickets`, [refundable] when it is in
 * `list_refundable_events`.
 */
internal data class TicketRow(
    val fiscalEventId: Long,
    val ticketNumber: Long,
    val tableNumber: Int?,
    val totalCents: Long,
    val createdAt: String,
    val kind: TicketKind,
    val copyable: CopyableTicket?,
    val refundable: RefundableEvent?,
) {
    /**
     * True for a sale ticket whose payment may be corrected (`isSaleRow` of `ReportsView.vue`): a positive N or T
     * ticket, or a refundable sale. The engine decides whether it really can.
     */
    val isSale: Boolean
        get() {
            if (totalCents <= 0L) return false
            return if (copyable != null) {
                copyable.eventLabel == LABEL_SALE || copyable.eventLabel == LABEL_TRAINING_SALE
            } else {
                refundable != null
            }
        }

    private companion object {
        const val LABEL_SALE = "N"
        const val LABEL_TRAINING_SALE = "T"
    }
}

/** The tickets tab. */
internal data class TicketsState(
    /** True once the lists were loaded at least once (a failed load counts: it shows the empty list). */
    val loaded: Boolean = false,
    /** True while the lists load. */
    val loading: Boolean = false,
    val copyable: List<CopyableTicket> = emptyList(),
    val refundable: List<RefundableEvent> = emptyList(),
    /** Both lists in one, newest first. */
    val rows: List<TicketRow> = emptyList(),
    /** True for a manager: the payment correction is offered on the sale rows. */
    val canCorrect: Boolean = false,
    /** The correction each sale already has, by fiscal event id (read for a manager, best effort). */
    val corrections: Map<Long, CorrectionDone> = emptyMap(),
)

/** What a copy is a copy of. */
internal enum class CopyTargetKind {
    /** A Z report, from the history. */
    Z,

    /** A customer ticket, from the tickets tab. */
    TICKET,
}

/** The step of the copy flow. */
internal enum class CopyStep {
    /** The operator chooses the medium; nothing is signed yet. */
    PICKER,

    /** The copy (event C) is being signed. */
    PROCESSING,

    /** The engine answered: [CopyFlow.result]. */
    RESULT,
}

/** The copy flow (event C). */
internal data class CopyFlow(
    val fiscalEventId: Long,
    val kind: CopyTargetKind,
    val step: CopyStep = CopyStep.PICKER,
    /** The engine's answer, with its print and delivery outcomes; set in the step [CopyStep.RESULT]. */
    val result: CopyResult? = null,
)

/** The step of the refund flow. */
internal enum class RefundStep {
    /** The operator chooses the sale to refund among the refundable events. */
    CHOOSE_EVENT,

    /** The operator ticks the lines to give back and steps their quantity. */
    CHOOSE_LINES,

    /** The operator chooses how the customer receives the refund ticket. */
    DELIVERY,

    /** The refund is being signed. */
    PROCESSING,

    /** The engine answered: [RefundFlow.result]. */
    RESULT,
}

/** A ticked line of the refund with the quantity to give back (1 up to the quantity of the line). */
internal data class RefundSelection(
    val line: OrderLine,
    val quantity: Int,
)

/**
 * The refund flow.
 *
 * No total is computed here (UI-PD307): [selected] gives the lines with their quantity, the screen builds the
 * VAT lines from them and asks the engine helpers for the amounts.
 */
internal data class RefundFlow(
    val step: RefundStep = RefundStep.CHOOSE_EVENT,
    /** The sale being refunded, null in the step [RefundStep.CHOOSE_EVENT]. */
    val event: RefundableEvent? = null,
    /** The lines of that sale, as the engine lists them. */
    val lines: List<OrderLine> = emptyList(),
    /** The ticked lines: line id to quantity, in the order they were ticked. */
    val checked: Map<Long, Int> = emptyMap(),
    /** The engine's answer, with its print and delivery outcomes; set in the step [RefundStep.RESULT]. */
    val result: CheckoutResult? = null,
) {
    /** The ticked lines with their quantity, in the order they were ticked. */
    val selected: List<RefundSelection>
        get() = checked.mapNotNull { (id, quantity) -> lines.firstOrNull { it.id == id }?.let { RefundSelection(it, quantity) } }

    /** True when at least one line is ticked: the delivery step can open. */
    val hasSelection: Boolean get() = checked.isNotEmpty()
}

/**
 * The payment correction flow (event F): the payment lines of a sale, the line and the method picked, and the
 * manager PIN that confirms it.
 */
internal data class CorrectionFlow(
    val fiscalEventId: Long,
    val ticketNumber: Long,
    /** The engine's answer (`list_sale_payments`), null while it loads. */
    val payments: SalePayments? = null,
    /** The screen text of a failed read, null otherwise. */
    val loadFailure: String? = null,
    /** The picked payment line (its index in the sale), or null. */
    val lineIndex: Int? = null,
    /** The method the payment really was made with, or null. */
    val method: PaymentMethod? = null,
    /** True while the PIN dialog is open. */
    val askPin: Boolean = false,
) {
    /** The lines that can be shown as choices: payments, not the cash rounding. */
    val lines: List<SalePaymentLine> get() = payments?.lines.orEmpty().filter { it.amountType == AMOUNT_PAYMENT }

    /** The picked line, or null. */
    val picked: SalePaymentLine? get() = lines.firstOrNull { it.index == lineIndex }

    /** True when a line and a method are picked: the confirm button is enabled. */
    val canConfirm: Boolean get() = picked != null && method != null

    companion object {
        /** `amountType` of a payment line (the other one is the rounding). */
        const val AMOUNT_PAYMENT: String = "PAYMENT"

        /** `inputMethod` of a card the linked terminal took: never corrected. */
        const val INPUT_AUTOMATIC: String = "AUTOMATIC"
    }
}

/** The method a payment line of `paymentType` belongs to; null for a type the till does not take. */
internal fun methodOf(paymentType: String): PaymentMethod? =
    when {
        paymentType == "CASH" -> PaymentMethod.CASH
        paymentType.startsWith("CARD") -> PaymentMethod.CARD
        paymentType == "CHEQUE_MEAL" -> PaymentMethod.MEAL_VOUCHER
        else -> null
    }

/** The methods a payment can be corrected to, in the order of the buttons. */
internal val CORRECTION_METHODS: List<PaymentMethod> = listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER)

/** The Vue key of the name of [method] in the correction dialog; null for the app, online and gift voucher methods. */
internal fun methodKey(method: PaymentMethod): String? =
    when (method) {
        PaymentMethod.CASH -> "reports_pc_method_cash"
        PaymentMethod.CARD -> "reports_pc_method_card"
        PaymentMethod.MEAL_VOUCHER -> "reports_pc_method_meal_voucher"
        PaymentMethod.APP, PaymentMethod.ONLINE, PaymentMethod.GIFT_VOUCHER -> null
    }

/** Everything the reports screen draws. */
internal data class ReportsState(
    val tab: ReportsTab = ReportsTab.DAY,
    val day: DayState = DayState(),
    val cash: CashState = CashState(),
    val tickets: TicketsState = TicketsState(),
    val vouchers: VouchersState = VouchersState(),
    /** The copy flow, or null when no copy is in progress. */
    val copyFlow: CopyFlow? = null,
    /** The refund flow, or null when no refund is in progress. */
    val refundFlow: RefundFlow? = null,
    /** The payment correction flow, or null when none is in progress. */
    val correctionFlow: CorrectionFlow? = null,
    /** True when an operator is logged in; without one the screen shows an in-place empty state (PD205). */
    val signedIn: Boolean = false,
    /** The name of the operator, for the PIN dialog of the payment correction. */
    val userName: String = "",
)
