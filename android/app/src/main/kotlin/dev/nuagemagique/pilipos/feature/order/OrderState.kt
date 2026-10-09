package dev.nuagemagique.pilipos.feature.order

import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.ModifierGroup
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.VatSplitRow

/** The dialog open over the order screen, if any (`pricing` and `choosing` of `OrderView.vue`). */
internal sealed interface OrderDialog {
    /** Nothing is open. */
    data object None : OrderDialog

    /** A free-price product waits for its price. */
    data class Price(
        val item: MenuItem,
    ) : OrderDialog

    /** A product with option [groups] waits for the picks. */
    data class Modifiers(
        val item: MenuItem,
        val groups: List<ModifierGroup>,
    ) : OrderDialog

    /** "Customer left without paying": a manager confirms with the PIN; [userName] is shown as the hint. */
    data class CloseUnpaid(
        val userName: String,
    ) : OrderDialog
}

/** One line of the cart column: the engine line and its amount, null until the VAT helper answered. */
internal data class CartRow(
    val line: CartLine,
    val amount: LineAmount?,
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
