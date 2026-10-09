package dev.nuagemagique.pilipos.feature.webshop

import dev.nuagemagique.pilipos.ui.format.PosDateTime
import uniffi.pos_ffi.Attention
import uniffi.pos_ffi.WebshopOrderSummary
import java.time.Instant
import java.time.temporal.ChronoUnit

/** The column an order sits in (`Stage` of `WebshopOrdersView.vue`). */
internal enum class Stage { NEW, KITCHEN, READY, DONE }

/** The main button of a card, one per order status (`nextAction` of `WebshopOrdersView.vue`). */
internal enum class PrimaryAction(
    /** The Vue key of its caption. */
    val labelKey: String,
) {
    /** A signed order HubRise cancelled: refund it (managers only). */
    REFUND("webshop_action_refund"),

    /** An unpaid order cancelled after its P: close it. */
    CLOSE_CANCELLED("webshop_action_close_cancelled"),

    /** A new order: accept it (the engine signs it). */
    ACCEPT("webshop_action_accept"),

    /** An accepted order: the kitchen starts. */
    PREPARE("webshop_action_prepare"),

    /** An order in preparation: it is ready. */
    READY("webshop_action_ready"),

    /** The restaurant's own delivery leaves. */
    OUT_FOR_DELIVERY("webshop_action_out_for_delivery"),

    /** An unpaid order is handed over against cash. */
    COLLECT_CASH("webshop_action_collect_cash"),

    /** A prepaid order is handed over. */
    COMPLETE("webshop_action_complete"),
}

/** How an order is paid, once known ("Payee" / "A encaisser"). */
internal enum class PaymentBadge(
    /** The Vue key of its caption. */
    val labelKey: String,
) {
    PAID("webshop_badge_prepaid"),
    DUE("webshop_badge_unpaid"),
}

/** The one time that matters for the stage of a card, shown top right. */
internal sealed interface KeyTime {
    /** A new order waits since [minutes]; [late] after the alarm delay. */
    data class Waiting(
        val minutes: Long,
        val late: Boolean,
    ) : KeyTime

    /** The kitchen target time, already formatted. */
    data class ReadyAt(
        val time: String,
    ) : KeyTime

    /** The time the order was ready, already formatted. */
    data class Ready(
        val time: String,
    ) : KeyTime

    /** The order left for delivery. */
    data object InDelivery : KeyTime

    /** A finished order; [labelKey] is the Vue key of its outcome. */
    data class Finished(
        val labelKey: String,
    ) : KeyTime
}

/** An order nobody answered for this long raises the alarm: platforms cancel orders left unanswered. */
internal const val ALARM_MINUTES = 5L

private const val DELAY_SHORT_MINUTES = 15
private const val DELAY_MEDIUM_MINUTES = 30
private const val DELAY_LONG_MINUTES = 45

/** The delays offered on a kitchen card, in minutes (`delayChoices`). +60 is two taps on +30. */
internal val DELAY_CHOICES: List<Int> = listOf(DELAY_SHORT_MINUTES, DELAY_MEDIUM_MINUTES, DELAY_LONG_MINUTES)

/**
 * The rules of the online orders board, ported from the script of `ui/src/views/WebshopOrdersView.vue`. Pure: no
 * state and no engine call. The fiscal rules live in the engine; this only reads the state it reports to decide
 * which column an order sits in and which buttons it shows.
 */
@Suppress("TooManyFunctions") // One function per rule of the Vue script, each tested on its own.
internal object WebshopRules {
    private const val STATUS_NEW = "new"
    private const val STATUS_RECEIVED = "received"
    private const val STATUS_ACCEPTED = "accepted"
    private const val STATUS_IN_PREPARATION = "in_preparation"
    private const val STATUS_AWAITING_COLLECTION = "awaiting_collection"
    private const val STATUS_IN_DELIVERY = "in_delivery"
    private const val STATUS_COMPLETED = "completed"
    private const val STATUS_REJECTED = "rejected"
    private const val STATUS_CANCELLED = "cancelled"

