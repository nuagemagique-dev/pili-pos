package dev.nuagemagique.pilipos.ui.scaffold

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
    val onOpenPin: () -> Unit = {},
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
