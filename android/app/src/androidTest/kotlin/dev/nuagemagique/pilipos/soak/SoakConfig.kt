package dev.nuagemagique.pilipos.soak

import android.os.Bundle

/**
 * The load profile of the soak, read from the instrumentation arguments (P7D12).
 *
 * `hours` (required, decimal), `dayMinutes` (60), `salesPerDay` (100), `managerInsz` / `managerPin` (the seeded
 * manager), `seed` (random seed of the sale mix), `idleGapMinutes` (6, must exceed the 5 minutes of the engine's idle
 * lock) and `splitCardCents` (the literal card part of a split payment, 100 cents: a figure of the profile, never
 * computed from an order).
 */
internal data class SoakConfig(
    val hours: Double,
    val dayMinutes: Int,
    val salesPerDay: Int,
    val insz: String,
    val pin: String,
    val seed: Long,
    val idleGapMinutes: Int,
    val splitCardCents: Long,
) {
    /** The whole run, in milliseconds. */
    val totalMs: Long get() = (hours * MS_PER_HOUR).toLong()

    /** One compressed service day, in milliseconds. */
    val dayMs: Long get() = dayMinutes * MS_PER_MINUTE

    /** The idle gap of a day, in milliseconds. */
    val gapMs: Long get() = idleGapMinutes * MS_PER_MINUTE

    init {
        require(hours > 0.0) { "hours must be positive" }
        require(salesPerDay >= MIN_SALES) { "salesPerDay must be at least $MIN_SALES" }
        require(idleGapMinutes * SECONDS_PER_MINUTE > IDLE_LOCK_SECONDS) { "idleGapMinutes must exceed the idle lock of 5 minutes" }
        require(dayMs > gapMs + MIN_WORK_MS) { "dayMinutes must leave at least a minute of work beside the idle gap" }
    }

    companion object {
        private const val MS_PER_HOUR = 3_600_000.0
        private const val MS_PER_MINUTE = 60_000L
        private const val SECONDS_PER_MINUTE = 60
        private const val IDLE_LOCK_SECONDS = 300
        private const val MIN_WORK_MS = 60_000L
        private const val MIN_SALES = 4

        private const val DEFAULT_DAY_MINUTES = 60
        private const val DEFAULT_SALES = 100
        private const val DEFAULT_GAP_MINUTES = 6
        private const val DEFAULT_SPLIT_CARD_CENTS = 100L
        private const val DEFAULT_INSZ = "94060242576"
        private const val DEFAULT_PIN = "1111"

        /** Reads the profile; a missing `hours` or a malformed number fails the run at once. */
        fun from(args: Bundle): SoakConfig =
            SoakConfig(
                hours = args.getString("hours")?.toDouble() ?: throw AssertionError("the argument hours is missing"),
                dayMinutes = args.getString("dayMinutes")?.toInt() ?: DEFAULT_DAY_MINUTES,
                salesPerDay = args.getString("salesPerDay")?.toInt() ?: DEFAULT_SALES,
                insz = args.getString("managerInsz") ?: DEFAULT_INSZ,
                pin = args.getString("managerPin") ?: DEFAULT_PIN,
                seed = args.getString("seed")?.toLong() ?: System.nanoTime(),
                idleGapMinutes = args.getString("idleGapMinutes")?.toInt() ?: DEFAULT_GAP_MINUTES,
                splitCardCents = args.getString("splitCardCents")?.toLong() ?: DEFAULT_SPLIT_CARD_CENTS,
            )
    }
}
