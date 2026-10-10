package dev.nuagemagique.pilipos.soak

import android.os.SystemClock
import uniffi.pos_ffi.PosException

/** The counters that the heartbeat and the end record report. */
internal class SoakStats {
    @Volatile var day = 0

    @Volatile var salesOk = 0

    @Volatile var salesFailed = 0

    @Volatile var zDone = 0

    @Volatile var beats = 0
}

/**
 * The clock of the run: sleeps in short slices so that the heartbeat (`SOAK_BEAT` every 10 s) keeps going while
 * the driver waits, tells the engine that the operator is active (the engine's idle lock counts from the last
 * `userActivity`, and the driver touches no screen), and watches the stop file.
 */
internal class SoakPacer(
    private val log: SoakLog,
    private val engine: SoakEngine,
    private val stats: SoakStats,
    private val totalMs: Long,
) {
    private val startedAt = SystemClock.elapsedRealtime()
    private var lastBeat = 0L
    private var lastActivity = 0L
    private var stopSeen = false

    /** True while the driver works: the engine is told once in a while that the operator is active. */
    @Volatile var keepAlive = true

    /** Milliseconds since the start of the run. */
    fun elapsed(): Long = SystemClock.elapsedRealtime() - startedAt

    /** True once the stop file exists. */
    fun stopRequested(): Boolean {
        if (!stopSeen) stopSeen = log.stopFile.exists()
        return stopSeen
    }

    /** True when the run must wind down: the hours are over or the stop file exists. */
    fun finished(): Boolean = elapsed() >= totalMs || stopRequested()

    /** Why [finished] is true. */
    fun reason(): String = if (stopRequested()) "stop" else "hours"

    /** Emits the heartbeat and the activity ping when they are due. Call it between two engine calls. */
    fun tick() {
        val now = elapsed()
        if (keepAlive && now - lastActivity >= ACTIVITY_MS) {
            lastActivity = now
            ping()
        }
        if (now - lastBeat >= BEAT_MS) {
            lastBeat = now
            stats.beats++
            log.event(
                "beat",
                "SOAK_BEAT n=${stats.beats} up=${now / MS}s day=${stats.day} ok=${stats.salesOk} fail=${stats.salesFailed}",
                mapOf("n" to stats.beats, "up" to now / MS, "day" to stats.day, "ok" to stats.salesOk, "fail" to stats.salesFailed),
            )
        }
    }

    /**
     * Waits until [deadlineMs] (a value of [elapsed]) while ticking; [watch] runs once per slice. Returns false
     * when [finished] cut the wait short.
     */
    fun sleepUntil(
        deadlineMs: Long,
        watch: () -> Unit = {},
    ): Boolean {
        while (elapsed() < deadlineMs) {
            if (finished()) return false
            tick()
            watch()
            Thread.sleep(minOf(SLICE_MS, maxOf(1L, deadlineMs - elapsed())))
        }
        return true
    }

    private fun ping() {
        try {
            engine.call { it.userActivity() }
        } catch (e: PosException) {
            log.line("userActivity refused: ${e.message}")
        }
    }

    private companion object {
        const val MS = 1000L
        const val BEAT_MS = 10_000L
        const val ACTIVITY_MS = 5_000L
        const val SLICE_MS = 250L
    }
}
