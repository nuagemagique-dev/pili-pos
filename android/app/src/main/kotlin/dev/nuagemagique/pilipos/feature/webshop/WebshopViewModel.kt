package dev.nuagemagique.pilipos.feature.webshop

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.WebshopOrderSummary
import java.time.Clock
import java.time.Instant

/** How often the age of the new orders is refreshed, in milliseconds (15 s in `WebshopOrdersView.vue`). */
internal const val WEBSHOP_CLOCK_TICK_MS: Long = 15_000L

/** A dialog of the board, by the id of the order it concerns (the order is looked up again when it is drawn). */
internal sealed interface WebshopDialog {
    /** The id of the order the dialog is about. */
    val orderId: Long

    /** "Refuser" asks for a reason, then a confirmation: the refusal is sent to the platform. */
    data class RejectConfirm(
        override val orderId: Long,
        /** The chip picked so far; the red button waits for one. */
        val reason: RejectReason? = null,
    ) : WebshopDialog

    /** The refund of a signed order HubRise cancelled, confirmed by the logged-in manager. */
    data class RefundConfirm(
        override val orderId: Long,
    ) : WebshopDialog

    /** The minutes to push the promised time forward by. */
    data class Delay(
        override val orderId: Long,
    ) : WebshopDialog
}

/** What the board asks of the navigation (UX audit O2). */
internal sealed interface WebshopEffect {
    /** Opens the checkout of the unpaid online order [orderId], on the card method when [card]. */
    data class OpenCheckout(
        val orderId: Long,
        val card: Boolean,
    ) : WebshopEffect
}

/** What the webshop screen shows. */
internal data class WebshopUiState(
    /** The orders as the engine listed them last, in its order. */
    val orders: List<WebshopOrderSummary> = emptyList(),
    /** False when nobody is logged in: the screen shows an empty state and does not redirect (PD205). */
    val signedIn: Boolean = false,
    /** The name of the logged-in operator, who confirms a refund. */
    val operator: String = "",
    /** True for an administrator: only a manager refunds. */
    val isManager: Boolean = false,
    /** True for a manager or a Serveur (role cashier): an Apprenti may not accept, refuse, collect or complete. */
    val canTakePayment: Boolean = true,
    /** True when HubRise is switched on or online orders are still to settle; false shows the off state. */
    val hubriseVisible: Boolean = false,
    /** True while the list is being read. */
    val loading: Boolean = false,
    /** The text of the last failure of the list or of an action. */
    val error: String? = null,
    /** The ids of the orders an action is running on. */
    val busy: Set<Long> = emptySet(),
    /** The time the ages of the new orders are counted to. */
    val now: Instant = Instant.EPOCH,
    /** The third column shows the finished orders instead of the ready ones. */
    val showDone: Boolean = false,
    /** The id of the card whose two reprint choices are open. */
    val reprintOpenId: Long? = null,
    /** The open dialog, if any. */
    val dialog: WebshopDialog? = null,
)

/**
 * State holder of the online orders screen, a port of the script of `ui/src/views/WebshopOrdersView.vue`.
 *
 * The orders, the busy marks and every webshop command live in [dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore]; this
 * resolves what a card offers per order status ([WebshopRules]), keeps what only the screen knows (the third column,
 * the open reprint choices, the dialogs, the clock) and runs the button taps through the store. One action at a time:
 * a second tap while one still talks to the engine is ignored, as the `busyId` guard of the Vue view. The store shows
 * the refusals as toasts. The list is read when the screen opens or comes back with a user and HubRise visible
 * ([refresh]). It never navigates.
 */
