package dev.nuagemagique.pilipos.platform.export

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@RunWith(RobolectricTestRunner::class)
class ExportPlannerTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @Test
    fun theSevenNewestOtherBackupsAreCopiedAndAllAreListed() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder(), backupCount = 12).open()
            val plan = fx.planner.plan()
            val copied = plan.files.map { it.path }.filter { it.startsWith("backups/") }
            assertEquals((5..11).map { "backups/pos-backup-%02d.db".format(it) }.reversed(), copied)
            val inventory = String(((plan.files.single { it.path == "backups.txt" }.source) as ExportSource.Bytes).bytes)
            assertEquals(12, inventory.lines().filter { it.isNotEmpty() }.size)
            val names = inventory.lines().filter { it.isNotEmpty() }.map { it.substringBefore('\t') }
            assertEquals((0..11).map { "pos-backup-%02d.db".format(it) }, names)
        }

    @Test
    fun theOrderIsDatabaseBackupsInventoryLogsAuditIdentityReadMe() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.audit.append("export", "seed")
            val paths = fx.planner.plan().files.map { it.path }
            assertEquals("pos.db", paths.first())
            assertEquals(listOf("identity.json", "LISEZMOI.txt"), paths.takeLast(2))
            val order = listOf("pos.db", "backups/", "backups.txt", "logs/", "audit/", "identity.json", "LISEZMOI.txt")
            val positions = order.map { prefix -> paths.indexOfFirst { it.startsWith(prefix) } }
            assertEquals(positions.sorted(), positions)
            assertFalse(paths.contains("MANIFEST.sha256"))
        }

    @Test
    fun theFreshBackupIsPosDbAndIsNotInTheOlderOnes() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val plan = fx.planner.plan()
            assertEquals(fx.fresh.length(), plan.files.first().length)
            assertFalse(plan.files.any { it.path.contains("fresh") })
            assertFalse(plan.files.any { it.path.contains("not-a-backup") })
        }

    @Test
    fun lengthsAreASnapshotTakenAtPlanTime() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            val log = File(fx.logsDir, "pos.log.2026-10-09")
            val before = log.length()
            val plan = fx.planner.plan()
            log.appendText("more")
            assertEquals(before, plan.files.single { it.path == "logs/pos.log.2026-10-09" }.length)
            assertEquals(plan.files.sumOf { it.length }, plan.totalBytes)
        }

    @Test
    fun theAuditFilesAreIncluded() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.audit.append("export", "seed")
            val audit = fx.planner.plan().files.filter { it.path.startsWith("audit/") }
            assertEquals(listOf("audit/audit.jsonl"), audit.map { it.path })
            assertTrue(audit.single().length > 0)
        }

    @Test
    fun theDirectoriesAreListedEvenWhenEmpty() =
        runTest {
            val fx = ExportFixture(this, tmp.newFolder(), backupCount = 0).open()
            assertEquals(listOf("backups", "logs", "audit"), fx.planner.plan().directories)
        }

    @Test
    fun theDeviceIdBecomesASafeFolderNamePart() {
        assertEquals("SN-0001", ExportPlanner.safeName("SN-0001"))
        assertEquals("a_b_c", ExportPlanner.safeName("a/b c"))
        assertEquals("unknown", ExportPlanner.safeName(null))
        assertEquals("unknown", ExportPlanner.safeName(""))
    }
}
