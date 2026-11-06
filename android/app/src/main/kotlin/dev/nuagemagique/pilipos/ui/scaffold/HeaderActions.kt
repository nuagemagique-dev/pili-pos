package dev.nuagemagique.pilipos.ui.scaffold

import dev.nuagemagique.pilipos.i18n.AppLanguage

/** What the own-PIN dialog of the header does (the `PinDialog.vue` the user button opens). */
internal data class HeaderPinActions(
    /** Saves the PIN typed twice (`set_pin` for the operator), a refusal is thrown for the dialog. */
    val onSubmit: suspend (String) -> Unit = {},
    /** The screen text of a refusal. */
    val describeError: (Throwable) -> String = { it.message.orEmpty() },
    /** The PIN was saved. */
    val onDone: () -> Unit = {},
    /** The dialog was dismissed. */
    val onCancel: () -> Unit = {},
)

/** Everything the operator can do in the header. */
internal data class HeaderActions(
    val onTables: () -> Unit = {},
    val onNewTakeaway: () -> Unit = {},
    val onOnlineOrders: () -> Unit = {},
    val onReports: () -> Unit = {},
    val onSettings: () -> Unit = {},
    val onLock: () -> Unit = {},
    /** The user button: opens or closes the menu under it (UX audit L1, R5). */
    val onOpenUserMenu: () -> Unit = {},
    /** A tap outside the user menu. */
    val onCloseUserMenu: () -> Unit = {},
    /** "Changer mon PIN" of the user menu. */
    val onOpenPin: () -> Unit = {},
    /** A language of the user menu: kept for this user, applied at once. */
    val onPickLanguage: (AppLanguage) -> Unit = {},
    /** "Fin de service" of the user menu: the first tap arms it, the second ends the shift. */
    val onEndShift: () -> Unit = {},
    /** The table chip of a movable order: opens the table picker (UX audit C8). */
    val onMoveTable: () -> Unit = {},
    /** The free table tapped in the picker: the order moves there at once. */
    val onPickTable: (Int) -> Unit = {},
    /** The picker was dismissed. */
    val onCloseTablePicker: () -> Unit = {},
    /** "Ajouter un nom" of a numbered takeaway (UX audit T2). */
    val onAddCustomer: () -> Unit = {},
    val onCustomerName: (String) -> Unit = {},
    val onCustomerPhone: (String) -> Unit = {},
    val onCloseCustomer: () -> Unit = {},
    val pin: HeaderPinActions = HeaderPinActions(),
)

/** What the header asks of the navigation; the composable performs it, the model never holds a navigator. */
internal sealed interface HeaderEffect {
    /** Opens [route] on top of the current screen. */
    data class Navigate(
        val route: String,
    ) : HeaderEffect

    /** Returns to the login screen. */
    data object ToLogin : HeaderEffect
}
