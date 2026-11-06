package dev.nuagemagique.pilipos.feature.checkout

import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.state.DishUnits
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EftPayment
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.GiftVoucherPrint
import uniffi.pos_ffi.GiftVoucherView
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.TicketDeliveryRequest
import uniffi.pos_ffi.VatSplitRow

/** The three steps of `CheckoutView.vue`. */
internal enum class CheckoutStep { PAY, PROCESSING, DONE }

/** The payment methods the till knows, in the order of the buttons (Vue `ALL_METHODS`). */
internal val TILL_METHODS: List<PaymentMethod> =
    listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER, PaymentMethod.GIFT_VOUCHER)

/**
 * The state of the gift voucher dialog (`GiftVoucherDialog.vue`): the [digits] of the code typed, the voucher the
 * engine [found] for them, the text of a refusal and a lookup running.
 */
internal data class VoucherDialogState(
    val digits: String = "",
    val found: GiftVoucherView? = null,
    val error: String? = null,
    val busy: Boolean = false,
)

/** The equal parts with a button of their own: 1 to 4 (the Vue `SPLIT_QUICK_MAX`). */
internal const val SPLIT_QUICK_MAX: Int = 4

/** "Autre": any number of equal parts, 2 to 20, on a pad (the Vue `SPLIT_OTHER_MIN` and `SPLIT_OTHER_MAX`). */
internal const val SPLIT_OTHER_MIN: Int = 2
internal const val SPLIT_OTHER_MAX: Int = 20

