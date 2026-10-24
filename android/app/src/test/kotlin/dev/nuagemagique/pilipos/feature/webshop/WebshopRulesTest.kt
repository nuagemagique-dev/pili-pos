package dev.nuagemagique.pilipos.feature.webshop

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.Attention
import java.time.Instant

/** The pure rules of the board: the column of an order, its buttons, its notices and its key time. */
@Suppress("TooManyFunctions")
class WebshopRulesTest {
    private fun stage(
        hubrise: String?,
        fiscal: String? = "signed_n",
    ) = WebshopRules.stageOf(webshopOrder(1, hubrise = hubrise, fiscal = fiscal))

    @Test
    fun theColumnFollowsTheHubriseStatus() {
        assertEquals(Stage.NEW, stage("new", "pending"))
        assertEquals(Stage.NEW, stage("received", "pending"))
        assertEquals(Stage.NEW, stage(null, "pending"))
        assertEquals(Stage.NEW, stage("something-new"))
        assertEquals(Stage.KITCHEN, stage("accepted"))
        assertEquals(Stage.KITCHEN, stage("in_preparation"))
        assertEquals(Stage.READY, stage("awaiting_collection"))
        assertEquals(Stage.READY, stage("in_delivery"))
        assertEquals(Stage.DONE, stage("completed"))
        assertEquals(Stage.DONE, stage("rejected"))
        assertEquals(Stage.DONE, stage("cancelled"))
    }

    @Test
    fun aCancelledSignedOrderWaitsInTheFirstColumnAndAFinishedFiscalStateIsDone() {
        assertEquals(Stage.NEW, stage("cancelled", "refund_pending"))
        assertEquals(Stage.NEW, stage("cancelled", "cancel_pending"))
        assertEquals(Stage.DONE, stage("accepted", "refunded"))
        assertEquals(Stage.DONE, stage("accepted", "closed"))
        assertEquals(Stage.DONE, stage("accepted", "voided"))
        assertEquals(Stage.DONE, stage("accepted", "rejected"))
    }

    @Test
    fun anOrderWithoutAWebshopRecordCountsAsPending() {
        val order = webshopOrder(1, hubrise = "new", fiscal = null)

        assertEquals("pending", WebshopRules.fiscalOf(order))
        assertEquals("new", WebshopRules.statusOf(order.copy(hubriseStatus = null)))
    }

    @Test
    fun theOrdersAreGroupedByColumnKeepingTheirOrder() {
        val orders =
            listOf(
                webshopOrder(1, hubrise = "accepted", fiscal = "signed_n"),
                webshopOrder(2),
                webshopOrder(3, hubrise = "accepted", fiscal = "signed_n"),
                webshopOrder(4, hubrise = "completed", fiscal = "signed_n"),
            )

        val grouped = WebshopRules.byStage(orders)

        assertEquals(listOf(2L), grouped.getValue(Stage.NEW).map { it.id })
        assertEquals(listOf(1L, 3L), grouped.getValue(Stage.KITCHEN).map { it.id })
        assertEquals(emptyList<Long>(), grouped.getValue(Stage.READY).map { it.id })
        assertEquals(listOf(4L), grouped.getValue(Stage.DONE).map { it.id })
    }

    @Test
    fun theOwnWebshopIsToldApartFromTheAggregators() {
        assertTrue(WebshopRules.isOwnWebshop(webshopOrder(1, source = "Webshop")))
        assertTrue(WebshopRules.isOwnWebshop(webshopOrder(1, source = "web")))
        assertFalse(WebshopRules.isOwnWebshop(webshopOrder(1, source = "deliveroo")))
    }

    @Test
    fun theButtonsOfACardFollowItsState() {
        val new = webshopOrder(1)
        val kitchen = webshopOrder(2, hubrise = "accepted", fiscal = "signed_n")
        val readyUnpaid = webshopOrder(3, hubrise = "awaiting_collection", fiscal = "signed_p")
        val done = webshopOrder(4, hubrise = "completed", fiscal = "signed_n")

        assertTrue(WebshopRules.canReject(new))
        assertFalse(WebshopRules.canReject(kitchen))
        assertTrue(WebshopRules.canDelay(kitchen))
        assertFalse(WebshopRules.canDelay(new))
        assertTrue(WebshopRules.canReprint(kitchen))
        assertTrue(WebshopRules.canReprint(readyUnpaid))
        assertFalse(WebshopRules.canReprint(new))
        assertFalse(WebshopRules.canReprint(done))
        assertTrue(WebshopRules.canCollectByCard(readyUnpaid))
        assertFalse(WebshopRules.canCollectByCard(kitchen.copy(webshop = webshopState(2, "signed_p"))))
    }

