package dev.nuagemagique.pilipos.soak

import android.util.Log
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.CheckoutResult
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RefundItem
import java.util.Random

/** The kinds of sale of the mix (P7D12). */
internal enum class SaleKind(val label: String) {
    TAKEAWAY("takeaway"),
    TABLE("table"),
    SPLIT("split"),
    REFUND("refund"),
}

/** A signed sale that a later refund of the same day can undo. */
internal data class RefundableSale(
    val fiscalEventId: Long,
    val orderId: Long,
)

/** What one sale attempt came to. */
internal data class SaleOutcome(
    val kind: SaleKind,
    val ok: Boolean,
    val ticket: Long?,
    val fiscalEventId: Long?,
    val orderId: Long?,
    val error: String?,
)

/**
 * The sales of the mix, as an operator would take them. Every call goes through [SoakEngine]. No amount is
 * computed here: a cash part carries no amount (the engine settles what is left), the split card part is the
 * literal [SoakConfig.splitCardCents] of the profile, and the card part of a table sale carries the total that
 * the engine itself reports for the order.
 */
internal class SoakSales(
    private val engine: SoakEngine,
    private val config: SoakConfig,
    private val rng: Random,
) {
    private var pool: List<MenuItem>? = null
    private val rejected = HashSet<Long>()

    /** A takeaway of one to three lines paid in cash. */
    fun takeaway(): SaleOutcome = attempt(SaleKind.TAKEAWAY) { sell(newOrder(OrderType.TAKEAWAY, null), cashOnly()) }

    /** A table: order, registration of the order, pre-bill, then the card. */
    fun table(): SaleOutcome =
        attempt(SaleKind.TABLE) {
            val free = engine.call { it.listTables() }.firstOrNull { it.orderId == null } ?: throw AssertionError("no free table")
            val order = newOrder(OrderType.DINE_IN, free.tableNumber)
            addLines(order)
            engine.call { it.signTableOrder(order) }
            engine.call { it.signPreBill(order) }
            val total = engine.call { it.getOrder(order) }.totalCents
            engine.call { it.checkout(order, listOf(PaymentPart(PaymentMethod.CARD, total)), null) } to order
        }

    /** A split payment: the card part is a literal of the profile, the cash part pays what is left. */
    fun split(): SaleOutcome =
        attempt(SaleKind.SPLIT) {
            val parts = listOf(PaymentPart(PaymentMethod.CARD, config.splitCardCents), PaymentPart(PaymentMethod.CASH, null))
            sell(newOrder(OrderType.TAKEAWAY, null), parts, MIN_SPLIT_LINES)
        }

    /** The full refund of [target]: every line of that sale, as the sale recorded it. */
    fun refund(target: RefundableSale): SaleOutcome =
        attempt(SaleKind.REFUND) {
            val lines = engine.call { it.listOrderLines(target.orderId) }
            val items = lines.map { RefundItem(it.menuItemId, it.quantity, it.id) }
            engine.call { it.refundCheckout(target.fiscalEventId, items, config.insz, null) } to target.orderId
        }

    private fun cashOnly() = listOf(PaymentPart(PaymentMethod.CASH, null))

    private fun sell(
        order: Long,
        parts: List<PaymentPart>,
        minLines: Int = 1,
    ): Pair<CheckoutResult, Long> {
        addLines(order, minLines)
        return engine.call { it.checkout(order, parts, null) } to order
    }

    private fun newOrder(
        type: OrderType,
        table: Int?,
    ): Long = engine.call { it.createOrder(CreateOrderInput(type, table, config.insz)) }.id

    private fun addLines(
        order: Long,
        minLines: Int = 1,
    ) {
        repeat(minLines + rng.nextInt(MAX_EXTRA_LINES + 1)) { addOneLine(order) }
    }

    /** Adds one product the engine accepts plainly; a product it refuses is not tried again. */
    private fun addOneLine(order: Long) {
        for (item in candidates().shuffled(rng)) {
            if (item.id in rejected) continue
            try {
                engine.call { it.addOrderLine(AddOrderLineInput(order, item.id, 1, null, null)) }
                return
            } catch (e: PosException) {
                rejected.add(item.id)
                Log.i(SoakLog.TAG, "product ${item.id} cannot be added plain: ${e.message}")
            }
        }
        throw AssertionError("no product of the menu can be added without choices")
    }

    private fun candidates(): List<MenuItem> =
        pool ?: engine.call { it.getMenuItems(null, "fr") }.filter { it.active && !it.openPrice }.also { pool = it }

    @Suppress("TooGenericExceptionCaught")
    private fun attempt(
        kind: SaleKind,
        block: () -> Pair<CheckoutResult, Long>,
    ): SaleOutcome =
        try {
            val (result, order) = block()
            SaleOutcome(
                kind = kind,
                ok = result.success,
                ticket = result.ticket?.ticketNumber,
                fiscalEventId = result.fiscalEventId,
                orderId = order,
                error = result.error,
            )
        } catch (e: PosException) {
            SaleOutcome(kind, false, null, null, null, "${e::class.simpleName}: ${e.message}")
        } catch (e: AssertionError) {
            SaleOutcome(kind, false, null, null, null, e.message ?: "assertion")
        } catch (e: RuntimeException) {
            SaleOutcome(kind, false, null, null, null, "${e::class.simpleName}: ${e.message}")
        }

    private companion object {
        const val MAX_EXTRA_LINES = 2
        const val MIN_SPLIT_LINES = 2
    }
}
