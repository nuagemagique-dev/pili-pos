package dev.nuagemagique.pilipos.feature.checkout

import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EftPayment
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.TicketDeliveryRequest
import uniffi.pos_ffi.VatSplitRow

/** The three steps of `CheckoutView.vue`. */
internal enum class CheckoutStep { PAY, PROCESSING, DONE }

/** The payment methods offered at the till, in the order of the buttons (Vue `METHODS`). */
internal val TILL_METHODS: List<PaymentMethod> = listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER)

/** How many equal parts a sale can be split in (`v-for="n in 5"`). */
internal const val SPLIT_MAX: Int = 5

/**
 * One part already taken. [cents] is what the part pays (for cash: the part, not the rounded amount handed over),
 * [changeCents] the change handed back for a cash part, fixed when the part was taken.
 */
internal data class TakenPart(
    val method: PaymentMethod,
    val cents: Long,
    val changeCents: Long,
    /** A card part taken on the linked terminal: the approved payment it settles (`EftPayment.id`). */
    val eftPaymentId: Long? = null,
    /** The card brand the terminal gave ("Bancontact"). */
    val brand: String? = null,
    /** The approved terminal card is a meal-voucher card: shown as meal vouchers, the part stays a card. */
    val mealVoucher: Boolean = false,
) {
    /** The method the part is called by: a meal-voucher card is a meal voucher. */
    val displayMethod: PaymentMethod get() = if (mealVoucher) PaymentMethod.MEAL_VOUCHER else method

    /** The card was charged: the part cannot be removed and its amount is fixed. */
    val locked: Boolean get() = eftPaymentId != null
}

/** Whether a chip of the part row is paid, the current part, or one to come. */
internal enum class ChipState { PAID, CURRENT, NEXT }

/** One chip of the part row: the method is null for a part to come. */
internal data class PartChip(
    val state: ChipState,
    val method: PaymentMethod?,
    val amountCents: Long,
)

/** The lines of the recap with what each one shows as its price and the VAT rows, all from the engine. */
internal data class CheckoutSummary(
    val lines: List<CartLine> = emptyList(),
    val amounts: List<LineAmount> = emptyList(),
    val vatRows: List<VatSplitRow> = emptyList(),
    /** Σ of the lines' discounts (zero or negative), from the engine's cart lines; 0 without a discount. */
    val discountCents: Long = 0L,
)

/**
 * The amounts that follow from the order total and the parts taken. None of them is computed in Kotlin: the rest
 * and the rounded cash due come from the engine (`settlePreview`), the shares and the quick amounts from
 * `CashDisplay`.
 */
internal data class CheckoutAmounts(
    /** What is left to pay after the parts taken (Vue `restCents`). */
    val restCents: Long = 0L,
    /** What the current part pays, unrounded (Vue `dueCents`). */
    val dueCents: Long = 0L,
    /** The cash to hand over for the current part, rounded by the engine (Vue `roundedDueCents`). */
    val roundedDueCents: Long = 0L,
    /** The "left to pay" line: rounded when cash is about to pay all of it (Vue `restShownCents`). */
    val restShownCents: Long = 0L,
    /** The equal shares of the total (Vue `shares`). */
    val shares: List<Long> = emptyList(),
    /** The part row, empty when there is a single part. */
    val chips: List<PartChip> = emptyList(),
    /** The likely cash amounts above the rounded due. */
    val quickAmountsCents: List<Long> = emptyList(),
    /** Parts are taken and nothing is left to pay: the sale only waits to be signed (the engine's rest is 0). */
    val allPaid: Boolean = false,
)

/** What the done screen shows; the cart and its total are gone by then, so everything is fixed at the sale. */
internal data class CheckoutDone(
    /** The label of the ticket type, as the engine's ticket gives it. */
    val ticketLabel: String?,
    /** The sale total (the engine order total read when the sale was validated). */
    val totalCents: Long,
    /** The change handed back for the cash part, 0 when there is none. */
    val changeCents: Long,
    /** The FDM id of the signature, or null. */
    val fdmId: String?,
    /** False when the engine reports the FDM was not connected (unsigned ticket). */
    val fdmConnected: Boolean,
    /** How the ticket reached the customer. */
    val delivery: DeliveryOutcome?,
    /** The archive id of the signed event, the key of the print-missing action. */
    val fiscalEventId: Long?,
)

