package dev.nuagemagique.pilipos.platform.export

import android.database.sqlite.SQLiteDatabase
import io.mockk.coVerify
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.UserRole
import java.io.File
import java.io.RandomAccessFile

@RunWith(RobolectricTestRunner::class)
class ExporterTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val outDir get() = File(tmp.root, "stick").apply { mkdirs() }

    private fun target(
        failWriteOf: String? = null,
        corruptReadOf: String? = null,
        beforeCreate: (String) -> Unit = {},
        uuid: String? = null,
    ) = ProbeTarget(FileTarget(outDir, uuid), failWriteOf, corruptReadOf, beforeCreate)

    private fun exported() = File(outDir, EXPORT_FOLDER)

    private fun tree(dir: File): List<String> = dir.walkTopDown().filter { it.isFile }.map { it.relativeTo(dir).path }.sorted().toList()

    @Test
    fun anExportWritesTheFolderWithEveryEntryAndTheManifestLast() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val probe = target()
            val outcome = fx.exporter.run(probe, "PILI-EXPORT")
            val done = (outcome as ExportOutcome.Done).result
            val dir = exported()
            assertTrue(dir.isDirectory)
            assertEquals(EXPORT_FOLDER, done.exportFolder)
            assertEquals("PILI-EXPORT", done.chosenFolder)

            val names = tree(dir)
            // 10 older backups: only the 7 newest are copied (09 down to 03).
            val backups = names.filter { it.startsWith("backups/") }
            assertEquals(7, backups.size)
            assertEquals((3..9).map { "backups/pos-backup-%02d.db".format(it) }, backups)
            assertTrue("pos.db" in names)
            assertTrue("backups.txt" in names)
            assertEquals(listOf("logs/pos.log.2026-10-08", "logs/pos.log.2026-10-09"), names.filter { it.startsWith("logs/") })
            assertTrue(names.any { it == "audit/audit.jsonl" })
            assertTrue("identity.json" in names)
            assertTrue("LISEZMOI.txt" in names)
            assertTrue("MANIFEST.sha256" in names)
            assertEquals(done.files, names.size)
            assertEquals(dir.walkTopDown().filter { it.isFile }.sumOf { it.length() }, done.bytes)

            // The manifest is the last file created.
            assertEquals("MANIFEST.sha256", probe.created.last())
            assertEquals(names.size, probe.created.size)

            // shasum -a 256 -c semantics: every line matches the file, every other file is listed.
            val lines = File(dir, "MANIFEST.sha256").readLines()
            assertEquals(names.size - 1, lines.size)
            assertEquals(lines.map { it.substringAfter("  ") }.sorted(), lines.map { it.substringAfter("  ") })
            lines.forEach { line ->
                val (sha, path) = line.split("  ", limit = 2)
                assertEquals("hash of $path", sha, sha256Of(File(dir, path)))
            }
            assertFalse(lines.any { it.endsWith("MANIFEST.sha256") })
            assertEquals(sha256Of(File(dir, "MANIFEST.sha256")), done.manifestSha256)
        }

    @Test
    fun posDbIsAConsistentCopyOfTheFreshBackup() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.exporter.run(target(), "PILI-EXPORT")
            val db = SQLiteDatabase.openDatabase(File(exported(), "pos.db").path, null, SQLiteDatabase.OPEN_READONLY)
            db.use {
                assertEquals(
                    "ok",
                    it.rawQuery("PRAGMA integrity_check", null).use { c ->
                        c.moveToFirst()
                        c.getString(0)
                    },
                )
                assertEquals(
                    3,
                    it.rawQuery("SELECT COUNT(*) FROM fiscal_event", null).use { c ->
                        c.moveToFirst()
                        c.getInt(0)
                    },
                )
            }
            assertEquals(sha256Of(fx.fresh), sha256Of(File(exported(), "pos.db")))
        }

    @Test
    fun theFilesCarryTheInventoryTheIdentityAndTheReadMe() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.exporter.run(target(), "PILI-EXPORT")
            val inventory = File(exported(), "backups.txt").readLines()
            assertEquals(10, inventory.size)
            inventory.forEach { line ->
                val (name, bytes, sha) = line.split("\t")
                val file = File(fx.backupsDir, name)
                assertEquals(file.length(), bytes.toLong())
                assertEquals(sha256Of(file), sha)
            }
            assertFalse(inventory.any { it.startsWith("pos-backup-fresh") })
            val identity = JSONObject(File(exported(), "identity.json").readText())
            assertEquals("SN-0001", identity.getString("deviceId"))
            val readMe = File(exported(), "LISEZMOI.txt").readText()
            assertTrue(readMe.contains("shasum -a 256 -c MANIFEST.sha256"))
            assertTrue(readMe.contains("SN-0001"))
            assertTrue(readMe.contains("POS-01"))
            assertTrue(readMe.contains("2026-10-09T10:15:30"))
        }

    @Test
    fun progressEndsFull() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.exporter.run(target(), "PILI-EXPORT")
            val p = fx.exporter.progress.value
            assertEquals(p.filesTotal, p.filesDone)
            assertEquals(p.bytesTotal, p.bytesDone)
            assertTrue(p.bytesTotal > 0)
        }

    @Test
    fun theOperationIsBusyWhileItRunsAndAuditedStartedThenDone() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            var busy = emptySet<String>()
            fx.exporter.run(target(beforeCreate = { busy = fx.ops.busy.value }), "PILI-EXPORT")
            assertTrue("export" in busy)
            assertTrue(fx.ops.busy.value.isEmpty())
            val events = fx.auditEvents()
            assertEquals(listOf("started", "done"), events.map { it.first })
            val detail = events.last().second
            assertEquals("PILI-EXPORT/$EXPORT_FOLDER", detail.getString("folder"))
            assertEquals(tree(exported()).size, detail.getString("files").toInt())
        }

    @Test
    fun theStartedRowIsPartOfTheExportedAuditFile() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.exporter.run(target(), "PILI-EXPORT")
            val copied = File(exported(), "audit/audit.jsonl").readLines().map { JSONObject(it).getString("event") }
            assertEquals(listOf("started"), copied)
        }

    @Test
    fun aLogThatGrowsDuringTheCopyIsCopiedUpToItsPlannedSize() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val log = File(fx.logsDir, "pos.log.2026-10-09")
            val planned = log.length()
            val probe = target(beforeCreate = { if (it == "pos.db") log.appendText("grown after the plan\n") })
            val outcome = fx.exporter.run(probe, "PILI-EXPORT")
            assertTrue(outcome is ExportOutcome.Done)
            assertTrue(log.length() > planned)
            assertEquals(planned, File(exported(), "logs/pos.log.2026-10-09").length())
        }

    @Test
    fun aFileLargerThanOneChunkIsCopiedWholeAndHashedRight() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val big = File(fx.logsDir, "pos.log.big")
            big.writeBytes(ByteArray(ExportCopier.CHUNK_BYTES * 2 + 12_345) { (it * 31 % 251).toByte() })
            val outcome = fx.exporter.run(target(), "PILI-EXPORT")
            assertTrue(outcome is ExportOutcome.Done)
            val copy = File(exported(), "logs/pos.log.big")
            assertEquals(big.length(), copy.length())
            assertEquals(sha256Of(big), sha256Of(copy))
            val manifest = File(exported(), "MANIFEST.sha256").readLines()
            assertTrue(manifest.contains("${sha256Of(big)}  logs/pos.log.big"))
        }

    @Test
    fun aReadBackMismatchFailsTheExportAndKeepsWhatWasWritten() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val before = fx.backupsDir.listFiles().orEmpty().map { it.name }.sorted()
            val outcome = fx.exporter.run(target(corruptReadOf = "backups.txt"), "PILI-EXPORT")
            val failure = (outcome as ExportOutcome.Failed).failure
            assertEquals(ExportStep.VERIFY, failure.step)
            assertEquals(ExportProblem.READ_BACK_MISMATCH, failure.problem)
            assertEquals("backups.txt", failure.file)
            val names = tree(exported())
            assertTrue("pos.db" in names)
            assertTrue("backups.txt" in names)
            assertFalse("MANIFEST.sha256" in names)
            assertFalse("identity.json" in names)
            val (event, detail) = fx.auditEvents().last()
            assertEquals("failed", event)
            assertEquals("verify", detail.getString("step"))
            assertEquals("backups.txt", detail.getString("file"))
            // Nothing in the till was deleted (the fresh backup stays next to the older ones).
            assertEquals(before, fx.backupsDir.listFiles().orEmpty().map { it.name }.sorted())
        }

    @Test
    fun aWriteErrorFailsTheExportAndKeepsWhatWasWritten() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val outcome = fx.exporter.run(target(failWriteOf = "identity.json"), "PILI-EXPORT")
            val failure = (outcome as ExportOutcome.Failed).failure
            assertEquals(ExportStep.COPY, failure.step)
            assertEquals(ExportProblem.WRITE_FAILED, failure.problem)
            assertEquals("identity.json", failure.file)
            assertTrue(failure.detail.orEmpty().contains("No space left"))
            val names = tree(exported())
            assertTrue("pos.db" in names)
            assertTrue(names.any { it.startsWith("logs/") })
            assertFalse("MANIFEST.sha256" in names)
            val (event, detail) = fx.auditEvents().last()
            assertEquals("failed", event)
            assertEquals("copy", detail.getString("step"))
            assertEquals("identity.json", detail.getString("file"))
        }

    @Test
    fun aWriteErrorOnAFileOfFourGibOrMoreHintsAtFat32() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            RandomAccessFile(File(fx.logsDir, "zz-big.log"), "rw").use { it.setLength(4L * 1024 * 1024 * 1024) }
            val outcome = fx.exporter.run(target(failWriteOf = "zz-big.log"), "PILI-EXPORT")
            val failure = (outcome as ExportOutcome.Failed).failure
            assertEquals(ExportProblem.FAT32, failure.problem)
            assertEquals("logs/zz-big.log", failure.file)
        }

    @Test
    fun aMissingSourceIsAReadFailure() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val vanishing = File(fx.logsDir, "pos.log.2026-10-08")
            val probe = target(beforeCreate = { if (it == "pos.log.2026-10-08") vanishing.delete() })
            val failure = (fx.exporter.run(probe, "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportProblem.READ_FAILED, failure.problem)
            assertEquals("logs/pos.log.2026-10-08", failure.file)
        }

    @Test
    fun aFailedFreshBackupIsAPlanFailure() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.backupAnswer = { throw IllegalStateException("disk image is malformed") }
            val failure = (fx.exporter.run(target(), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportStep.PLAN, failure.step)
            assertEquals(ExportProblem.PLAN_FAILED, failure.problem)
            assertTrue(failure.detail.orEmpty().contains("malformed"))
            assertFalse(exported().exists())
            assertEquals("failed", fx.auditEvents().last().first)
        }

    @Test
    fun aFreshBackupThatIsMissingOnDiskIsAPlanFailure() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.backupAnswer = { File(fx.backupsDir, "nowhere.db").path }
            val failure = (fx.exporter.run(target(), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportProblem.PLAN_FAILED, failure.problem)
        }

    @Test
    fun aDirectoryThatCannotBeCreatedIsAWriteFailure() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            File(outDir, EXPORT_FOLDER).mkdirs()
            val failure = (fx.exporter.run(target(), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportStep.CREATE_DIR, failure.step)
            assertEquals(ExportProblem.WRITE_FAILED, failure.problem)
        }

    @Test
    fun aNonManagerIsRefusedBeforeAnythingIsDone() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder(), role = UserRole.CASHIER).open()
            val failure = (fx.exporter.run(target(), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportStep.PRECHECK, failure.step)
            assertEquals(ExportProblem.NOT_MANAGER, failure.problem)
            assertFalse(exported().exists())
            assertEquals(listOf("refused"), fx.auditEvents().map { it.first })
            coVerify(exactly = 0) { fx.engine.engine.backupDatabase() }
        }

    @Test
    fun anOpenOrderRefusesTheExport() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder(), openOrders = 1).open()
            val failure = (fx.exporter.run(target(), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportProblem.ORDERS_OPEN, failure.problem)
            assertFalse(exported().exists())
            coVerify(exactly = 0) { fx.engine.engine.backupDatabase() }
        }

    @Test
    fun anOpenTakeawayRefusesTheExport() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder(), openTakeaways = 1).open()
            val failure = (fx.exporter.run(target(), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportProblem.ORDERS_OPEN, failure.problem)
        }

    @Test
    fun anEngineThatIsNotReadyRefusesTheExport() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder(), ready = false).open()
            val failure = (fx.exporter.run(target(), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportProblem.ENGINE_NOT_READY, failure.problem)
        }

    @Test
    fun aVolumeThatLeftRefusesTheExport() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val failure = (fx.exporter.run(target(uuid = "ABCD-1234"), "PILI-EXPORT") as ExportOutcome.Failed).failure
            assertEquals(ExportProblem.NO_VOLUME, failure.problem)
            fx.volumes.volumes = listOf(RemovableVolume("abcd-1234", "Stick"))
            assertTrue(fx.exporter.run(target(uuid = "ABCD-1234"), "PILI-EXPORT") is ExportOutcome.Done)
        }

    @Test
    fun accessReportsTheProblemOrNull() =
        runTest {
            assertNull(ExportFixture(this, tmp.newFolder()).open().exporter.access())
            assertNotNull(ExportFixture(this, tmp.newFolder(), role = UserRole.WAITER).open().exporter.access())
        }

    @Test
    fun theManifestOfAnEmptyEnvironmentStillListsEveryFile() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder(), backupCount = 0).open()
            fx.logsDir.listFiles().orEmpty().forEach { it.delete() }
            val outcome = fx.exporter.run(target(), "PILI-EXPORT")
            assertTrue(outcome is ExportOutcome.Done)
            assertEquals(emptyList<String>(), tree(exported()).filter { it.startsWith("backups/") || it.startsWith("logs/") })
            assertEquals("", File(exported(), "backups.txt").readText())
            assertTrue(File(exported(), "backups").isDirectory)
            assertTrue(File(exported(), "logs").isDirectory)
        }
}