    private const val FISCAL_PENDING = "pending"
    private const val FISCAL_SIGNED_N = "signed_n"
    private const val FISCAL_SIGNED_P = "signed_p"
    private const val FISCAL_REFUND_PENDING = "refund_pending"
    private const val FISCAL_CANCEL_PENDING = "cancel_pending"
    private const val FISCAL_REFUNDED = "refunded"
    private const val FISCAL_CLOSED = "closed"
    private const val FISCAL_VOIDED = "voided"
    private const val FISCAL_REJECTED = "rejected"

    private const val RESOLUTION_OK = "ok"
    private const val SERVICE_DELIVERY = "delivery"

    /** The HubRise status of [order]; a missing one counts as new. */
    fun statusOf(order: WebshopOrderSummary): String = order.hubriseStatus ?: STATUS_NEW

    /** The fiscal state of [order], `pending` before acceptance. */
    fun fiscalOf(order: WebshopOrderSummary): String = order.webshop?.fiscalState ?: FISCAL_PENDING

    /**
     * The column of [order]. A signed order HubRise cancelled waits for staff where they look first; refunded and
     * closed ones are done; a status nobody knows is surfaced in the first column rather than hidden.
     */
    fun stageOf(order: WebshopOrderSummary): Stage {
        val byFiscal =
            when (fiscalOf(order)) {
                FISCAL_REFUND_PENDING, FISCAL_CANCEL_PENDING -> Stage.NEW
                FISCAL_REFUNDED, FISCAL_CLOSED, FISCAL_VOIDED, FISCAL_REJECTED -> Stage.DONE
                else -> null
            }
        return byFiscal ?: byStatus(statusOf(order))
    }

    private fun byStatus(status: String): Stage =
        when (status) {
            STATUS_ACCEPTED, STATUS_IN_PREPARATION -> Stage.KITCHEN
            STATUS_AWAITING_COLLECTION, STATUS_IN_DELIVERY -> Stage.READY
            STATUS_COMPLETED, STATUS_REJECTED, STATUS_CANCELLED -> Stage.DONE
            else -> Stage.NEW
        }

    /** [orders] grouped by column, each keeping the engine's order. */
    fun byStage(orders: List<WebshopOrderSummary>): Map<Stage, List<WebshopOrderSummary>> {
        val grouped = orders.groupBy(::stageOf)
        return Stage.entries.associateWith { grouped[it].orEmpty() }
    }

    /** The restaurant's own webshop (the others are aggregators). */
    fun isOwnWebshop(order: WebshopOrderSummary): Boolean = order.source.lowercase().contains("web")

    /** Waiting for its money: registered (P), not sold yet. */
    fun isUnpaid(order: WebshopOrderSummary): Boolean = fiscalOf(order) == FISCAL_SIGNED_P

    /** An order the engine could not match with the menu: never signed, only refused or resynchronised. */
    fun needsReview(order: WebshopOrderSummary): Boolean = fiscalOf(order) == FISCAL_PENDING && order.webshop?.resolution != RESOLUTION_OK

    /** Card payment, offered next to cash when collecting. */
    fun canCollectByCard(order: WebshopOrderSummary): Boolean = isUnpaid(order) && stageOf(order) == Stage.READY

    /** Refusing is only possible before the order is signed. */
    fun canReject(order: WebshopOrderSummary): Boolean = fiscalOf(order) == FISCAL_PENDING && isNew(statusOf(order))

    /** Delaying the ready time only makes sense while the kitchen works on it. */
    fun canDelay(order: WebshopOrderSummary): Boolean = statusOf(order).let { it == STATUS_ACCEPTED || it == STATUS_IN_PREPARATION }

    /** Reprints once the order is taken and not finished yet. */
    fun canReprint(order: WebshopOrderSummary): Boolean = stageOf(order).let { it == Stage.KITCHEN || it == Stage.READY }