/** What the checkout screen shows. */
@Suppress("LongParameterList")
internal data class CheckoutUiState(
    /** The id of the order of the route. */
    val orderId: Long = -1L,
    /** False while no order is on screen: the screen shows an empty state. */
    val hasOrder: Boolean = false,
    /** True once the load of the route's order was tried (it may have failed). */
    val loadTried: Boolean = false,
    /** The order type, for the title of the recap. */
    val orderType: OrderType? = null,
    /** The table number of a dine-in order. */
    val tableNumber: Int? = null,
    /** The engine total of the order (`Order.totalCents`). */
    val totalCents: Long = 0L,
    /** True for the roles that may take a payment (administrator, cashier). */
    val canTakePayment: Boolean = false,
    val step: CheckoutStep = CheckoutStep.PAY,
    val splitCount: Int = 1,
    val method: PaymentMethod = PaymentMethod.CASH,
    val taken: List<TakenPart> = emptyList(),
    /** The digits typed, read as cents like a payment terminal. */
    val entry: AmountEntryState = AmountEntryState(),
    /** Set by a shortcut or a prefill: the next key replaces the entry instead of appending to it. */
    val fresh: Boolean = false,
    val amounts: CheckoutAmounts = CheckoutAmounts(),
    val summary: CheckoutSummary = CheckoutSummary(),
    /** The refusal text of the last attempt, shown in the footer. */
    val error: String? = null,
    val done: CheckoutDone? = null,
    val mailStatus: MailStatus? = null,
    val deliveryRequest: TicketDeliveryRequest? = null,
    val labelMode: ProductLabelMode = ProductLabelMode.NAME,
    val itemCount: Int = 0,
    /** A card terminal is linked and the till is not in training: card parts can go to the terminal. */
    val eftActive: Boolean = false,
    /** The operator chose to type the card by hand although a terminal is linked ("Saisie manuelle"). */
    val cardManual: Boolean = false,
    /** The order's payments that wait for the terminal's answer: no sale until they are checked. */
    val eftOpen: List<EftPayment> = emptyList(),
) {
    /** The amount being typed, in cents (no arithmetic: the digits are the cents). */
    val entryCents: Long get() = entry.cents

    /** True when a cash part has been taken already. */
    val hasCashPart: Boolean get() = taken.any { it.method == PaymentMethod.CASH }

    /** The index of the part being taken. */
    val partIndex: Int get() = taken.size

    /** True for the part that pays the rest. */
    val isLastPart: Boolean get() = partIndex >= splitCount - 1

    /** True when the cash handed over covers the rounded due. */
    val canPayCash: Boolean get() = entryCents >= amounts.roundedDueCents

    /** The change to give back for the cash handed over (`CashDisplay.changeCents`, never below 0). */
    val changeCents: Long get() = CashDisplay.changeCents(entryCents, amounts.roundedDueCents)

    /** True when a card or meal voucher amount is above what is left (refused by the screen). */
    val exactTooHigh: Boolean get() = method != PaymentMethod.CASH && entryCents > amounts.restCents

    /** The card part being taken goes to the linked terminal (not typed by hand). */
    val viaTerminal: Boolean get() = method == PaymentMethod.CARD && eftActive && !cardManual

    /** A terminal payment waits for its answer: the sale is blocked until it is checked. */
    val eftBlocked: Boolean get() = eftOpen.isNotEmpty()

    /** True when something can be validated: the sale itself when everything is paid, else the current part. */
    val canValidate: Boolean
        get() =
            canTakePayment &&
                hasOrder &&
                step == CheckoutStep.PAY &&
                !eftBlocked &&
                (amounts.allPaid || if (method == PaymentMethod.CASH) canPayCash else entryCents > 0L && !exactTooHigh)

    /** True when the "exact" shortcut is the current entry. */
    val exactActive: Boolean
        get() = fresh && entryCents == if (method == PaymentMethod.CASH) amounts.roundedDueCents else amounts.dueCents

    /** True when [quickCents] is the current entry. */
    fun quickActive(quickCents: Long): Boolean = fresh && entryCents == quickCents

    /** A split in more parts than cents would give a part of 0; only before the first part is taken (Vue `splitAllowed`). */
    fun splitAllowed(parts: Int): Boolean = taken.isEmpty() && (parts == 1 || parts.toLong() <= totalCents)
}

/** What the view model asks of the navigation. */
internal sealed interface CheckoutEffect {
    /** Back to the order screen of the order being paid. */
    data class BackToOrder(
        val orderId: Long,
    ) : CheckoutEffect

    /** The sale is done: on to the tables, for the next customer. */
    data object NewOrder : CheckoutEffect
}
