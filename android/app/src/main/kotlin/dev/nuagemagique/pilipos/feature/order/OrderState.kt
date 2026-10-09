package dev.nuagemagique.pilipos.feature.order

import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.ModifierGroup
import uniffi.pos_ffi.Order

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
}

/** One line of the cart column: the engine line and its amount, null until the VAT helper answered. */
internal data class CartRow(
    val line: CartLine,
    val amount: LineAmount?,
)

/**
 * What the cart column shows. The total is the engine's [Order.totalCents] (PD208: nothing is added here); the
 * amounts of the lines come from `CartAmounts`.
 */
internal data class OrderCartState(
    val order: Order? = null,
    val rows: List<CartRow> = emptyList(),
    val language: AppLanguage = AppLanguage.FR,
    val labelMode: ProductLabelMode = ProductLabelMode.NAME,
    val canPay: Boolean = false,
)

/** What the cart column can ask for. P5b-T12 adds the line actions. */
internal data class OrderCartActions(
    val onPay: () -> Unit = {},
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