    @Test
    fun theCaptionOfEachPrimaryActionIsAVueKey() {
        assertEquals("webshop_action_accept", PrimaryAction.ACCEPT.labelKey)
        assertEquals("webshop_action_prepare", PrimaryAction.PREPARE.labelKey)
        assertEquals("webshop_action_ready", PrimaryAction.READY.labelKey)
        assertEquals("webshop_action_out_for_delivery", PrimaryAction.OUT_FOR_DELIVERY.labelKey)
        assertEquals("webshop_action_complete", PrimaryAction.COMPLETE.labelKey)
        assertEquals("webshop_action_collect_cash", PrimaryAction.COLLECT_CASH.labelKey)
        assertEquals("webshop_action_refund", PrimaryAction.REFUND.labelKey)
        assertEquals("webshop_action_close_cancelled", PrimaryAction.CLOSE_CANCELLED.labelKey)
    }

    @Test
    fun theRefundIsOfferedToAManagerOnlyAndTheCloseToEveryone() {
        val refund = webshopOrder(1, hubrise = "cancelled", fiscal = "refund_pending")
        val close = webshopOrder(2, hubrise = "cancelled", fiscal = "cancel_pending")

        assertEquals(PrimaryAction.REFUND, WebshopRules.nextAction(refund, isManager = true))
        assertNull(WebshopRules.nextAction(refund, isManager = false))
        assertEquals(PrimaryAction.CLOSE_CANCELLED, WebshopRules.nextAction(close, isManager = false))
    }

    @Test
    fun anOrderToCheckListsItsReasonsWithoutDoublesAndFallsBackToOne() {
        val reasons =
            listOf(Attention.NoItems, Attention.UnknownProduct("sku"), Attention.UnknownProduct("other"), Attention.MissingServiceType)
        val review = webshopOrder(1, state = webshopState(1, "pending", resolution = "review", attention = reasons))
        val silent = webshopOrder(2, state = webshopState(2, "pending", resolution = "review"))

        assertTrue(WebshopRules.needsReview(review))
        assertEquals(
            listOf("webshop_attention_no_items", "webshop_attention_unknown_product", "webshop_attention_missing_service_type"),
            WebshopRules.reviewReasonKeys(review),
        )
        assertEquals(listOf("webshop_attention_unknown_product"), WebshopRules.reviewReasonKeys(silent))
        assertFalse(WebshopRules.needsReview(webshopOrder(3)))
        assertFalse(
            WebshopRules.needsReview(
                webshopOrder(4, hubrise = "accepted", fiscal = "signed_n", state = webshopState(4, "signed_n", "review")),
            ),
        )
    }

    @Test
    fun everyAttentionHasTheCodeThatNamesItsText() {
        val all =
            mapOf(
                Attention.UnknownChannel("c") to "unknown_channel",
                Attention.MissingServiceType to "missing_service_type",
                Attention.NoItems to "no_items",
                Attention.ItemWithoutRef("i") to "item_without_ref",
                Attention.UnknownProduct("p") to "unknown_product",
                Attention.UnknownOption("o", "r") to "unknown_option",
                Attention.UnknownCharge("c") to "unknown_charge",
                Attention.FractionalQuantity("q") to "fractional_quantity",
                Attention.BadPrice("b") to "bad_price",
                Attention.VatMismatch("v", "6", uniffi.pos_ffi.VatCode.A) to "vat_mismatch",
                Attention.Discount("d") to "discount",
                Attention.TotalMismatch(1L, 2L) to "total_mismatch",
                Attention.PaymentUnclear(1L, 2L) to "payment_unclear",
                Attention.ChangedAfterSigning to "changed_after_signing",
            )

        all.forEach { (attention, code) -> assertEquals(code, WebshopRules.attentionCode(attention)) }
    }

    @Test
    fun theNoticesOfASignedOrder() {
        val changed =
            webshopOrder(1, hubrise = "accepted", state = webshopState(1, "signed_n", attention = listOf(Attention.ChangedAfterSigning)))

        assertTrue(WebshopRules.changedAfterSigning(changed))
        assertFalse(
            WebshopRules.changedAfterSigning(
                webshopOrder(2, state = webshopState(2, "pending", attention = listOf(Attention.ChangedAfterSigning))),
            ),
        )
        assertEquals(
            "webshop_cancelled_refund",
            WebshopRules.cancelledBannerKey(webshopOrder(3, fiscal = "refund_pending"), isManager = true),
        )
        assertEquals(
            "webshop_manager_needed",
            WebshopRules.cancelledBannerKey(webshopOrder(3, fiscal = "refund_pending"), isManager = false),
        )
        assertEquals(
            "webshop_cancelled_close",
            WebshopRules.cancelledBannerKey(webshopOrder(4, fiscal = "cancel_pending"), isManager = false),
        )
        assertNull(WebshopRules.cancelledBannerKey(webshopOrder(5), isManager = true))
    }

    @Test
    fun thePaymentBadgeSaysPaidOrToCollect() {
        assertEquals(PaymentBadge.PAID, WebshopRules.paymentBadge(webshopOrder(1, fiscal = "signed_n")))
        assertEquals(
            PaymentBadge.PAID,
            WebshopRules.paymentBadge(webshopOrder(2, state = webshopState(2, "pending", paymentMode = "prepaid"))),
        )
        assertEquals(
            PaymentBadge.DUE,
            WebshopRules.paymentBadge(webshopOrder(3, state = webshopState(3, "signed_p", paymentMode = "unpaid"))),
        )
        assertNull(WebshopRules.paymentBadge(webshopOrder(4)))
    }

