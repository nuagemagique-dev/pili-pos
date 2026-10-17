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
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.AmountDialog
import dev.nuagemagique.pilipos.ui.components.PinDialog
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadTexts
import dev.nuagemagique.pilipos.ui.components.defaultAmountDialogTexts
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.launch
import uniffi.pos_ffi.MenuItem

/** What the operator can do on the menu part of the order screen. */
internal data class OrderCallbacks(
    val onCategory: (Long) -> Unit = {},
    val onItem: (MenuItem) -> Unit = {},
    val onPrice: (Long) -> Unit = {},
    val onPicks: (List<Long>) -> Unit = {},
    /** The "Allergènes" button: allergen mode on or off. */
    val onAllergenMode: () -> Unit = {},
    /** Print the open allergen sheet (`false`) or the whole menu's (`true`). */
    val onAllergenPrint: (Boolean) -> Unit = {},
    val onDismiss: () -> Unit = {},
    /** The "Bon cadeau" button: ask for the amount of a voucher to sell. */
    val onSellVoucher: () -> Unit = {},
    /** The amount typed for the voucher. */
    val onVoucherAmount: (Long) -> Unit = {},
    /** A course picked in the picker. */
    val onCoursePick: (Int) -> Unit = {},
    /** The manager PIN of "customer left without paying"; throws what the engine refused. */
    val onUnpaidPin: suspend (String) -> Unit = {},
    val describeError: (Throwable) -> String = { it.message.orEmpty() },
    /** The sale was closed unpaid. */
    val onUnpaidDone: () -> Unit = {},
)

/**
 * The order route (`OrderView.vue`): builds [MenuViewModel] from the application graph and draws [OrderContent] in
 * the staff scaffold, with the price and options dialogs in the overlay slot (PD214). It does not navigate when it
 * starts and does not redirect (PD205): the order of the route is loaded once the engine is ready and somebody is
 * logged in, and without an order an empty state is shown. It leaves only for the checkout (the pay button) and
 * for the tables (a sale closed unpaid). The root carries the tag `placeholder_order` (PD204: the tag means "feature root").
 */
@Composable
internal fun OrderRoute(
    navigator: AppNavigator,
    orderId: Long,
) {
    val entry = rememberEntryPoint<OrderEntryPoint>()
    val model = viewModel { MenuViewModel(entry, orderId) }
    val state by model.state.collectAsState()
    // The order of the route and the refusals are the shown screen's job only, not that of one left on the back stack.
    LaunchedEffect(model) {
        launch { model.runClock() }
        model.run()
    }
    val callbacks =
        OrderCallbacks(
            onCategory = model::selectCategory,
            onItem = model::addItem,
            onPrice = model::confirmPrice,
            onPicks = model::confirmModifiers,
            onAllergenMode = model::toggleAllergenMode,
            onAllergenPrint = model::printAllergenSheet,
            onDismiss = model::dismissDialog,
            onSellVoucher = model::askGiftVoucher,
            onVoucherAmount = model::confirmGiftVoucher,
            onCoursePick = model::pickCourse,
            onUnpaidPin = model::closeUnpaid,
            describeError = model::describe,
            onUnpaidDone = {
                model.unpaidClosed()
                navigator.navigate(TablesRoutes.TABLES)
            },
        )
    val cartActions =
        OrderCartActions(
            onPay = { state.order?.let { navigator.navigate(CheckoutRoutes.checkout(it.id)) } },
            onIncrement = model::incrementLine,
            onDecrement = model::decrementLine,
            onRemove = model::removeLine,
            onSend = model::sendToKitchen,
            onPreBill = model::printPreBill,
            onCloseUnpaid = model::askCloseUnpaid,
            onCourse = model::openCoursePicker,
            onSendNext = model::sendNextCourse,
        )
    PosScaffold(
        area = HeaderArea.ORDER,
        navigator = navigator,
        modifier = Modifier.testTag("placeholder_order"),
        overlay = { OrderDialogs(state.dialog, callbacks, state.menu.language) },
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
                AllergenBar(menu, callbacks.onAllergenMode, onSellVoucher = callbacks.onSellVoucher)
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

/**
 * The open dialog, over everything: the price of a free-price product, the options of a product (`OrderView.vue`)
 * or the manager PIN of an unpaid close (`CartSidebar.vue`).
 */
@Composable
internal fun BoxScope.OrderDialogs(
    dialog: OrderDialog,
    callbacks: OrderCallbacks,
    language: AppLanguage = AppLanguage.FR,
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
        OrderDialog.SellVoucher ->
            AmountDialog(
                title = vueText("gv_sell_title"),
                itemName = vueText("gv_sell_item"),
                texts = defaultAmountDialogTexts(),
                onConfirm = callbacks.onVoucherAmount,
                onCancel = callbacks.onDismiss,
            )
        is OrderDialog.Modifiers ->
            ModifierDialog(dialog.item, dialog.groups, onConfirm = callbacks.onPicks, onCancel = callbacks.onDismiss)
        is OrderDialog.AllergenSheet ->
            AllergenSheetDialog(dialog, language, onPrint = callbacks.onAllergenPrint, onClose = callbacks.onDismiss)
        is OrderDialog.CloseUnpaid -> CloseUnpaidDialog(dialog.userName, callbacks)
        is OrderDialog.CoursePicker -> CoursePickerDialog(dialog.current, callbacks.onCoursePick, callbacks.onDismiss)
    }
}

/** The manager PIN of "customer left without paying" (`PinDialog.vue` with the hint `unpaid_hint`). */
@Composable
private fun CloseUnpaidDialog(
    userName: String,
    callbacks: OrderCallbacks,
) {
    val hint = vueText("unpaid_hint")
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
        onSubmit = callbacks.onUnpaidPin,
        describeError = callbacks.describeError,
        onDone = callbacks.onUnpaidDone,
        onCancel = callbacks.onDismiss,
    )
}
