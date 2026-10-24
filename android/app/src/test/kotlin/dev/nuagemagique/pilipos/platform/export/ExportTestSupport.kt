package dev.nuagemagique.pilipos.platform.export

import android.content.Intent
import android.database.sqlite.SQLiteDatabase
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.LaunchResult
import dev.nuagemagique.pilipos.platform.PlatformOps
import dev.nuagemagique.pilipos.platform.about.IdentityProvider
import dev.nuagemagique.pilipos.platform.about.sampleReport
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import io.mockk.coEvery
import io.mockk.mockk
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import org.json.JSONObject
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.TakeawayInfo
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import java.io.File
import java.io.IOException
import java.io.InputStream
import java.security.MessageDigest
import java.time.Instant

/** 2026-10-09 10:15:30 in Brussels (CEST). */
val EXPORT_NOW: Long = Instant.parse("2026-10-09T08:15:30Z").toEpochMilli()

const val EXPORT_FOLDER = "pili-export-20261009-101530-SN-0001"

fun sha256Of(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }

fun sha256Of(file: File): String = sha256Of(file.readBytes())

/** A volume probe with a settable list. */
class FakeVolumes(
    var volumes: List<RemovableVolume> = emptyList(),
    private val intent: Intent? = null,
) : VolumeProbe {
    override fun mounted(): List<RemovableVolume> = volumes

    override fun pickerIntent(uuid: String): Intent? = intent.takeIf { volumes.any { it.uuid == uuid } }
}

/**
 * The whole export stack over temp directories: a real engine gateway over a mocked engine, a real planner,
 * exporter and audit. [filesDir] holds `engine/backups` ([backupCount] files, the n-th modified n seconds after
 * the first), `logs` (two files) and the platform audit.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LongParameterList")
class ExportFixture(
    scope: TestScope,
    val root: File,
    private val role: UserRole = UserRole.ADMIN,
    private val openOrders: Int = 0,
    private val openTakeaways: Int = 0,
    private val backupCount: Int = 10,
    private val ready: Boolean = true,
) {
    val dispatcher: CoroutineDispatcher = UnconfinedTestDispatcher(scope.testScheduler)
    val engine = EngineFixture(scope, root, if (ready) null else LaunchResult.NotProvisioned, dispatcher)
    val gateway = engine.gateway(scope)
    val filesDir = File(root, "files").apply { mkdirs() }
    val backupsDir = File(filesDir, "engine/backups").apply { mkdirs() }
    val logsDir = File(filesDir, "logs").apply { mkdirs() }
    val fresh = File(backupsDir, "pos-backup-fresh.db")
    val clock = PlatformClock { EXPORT_NOW }
    val audit = PlatformAudit(File(filesDir, "platform"), clock, dispatcher, { 1L to "1.0" }, { 1 })
    val identity: IdentityProvider = mockk()
    val volumes = FakeVolumes()
    val ops = PlatformOps()
    val planner = ExportPlanner(filesDir, gateway, audit, identity, clock, dispatcher)
    val exporter = Exporter(gateway, planner, volumes, audit, ops, dispatcher)
    var backupAnswer: () -> String = { fresh.path }

    suspend fun open(): ExportFixture {
        if (ready) engine.holder.open()
        repeat(backupCount) { i ->
            File(backupsDir, "pos-backup-%02d.db".format(i)).also {
                it.writeBytes(ByteArray(100 + i) { b -> (b * (i + 3)).toByte() })
                it.setLastModified(1_700_000_000_000L + i * 1000L)
            }
        }
        File(backupsDir, "not-a-backup.txt").writeText("ignored")
        File(logsDir, "pos.log.2026-10-08").writeText("log of the 8th\n")
        File(logsDir, "pos.log.2026-10-09").writeText("log of the 9th\n")
        makeFresh()
        coEvery { identity.report() } returns sampleReport()
        coEvery { engine.engine.backupDatabase() } coAnswers { backupAnswer() }
        coEvery { engine.engine.getCurrentUser() } returns User("1", "Admin", role, true, true)
        coEvery { engine.engine.listOpenOrders() } returns List(openOrders) { mockk<Order>() }
        coEvery { engine.engine.listOpenTakeaways() } returns List(openTakeaways) { mockk<TakeawayInfo>() }
        return this
    }

    /** A real SQLite file with 3 rows, the newest file of the backup directory. */
    fun makeFresh() {
        fresh.delete()
        val db = SQLiteDatabase.openOrCreateDatabase(fresh, null)
        db.execSQL("CREATE TABLE fiscal_event (id INTEGER PRIMARY KEY, kind TEXT)")
        for (i in 1..3) db.execSQL("INSERT INTO fiscal_event (kind) VALUES ('N$i')")
        db.close()
        fresh.setLastModified(1_800_000_000_000L)
    }

    fun auditEvents(): List<Pair<String, JSONObject>> =
        audit.files().flatMap { it.readLines() }.map { JSONObject(it) }.filter { it.getString("kind") == "export" }
            .map { it.getString("event") to it.getJSONObject("detail") }
}

/** Wraps a target to observe it and to inject faults; nothing here deletes anything. */
class ProbeTarget(
    private val inner: ExportTarget,
    private val failWriteOf: String? = null,
    private val corruptReadOf: String? = null,
    private val beforeCreate: (String) -> Unit = {},
) : ExportTarget {
    val created = mutableListOf<String>()
    private val names = HashMap<ExportFile, String>()

    override val root: ExportDir get() = inner.root
    override val volumeUuid: String? get() = inner.volumeUuid

    override fun createDirectory(
        parent: ExportDir,
        name: String,
    ): ExportDir = inner.createDirectory(parent, name)

    override fun createFile(
        dir: ExportDir,
        name: String,
        mime: String,
    ): ExportOutput {
        beforeCreate(name)
        created += name
        val out = inner.createFile(dir, name, mime)
        names[out.file] = name
        if (name != failWriteOf) return out
        return object : ExportOutput() {
            override val file: ExportFile get() = out.file

            override fun sync() = out.sync()

            override fun write(b: Int) = throw IOException("No space left on device")

            override fun write(
                b: ByteArray,
                off: Int,
                len: Int,
            ) = throw IOException("No space left on device")

            override fun close() = out.close()
        }
    }

    override fun openForRead(file: ExportFile): InputStream {
        val stream = inner.openForRead(file)
        if (names[file] != corruptReadOf) return stream
        val bytes = stream.use { it.readBytes() }
        if (bytes.isNotEmpty()) bytes[0] = (bytes[0] + 1).toByte()
        return bytes.inputStream()
    }
}
