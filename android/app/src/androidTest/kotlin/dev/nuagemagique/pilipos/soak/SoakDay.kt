package dev.nuagemagique.pilipos.soak

import uniffi.pos_ffi.PosException
import java.util.Random

/** What a report, a work-out or a logout came to: its success, its number when it has one, and the engine's error. */
internal data class ReportOutcome(
    val ok: Boolean,
    val number: Long?,
    val error: String?,
)

/**
 * One compressed service day: log in (which clocks the manager in), [SoakConfig.salesPerDay] sales spread over
 * the day, an X report at mid-day, one idle gap in which the engine's idle lock fires, log in again, then work
 * out and the Z at the end of the day.
 */
internal class SoakDay(
    private val config: SoakConfig,
    private val engine: SoakEngine,
    private val pacer: SoakPacer,
    private val log: SoakLog,
    private val stats: SoakStats,
    private val rng: Random,
) {
    private val sales = SoakSales(engine, config, rng)
    private val refundable = ArrayList<RefundableSale>()

    /** Runs day number [day]; returns when the day is over (its Z is made) or the run must stop. */
    fun run(day: Int) {
        stats.day = day
        val start = pacer.elapsed()
        log.event("day", "SOAK_DAY begin day=$day", mapOf("day" to day, "phase" to "begin"))
        refundable.clear()
        pacer.keepAlive = true
        ensureSession()
        val kinds = SoakMix.plan(config.salesPerDay, rng)
        val slot = (config.dayMs - config.gapMs - RESERVE_MS) / kinds.size
        val xAfter = maxOf(1, kinds.size / 2)
        val gapAfter = maxOf(xAfter + 1, kinds.size * GAP_NUMERATOR / GAP_DENOMINATOR)
        for ((index, kind) in kinds.withIndex()) {
            if (!pacer.sleepUntil(start + index * slot)) break
            takeSale(kind)
            if (index + 1 == xAfter) xReport()
            if (index + 1 == gapAfter) idleGap()
        }
        closeDay(day)
        pacer.sleepUntil(start + config.dayMs)
    }

    private fun ensureSession() {
        if (engine.call { it.getCurrentUser() } == null) engine.call { it.login(config.insz, config.pin) }
    }

    private fun takeSale(kind: SaleKind) {
        val outcome =
            when {
                kind == SaleKind.REFUND && refundable.isNotEmpty() -> sales.refund(refundable.removeAt(rng.nextInt(refundable.size)))
                kind == SaleKind.REFUND || kind == SaleKind.TAKEAWAY -> sales.takeaway()
                kind == SaleKind.TABLE -> sales.table()
                else -> sales.split()
            }
        remember(outcome)
        if (outcome.ok) stats.salesOk++ else stats.salesFailed++
        val verdict = if (outcome.ok) "ok" else "fail"
        log.event(
            "sale",
            "SOAK_SALE $verdict ${outcome.kind.label} ${outcome.ticket ?: "-"}",
            mapOf(
                "ok" to outcome.ok,
                "kind" to outcome.kind.label,
                "ticket" to outcome.ticket,
                "fiscalEventId" to outcome.fiscalEventId,
                "orderId" to outcome.orderId,
                "error" to outcome.error,
                "day" to stats.day,
            ),
        )
    }

    /** A signed sale that is not a refund can be refunded later the same day. */
    private fun remember(outcome: SaleOutcome) {
        val fiscalEventId = outcome.fiscalEventId
        val orderId = outcome.orderId
        if (!outcome.ok || outcome.kind == SaleKind.REFUND) return
        if (fiscalEventId != null && orderId != null) refundable.add(RefundableSale(fiscalEventId, orderId))
    }

    private fun xReport() {
        report("x", "SOAK_X") {
            val r = engine.call { it.generateXReport(config.insz) }
            ReportOutcome(r.success, r.reportNo, r.error)
        }
    }

    private fun report(
        type: String,
        label: String,
        make: () -> ReportOutcome,
    ): Boolean {
        val outcome = failSafe(make)
        log.event(
            type,
            "$label ${if (outcome.ok) "ok" else "fail"} ${outcome.number ?: "-"}",
            mapOf("ok" to outcome.ok, "reportNo" to outcome.number, "error" to outcome.error, "day" to stats.day),
        )
        return outcome.ok
    }

    /** The idle gap: no activity for [SoakConfig.gapMs]; the engine must lock the session by itself. */
    private fun idleGap() {
        pacer.keepAlive = false
        val start = pacer.elapsed()
        var lockedAt: Long? = null
        var lastCheck = start
        val watch = {
            if (lockedAt == null && pacer.elapsed() - lastCheck >= LOCK_CHECK_MS) {
                lastCheck = pacer.elapsed()
                if (engine.call { it.getCurrentUser() } == null) lockedAt = lastCheck - start
            }
        }
        val completed = pacer.sleepUntil(start + config.gapMs, watch)
        pacer.keepAlive = true
        val locked = lockedAt
        if (!completed && locked == null) {
            log.event("lock", "SOAK_LOCK skipped (run stopped inside the gap)", mapOf("ok" to null, "skipped" to true, "day" to stats.day))
            return
        }
        val seconds = locked?.div(MS)
        log.event(
            "lock",
            if (locked != null) "SOAK_LOCK ok after=${seconds}s" else "SOAK_LOCK fail (still logged in after the gap)",
            mapOf("ok" to (locked != null), "afterSeconds" to seconds, "day" to stats.day),
        )
        ensureSession()
    }

    /** Work out, the Z report and the end of the session. */
    private fun closeDay(day: Int) {
        report("workout", "SOAK_WORKOUT") {
            val r = engine.call { it.workOut(config.insz) }
            ReportOutcome(r.success, null, r.error)
        }
        val zDone =
            report("z", "SOAK_Z") {
                val r = engine.call { it.generateZReport(config.insz) }
                ReportOutcome(r.success, r.reportNo, r.error)
            }
        if (zDone) stats.zDone++
        failSafe {
            engine.call { it.logout() }
            ReportOutcome(true, null, null)
        }
        log.event("day", "SOAK_DAY end day=$day", mapOf("day" to day, "phase" to "end"))
    }

    @Suppress("TooGenericExceptionCaught")
    private fun failSafe(block: () -> ReportOutcome): ReportOutcome =
        try {
            block()
        } catch (e: PosException) {
            ReportOutcome(false, null, "${e::class.simpleName}: ${e.message}")
        } catch (e: RuntimeException) {
            ReportOutcome(false, null, "${e::class.simpleName}: ${e.message}")
        }

    private companion object {
        const val RESERVE_MS = 20_000L
        const val GAP_NUMERATOR = 3
        const val GAP_DENOMINATOR = 5
        const val MS = 1000L
        const val LOCK_CHECK_MS = 5_000L
    }
}