@Suppress("TooManyFunctions") // One function per button of the board.
internal class WebshopViewModel(
    private val entry: WebshopEntryPoint,
    private val clock: Clock = Clock.systemUTC(),
) : ViewModel() {
    private val store = entry.webshop()
    private val session = entry.session()
    private val integrations = entry.integrations()

    private data class Own(
        val now: Instant,
        val showDone: Boolean = false,
        val reprintOpenId: Long? = null,
        val dialog: WebshopDialog? = null,
        val busyId: Long? = null,
    )

    private data class Remote(
        val orders: List<WebshopOrderSummary>,
        val loading: Boolean,
        val error: String?,
        val busy: Set<Long>,
        val visible: Boolean,
    )

    private val own = MutableStateFlow(Own(clock.instant()))
    private val effectChannel = Channel<WebshopEffect>(Channel.BUFFERED)

    /** One-shot moves, each delivered once. */
    val effects: Flow<WebshopEffect> = effectChannel.receiveAsFlow()

    private val remote =
        combine(
            store.orders,
            store.loading,
            store.lastError,
            store.busyOrderIds,
            integrations.hubriseVisible,
            ::Remote,
        )

    /** What the screen draws. */
    val state: StateFlow<WebshopUiState> =
        combine(remote, own, session.currentUser, session.isManager, session.canTakePayment) { r, o, user, manager, payer ->
            WebshopUiState(
                orders = r.orders,
                signedIn = user != null,
                operator = user?.name.orEmpty(),
                isManager = manager,
                canTakePayment = payer,
                hubriseVisible = r.visible,
                loading = r.loading,
                error = r.error,
                busy = if (o.busyId == null) r.busy else r.busy + o.busyId,
                now = o.now,
                showDone = o.showDone,
                reprintOpenId = o.reprintOpenId,
                dialog = o.dialog,
            )
        }.stateIn(viewModelScope, SharingStarted.Eagerly, WebshopUiState(now = own.value.now))

    /**
     * Moves the clock of the ages every [WEBSHOP_CLOCK_TICK_MS] until cancelled. The screen runs it while it is
     * composed (`onMounted` and `onUnmounted` of the Vue view), so nothing ticks on a screen that is gone.
     */
    suspend fun runClock() {
        while (true) {
            delay(WEBSHOP_CLOCK_TICK_MS)
            own.update { it.copy(now = clock.instant()) }
        }
    }

    /**
     * Reads the list in the background when there is a user and HubRise is visible: the screen opened or came back to
     * the front (`onMounted` of the Vue view). The store also follows the engine events by itself.
     */
    fun refresh() {
        if (session.currentUser.value == null || !integrations.hubriseVisible.value) return
        viewModelScope.launch { store.fetchOrders() }
    }

    // ── Board ─────────────────────────────────────────────────────

    /** The "Terminees" button: the third column shows the finished orders, or the ready ones again. */
    fun toggleDone() {
        own.update { it.copy(showDone = !it.showDone) }
    }

    /** The "Reimprimer" button of [order]: a first tap reveals the two tickets, a second hides them. */
    fun toggleReprint(order: WebshopOrderSummary) {
        own.update { it.copy(reprintOpenId = if (it.reprintOpenId == order.id) null else order.id) }
    }

    // ── Buttons of a card ─────────────────────────────────────────

    /**
     * The main button of [order] (`handlePrimary`): the action of its status. A refund first asks the manager to
     * confirm; the others run at once.
     */
    fun primary(order: WebshopOrderSummary) {
        val action = WebshopRules.nextAction(order, session.isManager.value) ?: return
        // An Apprenti's disabled buttons are not tapped; the engine would refuse them anyway.
        if (action.needsCashier && !session.canTakePayment.value) return
        if (action == PrimaryAction.REFUND) {
            own.update { it.copy(dialog = WebshopDialog.RefundConfirm(order.id)) }
        } else if (action == PrimaryAction.COLLECT_CASH) {
            openCheckout(order, card = false)
        } else {
            guarded(order.id) { perform(action, order) }
        }
    }

    /** "Refuser": asks for a reason and a confirmation first (the refusal goes to the platform). */
    fun askReject(order: WebshopOrderSummary) {
        if (!WebshopRules.canReject(order) || !session.canTakePayment.value) return
        own.update { it.copy(dialog = WebshopDialog.RejectConfirm(order.id)) }
    }

    /** "Retarder": opens the choice of minutes while the kitchen works on [order]. */
    fun askDelay(order: WebshopOrderSummary) {
        if (!WebshopRules.canDelay(order)) return
        own.update { it.copy(dialog = WebshopDialog.Delay(order.id)) }
    }

    /**
     * "Encaisser carte" (`handleCollectCard`): the checkout opens for the order with Carte selected (UX audit O2), where
     * the amount, the terminal and the parts are handled; its sale is the handover of this order.
     */
    fun collectCard(order: WebshopOrderSummary) {
        if (!WebshopRules.canCollectByCard(order) || !session.canTakePayment.value) return
        openCheckout(order, card = true)
    }

    /** One tap at a time, as the `busyId` guard: nothing opens while an action runs on a card. */
    private fun openCheckout(
        order: WebshopOrderSummary,
        card: Boolean,
    ) {
        if (own.value.busyId != null) return
        effectChannel.trySend(WebshopEffect.OpenCheckout(order.id, card))
    }

    /** "Resynchroniser": fetches the HubRise catalog again for an order to check (`handleResync`). */
    fun resync(order: WebshopOrderSummary) {
        if (!WebshopRules.needsReview(order)) return
        guarded(order.id) { store.resync() }
    }

    /** "Ticket cuisine": reprints the kitchen ticket and closes the choice. */
    fun printKitchen(order: WebshopOrderSummary) {
        own.update { it.copy(reprintOpenId = null) }
        viewModelScope.launch { attempt { store.printKitchen(order.id) } }
    }

    /** "Ticket client": reprints the customer ticket and closes the choice. */
    fun printCustomer(order: WebshopOrderSummary) {
        own.update { it.copy(reprintOpenId = null) }
        viewModelScope.launch { attempt { store.printCustomer(order.id) } }
    }

    // ── Dialogs ───────────────────────────────────────────────────

    /** Closes the open dialog without doing anything. */
    fun dismissDialog() {
        own.update { it.copy(dialog = null) }
    }

    /** A reason chip of the refusal dialog is picked (a second tap picks another one). */
    fun pickRejectReason(reason: RejectReason) {
        own.update { o -> (o.dialog as? WebshopDialog.RejectConfirm)?.let { o.copy(dialog = it.copy(reason = reason)) } ?: o }
    }

    /**
     * The refusal is confirmed. Nothing is sent before a reason is picked. The reason itself stays on the till:
     * HubRise has no reject reason field.
     */
    fun confirmReject() {
        val open = own.value.dialog as? WebshopDialog.RejectConfirm
        val order = if (open?.reason != null) takeDialogOrder<WebshopDialog.RejectConfirm>() else null
        if (order != null) guarded(order.id) { store.rejectOrder(order) }
    }

    /**
     * The refund is confirmed by the logged-in manager, whose INSZ is sent to the engine. Without a manager nothing
     * is sent and a warning is shown (`webshop_manager_needed`).
     */
    fun confirmRefund() {
        val order = takeDialogOrder<WebshopDialog.RefundConfirm>() ?: return
        val insz = session.currentUser.value?.insz
        if (!session.isManager.value || insz == null) {
            warn("webshop_manager_needed")
            return
        }
        guarded(order.id) { store.refundOrder(order, insz) }
    }

    /** [minutes] more are promised to the customer; the dialog closes. */
    fun delay(minutes: Int) {
        val order = takeDialogOrder<WebshopDialog.Delay>() ?: return
        val hubriseId = order.hubriseOrderId ?: return
        viewModelScope.launch { attempt { store.delayByMinutes(hubriseId, minutes) } }
    }

    // ── Internals ─────────────────────────────────────────────────

    private suspend fun perform(
        action: PrimaryAction,
        order: WebshopOrderSummary,
    ): Boolean {
        val hubriseId = order.hubriseOrderId ?: return false
        return when (action) {
            PrimaryAction.ACCEPT -> store.acceptOrder(order)
            PrimaryAction.PREPARE -> store.startPreparation(hubriseId)
            PrimaryAction.READY -> store.markReady(hubriseId)
            PrimaryAction.OUT_FOR_DELIVERY -> store.markOutForDelivery(hubriseId)
            // Collecting opens the checkout (UX audit O2) and never reaches here.
            PrimaryAction.COLLECT_CASH -> false
            PrimaryAction.COMPLETE -> store.completeOrder(order)
            PrimaryAction.CLOSE_CANCELLED -> store.closeCancelledOrder(order)
            // The refund goes through its confirmation dialog.
            PrimaryAction.REFUND -> false
        }
    }

    /** Closes the dialog of type [T] and answers the order it was about, as the store lists it now. */
    private inline fun <reified T : WebshopDialog> takeDialogOrder(): WebshopOrderSummary? {
        val dialog = own.value.dialog as? T ?: return null
        own.update { it.copy(dialog = null) }
        return store.orders.value.firstOrNull { it.id == dialog.orderId }
    }

    /** Runs [block] when no other action is running (one at a time, as `busyId`); the busy mark stays until it ends. */
    private fun guarded(
        orderId: Long,
        block: suspend () -> Unit,
    ) {
        if (!takeBusy(orderId)) return
        viewModelScope.launch {
            try {
                attempt { block() }
            } finally {
                own.update { it.copy(busyId = null) }
            }
        }
    }

    private fun takeBusy(orderId: Long): Boolean {
        while (true) {
            val now = own.value
            if (now.busyId != null) return false
            if (own.compareAndSet(now, now.copy(busyId = orderId))) return true
        }
    }

    private fun warn(key: String) {
        entry.toasts().show(entry.catalog().text(key, entry.settings().language.value), ToastVariant.Warning)
    }

    /** Runs [block]; a failure the store did not already show is shown as an error toast (cancellation is rethrown). */
    @Suppress("TooGenericExceptionCaught")
    private suspend fun attempt(block: suspend () -> Any?) {
        try {
            block()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            entry.errors().show(e, entry.settings().language.value)
        }
    }
}
