package dev.nuagemagique.pilipos.platform.watchdog

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import org.json.JSONArray
import org.json.JSONObject
import timber.log.Timber
import java.io.File
import javax.inject.Inject
import javax.inject.Singleton

/**
 * How many times the watchdog may restart the process (PD207): at most [MAX_RESTARTS] in [WINDOW_MS].
 *
 * The last [KEPT] restart times (epoch milliseconds, wall clock) are kept in `filesDir/platform/restarts.json`,
 * because the process dies at each restart. A missing or corrupt file is an empty budget. A time in the future
 * (the clock was set back) does not count.
 */
@Singleton
class RestartBudget internal constructor(
    private val file: File,
) {
    @Inject
    constructor(
        @ApplicationContext context: Context,
    ) : this(File(File(context.filesDir, PlatformAudit.DIR_NAME), FILE_NAME))

    /** True when fewer than [MAX_RESTARTS] restarts happened in the [WINDOW_MS] before [now]. */
    @Synchronized
    fun allow(now: Long): Boolean = load().count { it in (now - WINDOW_MS + 1)..now } < MAX_RESTARTS

    /** Records a restart at [now]; a write failure is logged and the restart goes on. */
    @Synchronized
    fun record(now: Long) {
        try {
            val times = (load() + now).takeLast(KEPT)
            file.parentFile?.mkdirs()
            file.writeText(JSONObject().put("restarts", JSONArray(times)).toString())
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "RestartBudget: record failed")
        }
    }

    /** The most recent recorded restart, or null. */
    @Synchronized
    fun lastRestart(): Long? = load().maxOrNull()

    private fun load(): List<Long> =
        try {
            if (!file.isFile) {
                emptyList()
            } else {
                val array = JSONObject(file.readText()).getJSONArray("restarts")
                List(array.length()) { array.getLong(it) }
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "RestartBudget: file unreadable, empty budget")
            emptyList()
        }

    companion object {
        /** Name of the persisted file under `filesDir/platform`. */
        const val FILE_NAME = "restarts.json"

        /** Restarts allowed in the window. */
        const val MAX_RESTARTS = 3

        /** The window: 10 minutes. */
        const val WINDOW_MS: Long = 10L * 60L * 1000L

        /** Restart times kept in the file. */
        const val KEPT = 10
    }
}
