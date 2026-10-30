package dev.nuagemagique.pilipos.feature.order

import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import uniffi.pos_ffi.AllergenEntry
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.DiscountInput
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.ModifierGroup
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.SheetProductView
import uniffi.pos_ffi.SlipOutcome
import uniffi.pos_ffi.SoldOutItem
import uniffi.pos_ffi.VatSplitRow

/** The dialog open over the order screen, if any (`pricing` and `choosing` of `OrderView.vue`). */
internal sealed interface OrderDialog {
    /** Nothing is open. */
    data object None : OrderDialog

    /** A free-price product waits for its price. */
    data class Price(
        val item: MenuItem,
    ) : OrderDialog

    /** A product sold by weight is on the scale; [stage] is where the weighing stands. */
    data class Weighing(
        val item: MenuItem,
        val stage: String = "waiting",
    ) : OrderDialog

    /** A product with option [groups] waits for the picks. */
    data class Modifiers(
        val item: MenuItem,
        val groups: List<ModifierGroup>,
    ) : OrderDialog

    /** "Change table": the free tables [tables] the order of table [from] can move to. */
    data class TablePicker(
        val from: Int,
        val tables: List<Int>,
    ) : OrderDialog

    /**
     * "Discount": the dialog of a manager's discount on the order ([lineId] null) or for one line free. [baseCents]
     * is what it applies to: the order's total, or the line's amount.
     */
    data class Discount(
        val lineId: Long?,
        val baseCents: Long,
    ) : OrderDialog

    /** "Note": the kitchen note of line [lineId]; [sent] when the line already went to the kitchen (it prints now). */
    data class Note(
        val lineId: Long,
        val initial: String,
        val sent: Boolean,
    ) : OrderDialog

    /** The manager PIN that confirms a discount or its removal; [userName] is shown as the hint. */
    data class DiscountPin(
        val userName: String,
        val action: DiscountAction,
    ) : OrderDialog

    /** The availability of one product ("Épuisé" mode): sold out for the day, until re-enabled, or available. */
    data class SoldOut(
        val item: MenuItem,
    ) : OrderDialog

    /** "Produits épuisés (n)": every product sold out now, each one can be put back on sale. */
    data object SoldOutList : OrderDialog

    /**
     * The allergen sheet of [item] (allergen mode, `AllergenSheetDialog.vue`): [sheet] in the UI language and [sheetFr]
     * in French when that differs, null until read; [error] is the text of a refusal, [printResult] what the last
     * print came to and [busy] a print running.
     */
    data class AllergenSheet(
        val item: MenuItem,
        val sheet: SheetProductView? = null,
        val sheetFr: SheetProductView? = null,
        val error: String? = null,
        val printResult: SlipOutcome? = null,
        val busy: Boolean = false,
    ) : OrderDialog

    /** The amount of a gift voucher to sell (`sellingVoucher` of `OrderView.vue`). */
    data object SellVoucher : OrderDialog

    /** The four courses of the line [lineId] as big buttons (`CoursePickerDialog.vue`); [current] is marked. */
    data class CoursePicker(
        val lineId: Long,
        val current: Int,
    ) : OrderDialog

    /** "Customer left without paying": a manager confirms with the PIN; [userName] is shown as the hint. */
    data class CloseUnpaid(
        val userName: String,
    ) : OrderDialog
}

/** What the PIN of a discount confirms. */
internal sealed interface DiscountAction {
    /** Apply [input] to the order or to a line. */
    data class Apply(
        val input: DiscountInput,
    ) : DiscountAction

    /** Remove the order discount ([lineId] null) or the offered line [lineId]. */
    data class Remove(
        val lineId: Long?,
    ) : DiscountAction
}

/** What a cashier picks for a product in the sold-out dialog (`Availability` of `utils/soldOut.ts`). */
internal enum class SoldOutChoice {
    /** Sold out until the end of the business day. */
    DAY,

    /** Sold out until re-enabled. */
    UNTIL,

    /** Available. */
    AVAILABLE,
}

/** The availability a product is in, from the flags the menu read gives (`availabilityOf`). */
internal fun MenuItem.availability(): SoldOutChoice =
    when {
        !soldOut -> SoldOutChoice.AVAILABLE
        soldOutUntil != null -> SoldOutChoice.DAY
        else -> SoldOutChoice.UNTIL
    }

/** One line of the cart column: the engine line and its amount, null until the VAT helper answered. */
internal data class CartRow(
    val line: CartLine,
    val amount: LineAmount?,
    /** A manager may give the line for free ("Offrir"): no change on it and no order discount. */
    val canOffer: Boolean = false,
    /** A manager may remove the line's own change (an offered line). */
    val canRemoveDiscount: Boolean = false,
    /** The line's kitchen note may be written or changed (any staff member, open order). */
    val canNote: Boolean = false,
)

