package dev.nuagemagique.pilipos.platform

import android.content.Context
import android.content.Intent
import android.database.sqlite.SQLiteDatabase
import android.os.SystemClock
import android.os.storage.StorageManager
import android.util.Log
import androidx.test.platform.app.InstrumentationRegistry
import dagger.hilt.android.EntryPointAccessors
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.debugtools.E2eEntryPoint
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.platform.export.ExportEntryPoint
import dev.nuagemagique.pilipos.platform.update.UpdateScreenEntryPoint
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest

/**
 * Shared helpers of the platform end-to-end test (P6-T11): the Hilt entry points of the app under test, the
 * `PLATE2E_*` result lines that `scripts/android/plat_e2e.sh` reads from logcat, file sizes and hashes.
 *
 * The result lines go to logcat with the tag [TAG]; the host script greps them, so their text is a contract.
 */
internal class PlatformE2eEnv {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()

    /** The context of the app under test. */
    val context: Context get() = instrumentation.targetContext

    private val arguments get() = InstrumentationRegistry.getArguments()

    /** The INSZ of the seeded manager (`managerInsz`). */
    val insz: String get() = arguments.getString("managerInsz") ?: DEFAULT_INSZ

    /** The PIN of the seeded manager (`managerPin`). */
    val pin: String get() = arguments.getString("managerPin") ?: DEFAULT_PIN

    /** The number of sales of the `sales` step (`sales`). */
    val sales: Int get() = arguments.getString("sales")?.toInt() ?: DEFAULT_SALES

    /** The number of sales of the second phase of the `sales` step (`sales2`), made of several lines each. */
    val salesMultiLine: Int get() = arguments.getString("sales2")?.toInt() ?: DEFAULT_SALES_MULTI

    /** The number of lines of each sale of the second phase (`lines2`). */
    val linesPerMultiLineSale: Int get() = arguments.getString("lines2")?.toInt() ?: DEFAULT_LINES_MULTI

    /** Which refusal the `update_refusals` step expects (`refusal`). */
    val refusal: String? get() = arguments.getString("refusal")

    /** The device port of the registry mock (`registryPort`, default 19392). */
    val registryPort: Int? get() = arguments.getString("registryPort")?.toInt()

    /** The singletons of the real Hilt graph that the debug source set exposes. */
    val app: E2eEntryPoint
        get() = EntryPointAccessors.fromApplication(context.applicationContext, E2eEntryPoint::class.java)

    /** The SPF export collaborators. */
    val export: ExportEntryPoint
        get() = EntryPointAccessors.fromApplication(context.applicationContext, ExportEntryPoint::class.java)

    /** The updater collaborators. */
    val update: UpdateScreenEntryPoint
        get() = EntryPointAccessors.fromApplication(context.applicationContext, UpdateScreenEntryPoint::class.java)

    /** `filesDir/engine/pos.db`. */
    val posDb: File get() = File(context.filesDir, "engine/pos.db")

    /** `filesDir/engine/backups`. */
    val backups: File get() = File(context.filesDir, "engine/backups")

    /** `filesDir/platform`. */
    val platformDir: File get() = File(context.filesDir, "platform")

