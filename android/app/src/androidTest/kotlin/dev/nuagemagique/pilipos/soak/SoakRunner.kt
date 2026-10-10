package dev.nuagemagique.pilipos.soak

import java.util.Random

/**
 * The soak loop: compressed service days until `hours` have elapsed or the stop file exists, then one clean
 * wind-down (the day in progress is worked out and closed with its Z). Writes `start`, `beat`, `sale`, `x`, `z`,
 * `lock`, `day`, `error` and `end` records (see [SoakLog]).
 */
internal class SoakRunner(
    private val engine: SoakEngine,
) {
    /** Runs the soak described by the instrumentation arguments. */
    @Suppress("TooGenericExceptionCaught")
    fun run() {
        val config = SoakConfig.from(engine.arguments)
        val log = SoakLog(engine.soakDir())
        val stats = SoakStats()
        log.event(
            "start",
            "SOAK_START hours=${config.hours} dayMinutes=${config.dayMinutes} salesPerDay=${config.salesPerDay} seed=${config.seed}",
            mapOf(
                "pid" to android.os.Process.myPid(),
                "hours" to config.hours,
                "dayMinutes" to config.dayMinutes,
                "salesPerDay" to config.salesPerDay,
                "seed" to config.seed,
            ),
        )
        var reason = "aborted"
        try {
            engine.awaitReady()
            engine.openMain()
            reason = loop(config, engine, log, stats)
        } catch (e: RuntimeException) {
            val text = "${e::class.simpleName}: ${e.message}"
            log.event("error", "SOAK_ERROR $text", mapOf("error" to text))
        } catch (e: AssertionError) {
            log.event("error", "SOAK_ERROR ${e.message}", mapOf("error" to e.message))
        } finally {
            log.event(
                "end",
                "SOAK_END reason=$reason days=${stats.day} salesOk=${stats.salesOk} salesFailed=${stats.salesFailed} z=${stats.zDone}",
                mapOf(
                    "reason" to reason,
                    "days" to stats.day,
                    "salesOk" to stats.salesOk,
                    "salesFailed" to stats.salesFailed,
                    "z" to stats.zDone,
                ),
            )
        }
    }

    @Suppress("TooGenericExceptionCaught")
    private fun loop(
        config: SoakConfig,
        engine: SoakEngine,
        log: SoakLog,
        stats: SoakStats,
    ): String {
        val pacer = SoakPacer(log, engine, stats, config.totalMs)
        val rng = Random(config.seed)
        var failedDays = 0
        var day = 0
        pacer.tick()
        while (!pacer.finished() && failedDays < MAX_FAILED_DAYS) {
            day++
            try {
                SoakDay(config, engine, pacer, log, stats, rng).run(day)
                failedDays = 0
            } catch (e: RuntimeException) {
                failedDays++
                val text = "${e::class.simpleName}: ${e.message}"
                log.event("error", "SOAK_ERROR day=$day $text", mapOf("error" to text, "day" to day))
            }
        }
        return if (pacer.finished()) pacer.reason() else "aborted"
    }

    private companion object {
        const val MAX_FAILED_DAYS = 3
    }
}