/**
 * What the cart column shows (`CartSidebar.vue`). The total is the engine's [Order.totalCents] (PD208: nothing is
 * added here); the amounts of the lines and the VAT rows come from `CartAmounts`, the unsent count is
 * `OrderStore.unsentCount` (a quantity, not money).
 */
@Suppress("LongParameterList")
internal data class OrderCartState(
    val order: Order? = null,
    val rows: List<CartRow> = emptyList(),
    val language: AppLanguage = AppLanguage.FR,
    val labelMode: ProductLabelMode = ProductLabelMode.NAME,
    val canPay: Boolean = false,
    /** The VAT included in the total, per rate, as the engine split it. */
    val vatRows: List<VatSplitRow> = emptyList(),
    /** Units ordered and not yet sent to the bar or kitchen. */
    val unsentCount: Int = 0,
    /** "Send to kitchen" is running (`sending`). */
    val sending: Boolean = false,
    /** The provisional bill is being printed (`preBilling`). */
    val preBilling: Boolean = false,
    /** A manager may close the open order unpaid (`canCloseUnpaid`). */
    val canCloseUnpaid: Boolean = false,
    /** A dine-in order with lines may move to another table (`canTransfer`). */
    val canTransfer: Boolean = false,
    /** A manager may give an order discount (`canDiscount`). */
    val canDiscount: Boolean = false,
    /** An order discount applies: the button removes it instead (`orderDiscounted`). */
    val orderDiscounted: Boolean = false,
    /** Courses are on: the cart is grouped by course and each line has a chip. */
    val coursesEnabled: Boolean = false,
    /** The next course waiting for "Envoyer la suite" (1 to 3); null when none is held. */
    val nextHeldCourse: Int? = null,
    /** "Envoyer la suite" is running. */
    val sendingNext: Boolean = false,
)

/** What the cart column can ask for. */
internal data class OrderCartActions(
    val onPay: () -> Unit = {},
    val onIncrement: (lineId: Long) -> Unit = {},
    val onDecrement: (lineId: Long) -> Unit = {},
    val onRemove: (lineId: Long) -> Unit = {},
    val onSend: () -> Unit = {},
    val onPreBill: () -> Unit = {},
    val onCloseUnpaid: () -> Unit = {},
    val onTransfer: () -> Unit = {},
    val onDiscount: () -> Unit = {},
    val onRemoveOrderDiscount: () -> Unit = {},
    val onOffer: (lineId: Long) -> Unit = {},
    val onRemoveLineDiscount: (lineId: Long) -> Unit = {},
    val onNote: (lineId: Long) -> Unit = {},
    /** The course chip of a line: opens the picker. */
    val onCourse: (lineId: Long) -> Unit = {},
    /** "Envoyer la suite": release and print the next held course. */
    val onSendNext: () -> Unit = {},
)

/** The menu part of the screen: the categories, the products of the selected one and what is already in the cart. */
internal data class MenuState(
    val categories: List<Category> = emptyList(),
    val selectedCategoryId: Long? = null,
    val items: List<MenuItem> = emptyList(),
    /** Units of each product in the cart, for the badges on the product buttons. */
    val cartQuantities: Map<Long, Int> = emptyMap(),
    val language: AppLanguage = AppLanguage.FR,
    val labelMode: ProductLabelMode = ProductLabelMode.NAME,
    /** True once the products of the selected category were read for the engine that is running. */
    val loaded: Boolean = false,
    /** The user may mark products sold out (cashier and manager): the "Épuisé" bar is drawn. */
    val canSoldOut: Boolean = false,
    /** The "Épuisé" mode is on: a tap on a product opens its availability dialog. */
    val soldOutMode: Boolean = false,
    /** The products sold out now (`list_sold_out`), read for cashiers and managers only. */
    val soldOut: List<SoldOutItem> = emptyList(),
    /** Allergen mode: a tap on a product opens its allergen sheet instead of adding it. */
    val allergenMode: Boolean = false,
    /** The 14 allergens in Annex order (`list_allergens`), read when the mode is first turned on. */
    val allergenLegend: List<AllergenEntry> = emptyList(),
    /** Gift vouchers are on: the "Bon cadeau" button shows next to "Allergènes". */
    val giftVouchersEnabled: Boolean = false,
    /** The legend could not be read (`allergens.error` of `OrderView.vue`), already turned into text. */
    val allergenLegendError: String? = null,
)

/** What the order screen shows. */
internal data class OrderUiState(
    /** False when nobody is logged in: the screen shows an empty state and does not redirect (PD205). */
    val signedIn: Boolean = false,
    val menu: MenuState = MenuState(),
    val cart: OrderCartState = OrderCartState(),
    /** The Z-report reminder banner (`showZWarning`). */
    val zWarning: Boolean = false,
    val dialog: OrderDialog = OrderDialog.None,
) {
    /** The order being taken; null shows the empty state. */
    val order: Order? get() = cart.order
}