    private fun isNew(status: String): Boolean = status == STATUS_NEW || status == STATUS_RECEIVED

    /**
     * The main button of [order], or null when it has none (no HubRise id, an order to check, a refund without a
     * manager, a finished order). [isManager] is whether the logged-in operator is an administrator.
     */
    fun nextAction(
        order: WebshopOrderSummary,
        isManager: Boolean,
    ): PrimaryAction? {
        if (order.hubriseOrderId == null) return null
        return when (fiscalOf(order)) {
            FISCAL_REFUND_PENDING -> PrimaryAction.REFUND.takeIf { isManager }
            FISCAL_CANCEL_PENDING -> PrimaryAction.CLOSE_CANCELLED
            else -> byHubriseStatus(order)
        }
    }

    private fun byHubriseStatus(order: WebshopOrderSummary): PrimaryAction? =
        when (statusOf(order)) {
            STATUS_NEW, STATUS_RECEIVED -> PrimaryAction.ACCEPT.takeUnless { needsReview(order) }
            STATUS_ACCEPTED -> PrimaryAction.PREPARE
            STATUS_IN_PREPARATION -> PrimaryAction.READY
            STATUS_AWAITING_COLLECTION ->
                if (isOwnWebshop(order) && order.serviceType == SERVICE_DELIVERY) PrimaryAction.OUT_FOR_DELIVERY else finishAction(order)
            STATUS_IN_DELIVERY -> finishAction(order)
            else -> null
        }

    /** Handing over: an unpaid order is collected (its sale N), a prepaid one completed. */
    private fun finishAction(order: WebshopOrderSummary): PrimaryAction =
        if (isUnpaid(order)) PrimaryAction.COLLECT_CASH else PrimaryAction.COMPLETE

    /** "Payee" / "A encaisser", once the payment is known. */
    fun paymentBadge(order: WebshopOrderSummary): PaymentBadge? {
        val mode = order.webshop?.paymentMode
        return when {
            fiscalOf(order) == FISCAL_SIGNED_N || mode == "prepaid" -> PaymentBadge.PAID
            mode == "unpaid" -> PaymentBadge.DUE
            else -> null
        }
    }

    /** The Vue key of the banner of a signed order HubRise cancelled, or null. */
    fun cancelledBannerKey(
        order: WebshopOrderSummary,
        isManager: Boolean,
    ): String? =
        when (fiscalOf(order)) {
            FISCAL_REFUND_PENDING -> if (isManager) "webshop_cancelled_refund" else "webshop_manager_needed"
            FISCAL_CANCEL_PENDING -> "webshop_cancelled_close"
            else -> null
        }

    /** True when the order changed after its sale was signed (the signed sale stands). */
    fun changedAfterSigning(order: WebshopOrderSummary): Boolean =
        fiscalOf(order) != FISCAL_PENDING && order.webshop?.attention.orEmpty().any { it is Attention.ChangedAfterSigning }

    /** The Vue keys of the reasons an order needs a check, without doubles; one generic reason when the engine gave none. */
    fun reviewReasonKeys(order: WebshopOrderSummary): List<String> {
        val codes = order.webshop?.attention.orEmpty().map(::attentionCode).distinct()
        return if (codes.isEmpty()) listOf("webshop_attention_unknown_product") else codes.map { "webshop_attention_$it" }
    }

