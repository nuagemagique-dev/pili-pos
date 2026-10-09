package dev.nuagemagique.pilipos.platform.audit

import android.content.Context
import android.content.pm.PackageManager
import android.os.Process
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.di.IoDispatcher
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONObject
import timber.log.Timber
import java.io.File
import java.io.FileOutputStream
import java.time.Instant
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The platform audit log (PD208): `filesDir/platform/audit.jsonl`, one JSON object per line.
 *
 * Each line carries `ts` (ISO 8601 in the Europe/Brussels zone, never the runtime zone), `kind`, `event`,
 * `detail` (a string map), `versionCode`, `versionName` and `pid`. Every append is fsynced. When the file
 * reaches [MAX_BYTES] it is rotated: `audit.jsonl` becomes `audit.jsonl.1`, `.1` becomes `.2` and so on, and
 * [KEPT_ROTATIONS] rotated files are kept. This is not the fiscal journal and is never sent anywhere; it is
 * part of the SPF export (see [files]).
 *
 * [append] never throws: a failure is logged with Timber and the caller goes on.
 */
@Singleton
class PlatformAudit internal constructor(
    private val dir: File,
    private val clock: PlatformClock,
    private val ioDispatcher: CoroutineDispatcher,
    private val versions: () -> Pair<Long, String>,
    private val pid: () -> Int,
) {
    @Inject
    constructor(
        @ApplicationContext context: Context,
        @IoDispatcher ioDispatcher: CoroutineDispatcher,
    ) : this(
        dir = File(context.filesDir, DIR_NAME),
        clock = SystemPlatformClock,
        ioDispatcher = ioDispatcher,
        versions = { packageVersions(context) },
        pid = { Process.myPid() },
    )

    private val mutex = Mutex()

    private val current: File get() = File(dir, FILE_NAME)

    /** Appends one line; the caller is never given an error. */
    suspend fun append(
        kind: String,
        event: String,
        detail: Map<String, String> = emptyMap(),
    ) {
        try {
            mutex.withLock {
                withContext(ioDispatcher) { write(kind, event, detail) }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "PlatformAudit: append failed (%s/%s)", kind, event)
        }
    }

    /** The existing log files, the current one first, then `.1` (the newest rotated) to `.5`. */
    fun files(): List<File> = (listOf(current) + (1..KEPT_ROTATIONS).map { rotated(it) }).filter { it.isFile }

    private fun write(
        kind: String,
        event: String,
        detail: Map<String, String>,
    ) {
        dir.mkdirs()
        val (versionCode, versionName) = versions()
        val line =
            JSONObject()
                .put("ts", format(clock.nowMillis()))
                .put("kind", kind)
                .put("event", event)
                .put("detail", JSONObject(detail))
                .put("versionCode", versionCode)
                .put("versionName", versionName)
                .put("pid", pid())
                .toString() + "\n"
        FileOutputStream(current, true).use { out ->
            out.write(line.toByteArray(Charsets.UTF_8))
            out.flush()
            out.fd.sync()
        }
        if (current.length() >= MAX_BYTES) rotate()
    }

    private fun rotate() {
        rotated(KEPT_ROTATIONS).delete()
        for (i in KEPT_ROTATIONS - 1 downTo 1) {
            val from = rotated(i)
            if (from.exists()) from.renameTo(rotated(i + 1))
        }
        current.renameTo(rotated(1))
    }

    private fun rotated(index: Int): File = File(dir, "$FILE_NAME.$index")

    companion object {
        /** Directory of the platform files under `filesDir`. */
        const val DIR_NAME = "platform"

        /** Name of the current log file. */
        const val FILE_NAME = "audit.jsonl"

        /** Size at which the current file is rotated. */
        const val MAX_BYTES: Long = 1024L * 1024L

        /** Number of rotated files kept. */
        const val KEPT_ROTATIONS = 5

        /** The time zone of every timestamp written by the platform package. */
        val ZONE: ZoneId = ZoneId.of("Europe/Brussels")

        private val FORMAT: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss.SSSxxx")

        /** Formats [epochMillis] as ISO 8601 local time in Europe/Brussels with its offset. */
        fun format(epochMillis: Long): String = ZonedDateTime.ofInstant(Instant.ofEpochMilli(epochMillis), ZONE).format(FORMAT)

        private fun packageVersions(context: Context): Pair<Long, String> =
            try {
                val info = context.packageManager.getPackageInfo(context.packageName, PackageManager.PackageInfoFlags.of(0L))
                info.longVersionCode to (info.versionName ?: "unknown")
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "PlatformAudit: version unavailable")
                0L to "unknown"
            }
    }
}
