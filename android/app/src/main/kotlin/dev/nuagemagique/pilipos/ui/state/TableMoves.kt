package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastAction
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException

/** How long the "Annuler" of a table move stays on screen, in milliseconds (`MOVE_UNDO_TTL_MS` of `order.ts`). */
internal const val MOVE_UNDO_TTL_MS: Long = 5000L

/** Lifetime of the toast of a refused undo, in milliseconds (6000 in `order.ts`). */
private const val UNDO_ERROR_TTL_MS: Long = 6000L

/**
 * `moveTable` and `undoMove` of `ui/src/stores/order.ts` (UX audit C8): the move of an order to a free table in one
 * tap. The order goes at once (`transfer_table`, M122, unchanged); the toast "Table 4 déplacée en 9" stays 5 s with an
 * "Annuler" that moves it back with a second transfer, signed like any other. A refusal of the move is thrown for the
 * caller's toast; a refusal of the undo is shown here as an error toast.
 */
internal class TableMoves(
    private val order: OrderStore,
    private val toasts: ToastStore,
    private val catalog: MessageCatalog,
    private val settings: PosSettingsStore,
    private val errors: ErrorPresenter,
) {
    /** Moves the open order [orderId] from table [from] to the free table [to] and offers the undo. */
    suspend fun move(
        orderId: Long,
        from: Int,
        to: Int,
    ) {
        order.transferOrder(orderId, to)
        toasts.showWithAction(
            text("transfer_moved", "from" to from, "to" to to),
            ToastAction(text("transfer_undo")) { undo(orderId, from, to) },
            MOVE_UNDO_TTL_MS,
        )
    }

    @Suppress("TooGenericExceptionCaught")
    private suspend fun undo(
        orderId: Long,
        from: Int,
        to: Int,
    ) {
        try {
            order.transferOrder(orderId, from)
            toasts.show(text("transfer_moved", "from" to to, "to" to from), ToastVariant.Success)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            errors.show(e, settings.language.value, null, UNDO_ERROR_TTL_MS)
        }
    }

    private fun text(
        key: String,
        vararg vars: Pair<String, Any>,
    ): String = catalog.text(key, settings.language.value, vars.toMap())
}