    /** The code of an attention (`attention.code` of the Vue records), which names the key of its text. */
    @Suppress("CyclomaticComplexMethod") // One branch per variant of the engine's Attention.
    fun attentionCode(attention: Attention): String =
        when (attention) {
            is Attention.UnknownChannel -> "unknown_channel"
            Attention.MissingServiceType -> "missing_service_type"
            Attention.NoItems -> "no_items"
            is Attention.ItemWithoutRef -> "item_without_ref"
            is Attention.UnknownProduct -> "unknown_product"
            is Attention.UnknownOption -> "unknown_option"
            is Attention.UnknownCharge -> "unknown_charge"
            is Attention.FractionalQuantity -> "fractional_quantity"
            is Attention.BadPrice -> "bad_price"
            is Attention.VatMismatch -> "vat_mismatch"
            is Attention.Discount -> "discount"
            is Attention.TotalMismatch -> "total_mismatch"
            is Attention.PaymentUnclear -> "payment_unclear"
            Attention.ChangedAfterSigning -> "changed_after_signing"
        }

    /** The Vue key naming the service of [order]. */
    fun serviceLabelKey(order: WebshopOrderSummary): String =
        when (order.serviceType) {
            SERVICE_DELIVERY -> "webshop_service_delivery"
            "collection" -> "webshop_service_collection"
            "eat_in" -> "webshop_service_eat_in"
            else -> "webshop_service_unknown"
        }

    /**
     * The full delivery address, only on delivery orders of the own webshop: the restaurant's driver decides from
     * the distance whether to accept. Aggregators deliver themselves, so their cards stay uncluttered.
     */
    fun deliveryAddress(order: WebshopOrderSummary): String? {
        if (!isOwnWebshop(order) || order.serviceType != SERVICE_DELIVERY) return null
        val street = listOfNotNull(order.customerAddress1, order.customerAddress2).filter { it.isNotBlank() }.joinToString(" ").trim()
        val city = listOfNotNull(order.customerPostalCode, order.customerCity).filter { it.isNotBlank() }.joinToString(" ").trim()
        return listOf(street, city).filter { it.isNotEmpty() }.joinToString(", ").ifEmpty { null }
    }

    /** Whole minutes since the order was created, or null when its time does not parse. */
    fun waitingMinutes(
        order: WebshopOrderSummary,
        now: Instant,
    ): Long? = PosDateTime.parseTimestamp(order.createdAt)?.let { ChronoUnit.MINUTES.between(it, now).coerceAtLeast(0L) }

    /** True for an unanswered new order that waited for [ALARM_MINUTES] or more. */
    fun isLate(
        order: WebshopOrderSummary,
        now: Instant,
    ): Boolean = fiscalOf(order) == FISCAL_PENDING && isNew(statusOf(order)) && (waitingMinutes(order, now) ?: 0L) >= ALARM_MINUTES

    /** The Vue key of the outcome of a finished order. */
    fun doneLabelKey(order: WebshopOrderSummary): String =
        when {
            fiscalOf(order) == FISCAL_REFUNDED -> "webshop_status_refunded"
            statusOf(order) == STATUS_REJECTED -> "webshop_status_rejected"
            statusOf(order) == STATUS_CANCELLED -> "webshop_status_cancelled"
            else -> "webshop_status_completed"
        }

    /** The time of [iso] as `HH:mm` in Europe/Brussels, or null when it does not parse. */
    fun clock(iso: String?): String? = iso?.let(PosDateTime::parseTimestamp)?.let(PosDateTime::formatTime)

    /**
     * The one time that matters for the stage of [order]: age for new orders, the target time in the kitchen, the
     * ready time once ready, the outcome when finished. Null when there is nothing to show.
     */
    fun keyTime(
        order: WebshopOrderSummary,
        now: Instant,
    ): KeyTime? =
        when (stageOf(order)) {
            Stage.NEW -> waitingMinutes(order, now)?.let { KeyTime.Waiting(it, isLate(order, now)) }
            Stage.KITCHEN -> clock(order.confirmedTime)?.let { KeyTime.ReadyAt(it) }
            Stage.READY ->
                if (statusOf(order) == STATUS_IN_DELIVERY) KeyTime.InDelivery else clock(order.confirmedTime)?.let { KeyTime.Ready(it) }
            Stage.DONE -> KeyTime.Finished(doneLabelKey(order))
        }
}
