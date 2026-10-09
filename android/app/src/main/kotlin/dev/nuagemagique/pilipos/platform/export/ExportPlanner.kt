package dev.nuagemagique.pilipos.platform.export

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.platform.about.Hasher
import dev.nuagemagique.pilipos.platform.about.IdentityProvider
import dev.nuagemagique.pilipos.platform.about.IdentityReport
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Decides which files an export writes (PD210) and makes the fresh copy of the database they start with.
 *
 * The plan, in the order of writing: `pos.db` (a fresh `backupDatabase()`, made by the online backup API, so a
 * consistent SQLite file), the [NEWEST_BACKUPS] newest other backups in `backups/`, `backups.txt` (every other
 * backup: name, bytes, SHA-256), every file of `logs/` and `audit/` (copied up to the size they have when the
 * plan is made, so that a log that grows during the copy still hashes the same), `identity.json` and
 * `LISEZMOI.txt`. `MANIFEST.sha256` is not part of the plan: the exporter writes it last.
 *
 * Nothing is deleted: the fresh backup stays in `engine/backups`, like every backup the engine made.
 */
@Singleton
class ExportPlanner internal constructor(
    private val filesDir: File,
    private val gateway: EngineGateway,
    private val audit: PlatformAudit,
    private val identity: IdentityProvider,
    private val clock: PlatformClock,
    private val ioDispatcher: CoroutineDispatcher,
) {
    @Inject
    constructor(
        @ApplicationContext context: Context,
        gateway: EngineGateway,
        audit: PlatformAudit,
        identity: IdentityProvider,
        clock: PlatformClock,
        @IoDispatcher ioDispatcher: CoroutineDispatcher,
    ) : this(context.filesDir, gateway, audit, identity, clock, ioDispatcher)

    /** Makes the fresh database copy and lists the files; @throws IOException or an engine exception on failure. */
    suspend fun plan(): ExportPlan {
        val exportedAtMillis = clock.nowMillis()
        val fresh = File(gateway.call { it.backupDatabase() })
        val report = identity.report()
        return withContext(ioDispatcher) { build(fresh, report, exportedAtMillis) }
    }

    private fun build(
        fresh: File,
        report: IdentityReport,
        exportedAtMillis: Long,
    ): ExportPlan {
        if (!fresh.isFile) throw IOException("the fresh backup is missing: $fresh")
        val deviceId = safeName(report.deviceId)
        val others = otherBackups(fresh)
        val files = mutableListOf<PlannedFile>()
        files += diskFile("pos.db", fresh)
        others.take(NEWEST_BACKUPS).forEach { files += diskFile("backups/${it.name}", it) }
        files += bytesFile("backups.txt", inventory(others))
        File(filesDir, LOGS_DIR)
            .listFiles { f -> f.isFile }
            .orEmpty()
            .sortedBy { it.name }
            .forEach { files += diskFile("logs/${it.name}", it) }
        audit.files().forEach { files += diskFile("audit/${it.name}", it) }
        files += bytesFile("identity.json", report.toJson().toString(JSON_INDENT) + "\n")
        files += bytesFile("LISEZMOI.txt", LisezMoi.render(PlatformAudit.format(exportedAtMillis), deviceId, report.posId))
        return ExportPlan(deviceId, exportedAtMillis, DIRECTORIES, files)
    }

    /** Every `*.db` of the backup directory except [fresh], newest first. */
    private fun otherBackups(fresh: File): List<File> =
        File(filesDir, BACKUPS_DIR)
            .listFiles { f -> f.isFile && f.name.endsWith(".db") && f.name != fresh.name }
            .orEmpty()
            .sortedWith(compareByDescending<File> { it.lastModified() }.thenByDescending { it.name })

    private fun inventory(backups: List<File>): String =
        backups.sortedBy { it.name }.joinToString(separator = "") { "${it.name}\t${it.length()}\t${Hasher.sha256(it)}\n" }

    private fun diskFile(
        path: String,
        file: File,
    ) = PlannedFile(path, file.length(), ExportSource.Disk(file))

    private fun bytesFile(
        path: String,
        text: String,
    ): PlannedFile {
        val bytes = text.toByteArray(Charsets.UTF_8)
        return PlannedFile(path, bytes.size.toLong(), ExportSource.Bytes(bytes))
    }

    companion object {
        /** Number of older backups copied into `backups/`. */
        const val NEWEST_BACKUPS = 7

        /** Backup directory under `filesDir` (written by the engine). */
        const val BACKUPS_DIR = IdentityProvider.BACKUPS_DIR

        /** Engine log directory under `filesDir`. */
        const val LOGS_DIR = "logs"

        /** Directories of the export folder, created before the files. */
        val DIRECTORIES: List<String> = listOf("backups", "logs", "audit")

        private const val JSON_INDENT = 2
        private val UNSAFE = Regex("[^A-Za-z0-9._-]")

        /** [name] as a folder name part: letters, digits, dot, underscore and dash; `unknown` when null or empty. */
        fun safeName(name: String?): String = name?.replace(UNSAFE, "_")?.takeIf { it.isNotEmpty() } ?: "unknown"
    }
}