    /**
     * Brings the application to the front the way the package-replaced receiver does (a new task of the same
     * package, which Lock Task allows). The steps other than the first do not use an activity scenario: the
     * kiosk keeps the activity in front, and a scenario could not close it.
     */
    fun openMain() {
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    /** Waits until the engine is Ready (or fails when it becomes Fatal). */
    fun awaitReady(): EngineState.Ready {
        val state =
            runBlocking {
                withTimeout(READY_TIMEOUT_MS) { app.holder().state.first { it is EngineState.Ready || it is EngineState.Fatal } }
            }
        return state as? EngineState.Ready ?: throw AssertionError("the engine did not become Ready: $state")
    }

    /** Logs the manager in through the engine (a new process has no session). */
    fun login() {
        awaitReady()
        val user = runBlocking { app.gateway().call { it.login(insz, pin) } }
        check(user.insz == insz) { "logged in as ${user.insz}, expected $insz" }
    }

    /** Polls [condition] until it holds; fails with [what] after [timeoutMs]. */
    fun waitFor(
        what: String,
        timeoutMs: Long,
        condition: () -> Boolean,
    ) {
        val deadline = SystemClock.elapsedRealtime() + timeoutMs
        while (!condition()) {
            if (SystemClock.elapsedRealtime() > deadline) throw AssertionError("timeout after $timeoutMs ms: $what")
            Thread.sleep(POLL_MS)
        }
    }

    /**
     * The app-specific directory (`getExternalFilesDirs`) of the removable, non-primary volume, and the uuid of
     * that volume.
     */
    fun removableAppDir(): Pair<File, String> {
        val storage = context.getSystemService(StorageManager::class.java)
        for (dir in context.getExternalFilesDirs(null)) {
            if (dir == null) continue
            val volume = storage.getStorageVolume(dir) ?: continue
            val uuid = volume.uuid
            if (volume.isRemovable && !volume.isPrimary && uuid != null) return dir to uuid
        }
        throw AssertionError("no mounted removable non-primary volume with an app-specific directory")
    }

    /** The audit files, the current one first. */
    fun auditRows(): List<JSONObject> {
        val files = listOf("audit.jsonl") + (1..AUDIT_ROTATIONS).map { "audit.jsonl.$it" }
        return files.map { File(platformDir, it) }.filter { it.isFile }.flatMap { file ->
            file.readLines().filter { it.isNotBlank() }.map { JSONObject(it) }
        }
    }

    /** Size of the database file and of its write-ahead log, summed (no checkpoint is forced). */
    fun dbBytes(): Long = posDb.length() + File(posDb.path + "-wal").length()

    /**
     * The size SQLite itself reports for the database as readers see it, WAL frames included:
     * `page_count * page_size` and the free pages; null when the file cannot be opened read-only.
     */
    fun dbLogical(): Triple<Long, Long, Long>? =
        try {
            SQLiteDatabase.openDatabase(posDb.path, null, SQLiteDatabase.OPEN_READONLY).use { db ->
                val pages = db.compileStatement("PRAGMA page_count").simpleQueryForLong()
                val size = db.compileStatement("PRAGMA page_size").simpleQueryForLong()
                val free = db.compileStatement("PRAGMA freelist_count").simpleQueryForLong()
                Triple(pages * size, size, free * size)
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") e: RuntimeException,
        ) {
            Log.w(TAG, "logical size unavailable: ${e.message}")
            null
        }

    /** Total size of the regular files under [dir] (0 when it does not exist) and their count. */
    fun treeBytes(dir: File): Pair<Long, Int> {
        val files = dir.walkTopDown().filter { it.isFile }.toList()
        return files.sumOf { it.length() } to files.size
    }

    companion object {
        /** The logcat tag of the result lines. */
        const val TAG = "PlatE2e"

        /** Logs one `PLATE2E_*` line. */
        fun line(text: String) {
            Log.i(TAG, text)
        }

        /** SHA-256 of [file], lowercase hex. */
        fun sha256(file: File): String {
            val digest = MessageDigest.getInstance("SHA-256")
            file.inputStream().use { input ->
                val buffer = ByteArray(BUFFER)
                var read = input.read(buffer)
                while (read >= 0) {
                    digest.update(buffer, 0, read)
                    read = input.read(buffer)
                }
            }
            return digest.digest().joinToString("") { "%02x".format(it) }
        }

        private const val DEFAULT_INSZ = "94060242576"
        private const val DEFAULT_PIN = "1111"
        private const val DEFAULT_SALES = 50
        private const val DEFAULT_SALES_MULTI = 10
        private const val DEFAULT_LINES_MULTI = 6
        private const val READY_TIMEOUT_MS = 90_000L
        private const val POLL_MS = 100L
        private const val AUDIT_ROTATIONS = 5
        private const val BUFFER = 64 * 1024
    }
}
