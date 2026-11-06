package dev.nuagemagique.pilipos.ui.feedback

import dev.nuagemagique.pilipos.feature.webshop.WebshopRules
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.map
import uniffi.pos_ffi.WebshopOrderSummary
import java.time.Instant

/** The chime of the late orders repeats at this interval, in milliseconds (`LATE_CHIME_EVERY_MS` of `webshopLate.ts`). */
internal const val LATE_CHIME_EVERY_MS: Long = 120_000L

/** How often the age of the waiting orders is looked at again, in milliseconds (`LATE_CLOCK_TICK_MS` of `webshopOrders.ts`). */
internal const val LATE_TICK_MS: Long = 5_000L

/** The number of late online orders at one moment. */
internal data class LateSample(
    val count: Int,
    val nowMs: Long,
)

/** The orders that waited for [WebshopRules] "late" (5 minutes) or more without an answer at [now]. */
internal fun lateOrderCount(
    orders: List<WebshopOrderSummary>,
    now: Instant,
): Int = orders.count { WebshopRules.isLate(it, now) }

/**
 * True when the late chime sounds: some order is late ([count] above 0) and either no chime sounded since the
 * orders became late ([lastChimeAtMs] null) or the last one is [LATE_CHIME_EVERY_MS] old (`lateChimeDue`).
 */
internal fun lateChimeDue(
    count: Int,
    lastChimeAtMs: Long?,
    nowMs: Long,
): Boolean = count > 0 && (lastChimeAtMs == null || nowMs - lastChimeAtMs >= LATE_CHIME_EVERY_MS)

/** True for an order that is not answered yet: it may become late, so the clock has to run. */
private fun mayBecomeLate(order: WebshopOrderSummary): Boolean = WebshopRules.canReject(order)

/**
 * The late count of [orders], looked at again every [tickMs] while an unanswered order exists (an idle board runs
 * no clock). Each tick emits, even when the count did not change: the chime repeats on the ticks.
 */
@OptIn(ExperimentalCoroutinesApi::class)
internal fun lateSamples(
    orders: Flow<List<WebshopOrderSummary>>,
    now: () -> Instant,
    tickMs: Long = LATE_TICK_MS,
): Flow<LateSample> =
    orders.flatMapLatest { list ->
        flow {
            if (list.none(::mayBecomeLate)) {
                emit(LateSample(0, now().toEpochMilli()))
            } else {
                while (true) {
                    val at = now()
                    emit(LateSample(lateOrderCount(list, at), at.toEpochMilli()))
                    delay(tickMs)
                }
            }
        }
    }

/** The late count alone, for the banner (it changes rarely). */
internal fun lateCounts(
    orders: Flow<List<WebshopOrderSummary>>,
    now: () -> Instant,
    tickMs: Long = LATE_TICK_MS,
): Flow<Int> = lateSamples(orders, now, tickMs).map { it.count }.distinctUntilChanged()

/**
 * Plays [player] when the late chime is due (see [lateChimeDue]): at once when orders become late, then every
 * [LATE_CHIME_EVERY_MS] while any is late; it starts over when none is late.
 */
internal suspend fun playLateChime(
    samples: Flow<LateSample>,
    player: OrderChimePlayer,
) {
    var last: Long? = null
    samples.collect { sample ->
        if (sample.count == 0) {
            last = null
        } else if (lateChimeDue(sample.count, last, sample.nowMs)) {
            player.play()
            last = sample.nowMs
        }
    }
}