/** How the bill is shared: in equal parts, or by dishes (UX audit P3). */
internal enum class SplitMode { EQUAL, DISHES }

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
    /** A gift voucher part: the voucher it redeems, and its masked code. */
    val giftVoucherId: Long? = null,
    val codeMasked: String? = null,
    /** The cash handed over for a cash part (shown in the recap with its change). */
    val givenCents: Long = 0L,
    /** The cart units this part pays when the sale is split by dishes; never sent to the engine. */
    val units: List<DishUnits> = emptyList(),
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
    /** By dishes: every unit not paid yet is checked, so this part pays the whole rest. */
    val everythingPicked: Boolean = false,
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
    /** What became of the gift voucher slips (vouchers sold, balances left); empty for a sale without vouchers. */
    val voucherPrints: List<GiftVoucherPrint> = emptyList(),
) {
    /**
     * Something must be read or decided, so the done step does not leave by itself (`doneNeedsAttention` of the Vue
     * view): the paper ticket was not printed, the digital ticket waits in the outbox, the FDM was not connected
     * (unsigned ticket), or a gift voucher slip has a note.
     */
    val needsAttention: Boolean
        get() =
            !fdmConnected ||
                delivery?.needsPaper == true ||
                delivery?.email == EmailStatus.QUEUED ||
                voucherPrints.isNotEmpty()
}

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
    /** How that table is written: "A-19" in a zone, "19" without one (UX audit T1). */
    val tableLabel: String? = null,
    /** The covers (guests) of a table order (UX audit T5); null when none were set or for another order. */
    val covers: Int? = null,
    /** The engine total of the order (`Order.totalCents`). */
    val totalCents: Long = 0L,
    /** True for the roles that may take a payment (administrator, cashier). */
    val canTakePayment: Boolean = false,
    val step: CheckoutStep = CheckoutStep.PAY,
    val splitCount: Int = 1,
    /** In equal parts or by dishes (UX audit P3). */
    val splitMode: SplitMode = SplitMode.EQUAL,
    /** By dishes: the units checked for the part being taken, per cart line id. */
    val pick: Map<Long, Int> = emptyMap(),
    /** The pad of "Autre" (any number of equal parts) is open. */
    val otherSplitOpen: Boolean = false,
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
    /** The next course still held at the table (1 to 3): the sale does not print it, so the screen warns. */
    val nextHeldCourse: Int? = null,
    /** "Envoyer maintenant" is running. */
    val sendingHeld: Boolean = false,
    /** Gift vouchers are on: the "Bon cadeau" method shows. */
    val giftVouchersEnabled: Boolean = false,
    /** The sale sells a gift voucher: it is paid in cash or by card only. */
    val sellsVoucher: Boolean = false,
    /** The gift voucher dialog; null while closed. */
    val voucher: VoucherDialogState? = null,
    /** An unpaid online order is collected here (UX audit O2): its sale is the handover, back goes to the board. */
    val online: Boolean = false,
) {
    /** The methods offered now: no gift voucher while the setting is off, only cash and card when the sale sells one. */
    val methods: List<PaymentMethod>
        get() =
            TILL_METHODS.filter { m ->
                if (sellsVoucher) {
                    m == PaymentMethod.CASH || m == PaymentMethod.CARD
                } else {
                    m != PaymentMethod.GIFT_VOUCHER || giftVouchersEnabled
                }
            }

    /** The amount being typed, in cents (no arithmetic: the digits are the cents). */
    val entryCents: Long get() = entry.cents

    /**
     * The amount the primary button collects: the rounded due for cash (what is handed over only decides the change),
     * the typed amount for the other methods (Vue `collectCents`).
     */
    val collectCents: Long get() = if (method == PaymentMethod.CASH) amounts.roundedDueCents else entryCents

    /** The cash handed over differs from what is due: the button adds the received amount under its label. */
    val showReceived: Boolean
        get() = method == PaymentMethod.CASH && entryCents > 0L && entryCents != amounts.roundedDueCents

    /** A part left a rest: the row "Reste 12,30 € :" is shown in front of the methods still allowed. */
    val showRest: Boolean get() = taken.isNotEmpty() && amounts.restCents > 0L

    /** The sale is split by dishes (UX audit P3). */
    val dishMode: Boolean get() = splitMode == SplitMode.DISHES

    /** How many units are checked for the current part. */
    val pickedTotal: Int get() = pick.values.sum()

    /** "Autre" is highlighted for a number of parts the four buttons do not give. */
    val otherActive: Boolean get() = !dishMode && splitCount > SPLIT_QUICK_MAX

    /**
     * The covers of the table when they can be a number of equal parts (UX audit T5, `coversSplit` of `CheckoutView.vue`):
     * the matching button is marked and the pad of "Autre" starts on them. Never applied by itself, so a table paid in one
     * go stays one tap.
     */
    val coversSplit: Int? get() = covers?.takeIf { it in SPLIT_OTHER_MIN..SPLIT_OTHER_MAX }

    /** The dish list of the recap: what each line has left, what was paid, what is checked. */
    val dishRows: List<DishRow>
        get() {
            val lines = dishLinesOf(summary.lines)
            val paid = paidUnits(taken)
            return summary.lines.mapIndexed { index, line ->
                DishRow(line, unitsLeft(lines[index], paid), paid[line.id] ?: 0, pick[line.id] ?: 0)
            }
        }

    /** A part by dishes needs a unit checked (the sale only waiting to be signed excepted). */
    val dishBlocked: Boolean
        get() = dishMode && !amounts.allPaid && (pickedTotal == 0 || (amounts.dueCents == 0L && !amounts.everythingPicked))

    /** The index of the part being taken. */
    val partIndex: Int get() = taken.size

    /** True for the part that pays the rest. */
    val isLastPart: Boolean get() = if (dishMode) amounts.everythingPicked else partIndex >= splitCount - 1

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
                !dishBlocked &&
                (amounts.allPaid || if (method == PaymentMethod.CASH) canPayCash else entryCents > 0L && !exactTooHigh)

    /** True when the "exact" shortcut is the current entry. */
    val exactActive: Boolean
        get() = fresh && entryCents == if (method == PaymentMethod.CASH) amounts.roundedDueCents else amounts.dueCents

    /** True when [quickCents] is the current entry. */
    fun quickActive(quickCents: Long): Boolean = fresh && entryCents == quickCents

    /** A split in more parts than cents would give a part of 0; only before the first part is taken (Vue `splitAllowed`). */
    fun splitAllowed(parts: Int): Boolean = taken.isEmpty() && (parts == 1 || parts.toLong() <= totalCents)

    /** The by-dishes button: before the first part is taken, or while the sale is already split by dishes. */
    val dishesAllowed: Boolean get() = dishMode || taken.isEmpty()

    /** The "Autre" button: before the first part is taken, or while it is the split in use. */
    val otherAllowed: Boolean get() = otherActive || taken.isEmpty()
}

/** What the view model asks of the navigation. */
internal sealed interface CheckoutEffect {
    /** Back to the order screen of the order being paid. */
    data class BackToOrder(
        val orderId: Long,
    ) : CheckoutEffect

    /** Back to the online orders board, from the checkout of an online order (UX audit O2). */
    data object BackToBoard : CheckoutEffect

    /** The sale of an online order is done: back to the online orders board. */
    data object BoardAfterSale : CheckoutEffect

    /** The sale is done: on to the tables, for the next customer. */
    data object NewOrder : CheckoutEffect
}
