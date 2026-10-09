@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.checkout.CheckoutRoutes
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.AmountDialog
import dev.nuagemagique.pilipos.ui.components.defaultAmountDialogTexts
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.MenuItem

/** What the operator can do on the menu part of the order screen. */
internal data class OrderCallbacks(
    val onCategory: (Long) -> Unit = {},
    val onItem: (MenuItem) -> Unit = {},
    val onPrice: (Long) -> Unit = {},
    val onPicks: (List<Long>) -> Unit = {},
    val onDismiss: () -> Unit = {},
)

/**
 * The order route (`OrderView.vue`): builds [MenuViewModel] from the application graph and draws [OrderContent] in
 * the staff scaffold, with the price and options dialogs in the overlay slot (PD214). It does not navigate when it
 * starts and does not redirect (PD205): the order of the route is loaded once the engine is ready and somebody is
 * logged in, and without an order an empty state is shown. It leaves only for the checkout, from the pay button. The
 * root carries the tag `placeholder_order` (PD204: the tag means "feature root").
 */
@Composable
internal fun OrderRoute(
    navigator: AppNavigator,
    orderId: Long,
) {
    val entry = rememberEntryPoint<OrderEntryPoint>()
    val model = viewModel { MenuViewModel(entry, orderId) }
    val state by model.state.collectAsState()
    LaunchedEffect(model) { model.runClock() }
    val callbacks =
        OrderCallbacks(
            onCategory = model::selectCategory,
            onItem = model::addItem,
            onPrice = model::confirmPrice,
            onPicks = model::confirmModifiers,
            onDismiss = model::dismissDialog,
        )
    val cartActions =
        OrderCartActions(onPay = { state.order?.let { navigator.navigate(CheckoutRoutes.checkout(it.id)) } })
    PosScaffold(
        area = HeaderArea.ORDER,
        navigator = navigator,
        modifier = Modifier.testTag("placeholder_order"),
        overlay = { OrderDialogs(state.dialog, callbacks) },
    ) {
        OrderContent(state, callbacks, cartActions)
    }
}

/**
 * The order screen, stateless: the Z reminder, the category tabs over the product grid on the left and the cart
 * column on the right, laid out for 1920 x 1080 dp. Without a user or without an order it shows an empty state
 * instead. Test tags: `order_z_warning`, `order_empty` and those of [CategoryTabs], [MenuGrid] and [OrderCartColumn].
 */
@Composable
internal fun OrderContent(
    state: OrderUiState,
    callbacks: OrderCallbacks,
    cartActions: OrderCartActions,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    if (!state.signedIn || state.order == null) {
        Box(modifier.fillMaxSize().background(c.bgPrimary).testTag("order_empty"), contentAlignment = Alignment.Center) {
            Text(uiText(R.string.ui_order_empty), style = PiliTheme.typography.title, color = c.textSecondary)
        }
        return
    }
    val menu = state.menu
    val color = menu.categories.firstOrNull { it.id == menu.selectedCategoryId }?.color
    Column(modifier.fillMaxSize().background(c.bgPrimary)) {
        if (state.zWarning) {
            val t = PiliTheme.typography
            Text(
                text = vueText("z_report_warning"),
                style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
                color = c.textOnAccent,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(c.accentWarning)
                        .padding(horizontal = 20.dp, vertical = 14.dp)
                        .testTag("order_z_warning"),
            )
        }
        Row(Modifier.weight(1f).fillMaxWidth()) {
            Column(Modifier.weight(1f).fillMaxHeight()) {
                CategoryTabs(menu.categories, menu.selectedCategoryId, menu.language, callbacks.onCategory)
                MenuGrid(
                    items = menu.items,
                    categoryColor = parseCategoryColor(color),
                    cartQuantities = menu.cartQuantities,
                    language = menu.language,
                    labelMode = menu.labelMode,
                    loaded = menu.loaded,
                    onAdd = callbacks.onItem,
                    modifier = Modifier.weight(1f).fillMaxWidth(),
                )
            }
            OrderCartColumn(state.cart, cartActions, Modifier.width(PiliTheme.dimens.cartWidth).fillMaxHeight())
        }
    }
}

/** The open dialog, over everything: the price of a free-price product or the options of a product (`OrderView.vue`). */
@Composable
internal fun BoxScope.OrderDialogs(
    dialog: OrderDialog,
    callbacks: OrderCallbacks,
) {
    when (dialog) {
        OrderDialog.None -> Unit
        is OrderDialog.Price ->
            AmountDialog(
                title = vueText("open_price_title"),
                itemName = dialog.item.name,
                texts = defaultAmountDialogTexts(),
                onConfirm = callbacks.onPrice,
                onCancel = callbacks.onDismiss,
            )
        is OrderDialog.Modifiers ->
            ModifierDialog(dialog.item, dialog.groups, onConfirm = callbacks.onPicks, onCancel = callbacks.onDismiss)
    }
}