    @Test
    fun theServiceHasAVueKey() {
        assertEquals("webshop_service_delivery", WebshopRules.serviceLabelKey(webshopOrder(1, service = "delivery")))
        assertEquals("webshop_service_collection", WebshopRules.serviceLabelKey(webshopOrder(1, service = "collection")))
        assertEquals("webshop_service_eat_in", WebshopRules.serviceLabelKey(webshopOrder(1, service = "eat_in")))
        assertEquals("webshop_service_unknown", WebshopRules.serviceLabelKey(webshopOrder(1, service = null)))
    }

    @Test
    fun theAddressIsOnlyForAnOwnWebshopDelivery() {
        val address = listOf("Rue Neuve 5", "bte 2", "4000", "Liège")
        val own = webshopOrder(1, source = "Webshop", service = "delivery", address = address)

        assertEquals("Rue Neuve 5 bte 2, 4000 Liège", WebshopRules.deliveryAddress(own))
        assertNull(WebshopRules.deliveryAddress(own.copy(source = "ubereats")))
        assertNull(WebshopRules.deliveryAddress(own.copy(serviceType = "collection")))
        assertNull(WebshopRules.deliveryAddress(webshopOrder(2, source = "Webshop", service = "delivery")))
        assertEquals(
            "4000 Liège",
            WebshopRules.deliveryAddress(
                webshopOrder(3, source = "Webshop", service = "delivery", address = listOf(null, " ", "4000", "Liège")),
            ),
        )
    }

    @Test
    fun anUnansweredOrderIsLateAfterFiveMinutes() {
        val now = Instant.parse("2026-10-09T10:30:00Z")
        val fresh = webshopOrder(1, createdAt = "2026-10-09T10:26:00Z")
        val old = webshopOrder(2, createdAt = "2026-10-09T10:25:00Z")
        val signed = webshopOrder(3, hubrise = "accepted", fiscal = "signed_n", createdAt = "2026-10-09T09:00:00Z")

        assertEquals(4L, WebshopRules.waitingMinutes(fresh, now))
        assertFalse(WebshopRules.isLate(fresh, now))
        assertTrue(WebshopRules.isLate(old, now))
        assertFalse(WebshopRules.isLate(signed, now))
        assertNull(WebshopRules.waitingMinutes(fresh.copy(createdAt = "later"), now))
        assertEquals(0L, WebshopRules.waitingMinutes(fresh.copy(createdAt = "2026-10-09T11:00:00Z"), now))
    }

    @Test
    fun theKeyTimeIsTheOneThatMattersForTheColumn() {
        val now = Instant.parse("2026-10-09T10:30:00Z")
        val waiting = webshopOrder(1, createdAt = "2026-10-09T10:20:00Z")
        val kitchen = webshopOrder(2, hubrise = "accepted", fiscal = "signed_n", confirmed = "2026-10-09T10:45:00Z")
        val ready = webshopOrder(3, hubrise = "awaiting_collection", fiscal = "signed_n", confirmed = "2026-10-09T10:40:00Z")
        val delivery = webshopOrder(4, hubrise = "in_delivery", fiscal = "signed_n")
        val rejected = webshopOrder(5, hubrise = "rejected", fiscal = "rejected")

        assertEquals(KeyTime.Waiting(10L, late = true), WebshopRules.keyTime(waiting, now))
        assertEquals(KeyTime.ReadyAt("12:45"), WebshopRules.keyTime(kitchen, now))
        assertEquals(KeyTime.Ready("12:40"), WebshopRules.keyTime(ready, now))
        assertEquals(KeyTime.InDelivery, WebshopRules.keyTime(delivery, now))
        assertEquals(KeyTime.Finished("webshop_status_rejected"), WebshopRules.keyTime(rejected, now))
        assertNull(WebshopRules.keyTime(kitchen.copy(confirmedTime = null), now))
        assertNull(WebshopRules.keyTime(waiting.copy(createdAt = "?"), now))
    }

    @Test
    fun theOutcomeOfAFinishedOrderHasAVueKey() {
        assertEquals("webshop_status_refunded", WebshopRules.doneLabelKey(webshopOrder(1, hubrise = "cancelled", fiscal = "refunded")))
        assertEquals("webshop_status_rejected", WebshopRules.doneLabelKey(webshopOrder(2, hubrise = "rejected", fiscal = "signed_n")))
        assertEquals("webshop_status_cancelled", WebshopRules.doneLabelKey(webshopOrder(3, hubrise = "cancelled", fiscal = "signed_n")))
        assertEquals("webshop_status_completed", WebshopRules.doneLabelKey(webshopOrder(4, hubrise = "completed", fiscal = "signed_n")))
    }

    @Test
    fun clockFormatsBrusselsTimeAndAnswersNullWhenUnreadable() {
        assertEquals("12:45", WebshopRules.clock("2026-10-09T10:45:00Z"))
        assertNull(WebshopRules.clock(null))
        assertNull(WebshopRules.clock("soon"))
    }
}
