package dev.nuagemagique.pilipos.platform.update

import dev.nuagemagique.pilipos.engine.CloseResult
import dev.nuagemagique.pilipos.platform.PlatformOps
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import dev.nuagemagique.pilipos.platform.policy.FakeDevicePolicies
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.currentTime
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class UpdateInstallerTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val log = mutableListOf<String>()
    private val engine = readyEngine()
    private val ops = PlatformOps()
    private val signal = InstallSignal()
    private val sessions = FakeSessions(log)
    private val policies = FakeDevicePolicies()
    private var restarts = 0
    private var closeAnswer: () -> CloseResult = { CloseResult.Closed }
    private lateinit var updateDir: File
    private lateinit var markers: UpdateMarkerStore
    private lateinit var audit: PlatformAudit
    private lateinit var apk: File

    /** What the world looked like at the moment of the commit. */
    private var markerAtCommit: MarkerRead? = null
    private var auditAtCommit: List<String> = emptyList()
    private var busyAtCommit: Set<String> = emptySet()

    @Before
    fun setUp() {
        updateDir = tmp.newFolder("files", "platform", "update")
        markers = UpdateMarkerStore(updateDir)
        val clock = PlatformClock { UPDATE_NOW }
        audit = PlatformAudit(File(tmp.root, "files/platform"), clock, UnconfinedTestDispatcher(), { 101L to "1.0.1" }, { 1 })
        apk = File(updateDir, "pili-pos-android-1.2.0.apk").also { it.writeBytes(ByteArray(1234) { b -> b.toByte() }) }
        coEvery { engine.getCurrentUser() } coAnswers {
            log += "preconditions"
            adminUser()
        }
        coEvery { engine.backupDatabase() } coAnswers {
            log += "backup"
            "/data/backup.db"
        }
        sessions.onCommit = {
            markerAtCommit = markers.read()
            auditAtCommit = events().map { it.first }
            busyAtCommit = ops.busy.value
            signal.complete()
        }
    }

    private fun installer(
        hold: Long = 0,
        gatewayClose: () -> CloseResult = {
            log += "close"
            closeAnswer()
        },
    ) = UpdateInstaller(
        gateway = fakeGateway(engine, gatewayClose),
        preconditions = UpdatePreconditions(fakeGateway(engine)),
        ops = ops,
        audit = audit,
        markers = markers,
        sessions = sessions,
        policies = policies,
        restarter = { restarts++ },
        signal = signal,
        versionCode = { 101L },
        clock = PlatformClock { UPDATE_NOW },
        ioDispatcher = UnconfinedTestDispatcher(),
        holdMillis = hold,
    )

    private fun events(): List<Pair<String, JSONObject>> =
        audit.files().reversed().flatMap { it.readLines() }.map { JSONObject(it) }.filter { it.getString("kind") == "update" }
            .map { it.getString("event") to it.getJSONObject("detail") }

    @Test
    fun theStepsRunInTheDocumentedOrder() =
        runTest {
            val outcome = installer().install(verifiedOf(apk))

            assertEquals(InstallOutcome.Committed, outcome)
            assertEquals(listOf("preconditions", "backup", "close", "create", "write", "commit"), log)
            // At the moment of the commit the marker is on disk and the "started" row is written.
            val found = (markerAtCommit as MarkerRead.Found).marker
            assertEquals(UpdateMarker(101L, 102L, "1.2.0", "a".repeat(40), UPDATE_NOW), found)
            assertEquals(listOf("started"), auditAtCommit)
            assertEquals(setOf("update"), busyAtCommit)
            assertEquals(1234L, sessions.lastSize)
            assertEquals(0, restarts)
            assertEquals(emptySet<String>(), ops.busy.value)
            assertTrue(markers.exists())
            assertEquals(1, sessions.closed)
        }

    @Test
    fun theStartedRowCarriesTheVersions() =
        runTest {
            installer().install(verifiedOf(apk))
            val started = events().single { it.first == "started" }.second
            assertEquals("101", started.getString("from"))
            assertEquals("102", started.getString("to"))
            assertEquals("1.2.0", started.getString("version"))
            assertEquals("a".repeat(40), started.getString("sha1"))
        }

    @Test
    fun aPreconditionFailureTouchesNothing() =
        runTest {
            coEvery { engine.needsZReport() } returns true
            val outcome = installer().install(verifiedOf(apk))

            assertEquals(InstallOutcome.Blocked(listOf(PreconditionFailure.Z_DUE)), outcome)
            coVerify(exactly = 0) { engine.backupDatabase() }
            assertFalse("close" in log)
            assertFalse("create" in log)
            assertFalse(markers.exists())
            assertEquals(0, restarts)
            assertEquals("z_due", events().single { it.first == "install_blocked" }.second.getString("failures"))
        }

    @Test
    fun aFailedBackupRefusesBeforeClosingTheEngine() =
        runTest {
            coEvery { engine.backupDatabase() } throws IOException("disk full")
            val outcome = installer().install(verifiedOf(apk))

            assertEquals(InstallOutcome.Refused(InstallRefusal.BACKUP_FAILED, "disk full"), outcome)
            assertFalse("close" in log)
            assertFalse(markers.exists())
            assertEquals(0, restarts)
            assertEquals("backup_failed", events().single { it.first == "install_refused" }.second.getString("code"))
        }

    @Test
    fun anEngineThatStaysOpenRefusesAndIsRestarted() =
        runTest {
            closeAnswer = { CloseResult.StillOpen("a call is in flight") }
            val outcome = installer().install(verifiedOf(apk))

            assertEquals(InstallOutcome.Refused(InstallRefusal.ENGINE_BUSY, "a call is in flight"), outcome)
            assertEquals(listOf("preconditions", "backup", "close"), log)
            assertFalse(markers.exists())
            assertEquals(1, restarts)
            assertEquals("engine_busy", events().single { it.first == "install_refused" }.second.getString("code"))
        }

    @Test
    fun aWriteErrorAbandonsTheSessionAndDeletesTheMarker() =
        runTest {
            sessions.onWrite = { throw IOException("no space left") }
            val outcome = installer().install(verifiedOf(apk))

            assertTrue(outcome is InstallOutcome.Refused && outcome.reason == InstallRefusal.INSTALL_FAILED)
            assertEquals(listOf("preconditions", "backup", "close", "create", "write", "abandon"), log)
            assertEquals(listOf(7), sessions.abandoned)
            assertEquals(0, sessions.committed)
            assertFalse(markers.exists())
            assertEquals(1, restarts)
            assertEquals(listOf("started", "failed"), events().map { it.first })
            assertTrue(events().last().second.getString("detail").contains("no space left"))
        }

    @Test
    fun aCommitErrorAbandonsTheSessionAndDeletesTheMarker() =
        runTest {
            sessions.onCommit = { throw IllegalStateException("session is sealed") }
            val outcome = installer().install(verifiedOf(apk))

            assertTrue(outcome is InstallOutcome.Refused && outcome.reason == InstallRefusal.INSTALL_FAILED)
            assertEquals(listOf(7), sessions.abandoned)
            assertFalse(markers.exists())
            assertEquals(1, restarts)
            assertEquals("failed", events().last().first)
        }

    @Test
    fun aSessionThatCannotBeCreatedReopensTheEngine() =
        runTest {
            sessions.failCreate = IOException("no installer")
            val outcome = installer().install(verifiedOf(apk))

            assertTrue(outcome is InstallOutcome.Refused && outcome.reason == InstallRefusal.INSTALL_FAILED)
            assertFalse(markers.exists())
            assertEquals(1, restarts)
        }

    @Test
    fun aMarkerThatCannotBeWrittenReopensTheEngine() =
        runTest {
            val blocked = File(tmp.root, "blocked").also { it.writeText("a file, not a directory") }
            val installer =
                UpdateInstaller(
                    fakeGateway(engine) { CloseResult.Closed },
                    UpdatePreconditions(fakeGateway(engine)),
                    ops,
                    audit,
                    UpdateMarkerStore(File(blocked, "update")),
                    sessions,
                    policies,
                    { restarts++ },
                    signal,
                    { 101L },
                    PlatformClock { UPDATE_NOW },
                    UnconfinedTestDispatcher(),
                    0,
                )
            val outcome = installer.install(verifiedOf(apk))

            assertTrue(outcome is InstallOutcome.Refused && outcome.reason == InstallRefusal.INSTALL_FAILED)
            assertFalse("create" in log)
            assertEquals(1, restarts)
        }

    @Test
    fun theOperationIsHeldUntilTheSystemAnswersOrTheHoldEnds() =
        runTest {
            sessions.onCommit = { busyAtCommit = ops.busy.value }
            val before = currentTime
            installer(hold = 5_000).install(verifiedOf(apk))
            assertEquals(5_000L, currentTime - before)
            assertEquals(emptySet<String>(), ops.busy.value)
        }

    @Test
    fun aStatusThatArrivesEndsTheHoldAtOnce() =
        runTest {
            val before = currentTime
            installer(hold = 5_000).install(verifiedOf(apk))
            assertEquals(0L, currentTime - before)
        }

    @Test
    fun theMarkerFileIsPlainJsonOfTheDocumentedShape() =
        runTest {
            installer().install(verifiedOf(apk))
            val json = JSONObject(File(updateDir, "pending.json").readText())
            assertEquals(setOf("fromVersionCode", "toVersionCode", "version", "sha1", "startedAt"), json.keys().asSequence().toSet())
            assertNotNull(json.getLong("startedAt"))
        }

    private val clear = "clearUserRestriction(no_install_apps)"
    private val set = "addUserRestriction(no_install_apps)"

    @Test
    fun installAppsIsClearedForTheSessionAndSetAgainAfterTheAnswer() =
        runTest {
            var atWrite = emptyList<String>()
            var atCommit = emptyList<String>()
            sessions.onWrite = { atWrite = policies.calls.toList() }
            sessions.onCommit = {
                atCommit = policies.calls.toList()
                signal.complete()
            }
            installer().install(verifiedOf(apk))

            assertEquals(listOf(clear), atWrite)
            assertEquals(listOf(clear), atCommit)
            assertEquals(listOf(clear, set), policies.calls)
        }

    @Test
    fun installAppsIsNotTouchedWhenAPreconditionBlocksTheInstall() =
        runTest {
            coEvery { engine.needsZReport() } returns true
            installer().install(verifiedOf(apk))
            assertEquals(emptyList<String>(), policies.calls)
        }

    @Test
    fun installAppsIsSetAgainWhenTheSessionFails() =
        runTest {
            sessions.onWrite = { throw IOException("no space left") }
            installer().install(verifiedOf(apk))
            assertEquals(listOf(clear, set), policies.calls)

            policies.calls.clear()
            sessions.onWrite = {}
            sessions.onCommit = { throw IllegalStateException("session is sealed") }
            installer().install(verifiedOf(apk))
            assertEquals(listOf(clear, set), policies.calls)
        }

    @Test
    fun installAppsStaysClearedUntilTheHoldEnds() =
        runTest {
            sessions.onCommit = {}
            var restoredAt = -1L
            val hold = 5_000L
            val before = currentTime
            installer(hold = hold).install(verifiedOf(apk))
            restoredAt = currentTime - before
            assertEquals(hold, restoredAt)
            assertEquals(listOf(clear, set), policies.calls)
        }

    @Test
    fun aRefusedClearDoesNotStopTheInstallAndTheRestrictionIsStillSetAgain() =
        runTest {
            policies.failing += "clearUserRestriction"
            val outcome = installer().install(verifiedOf(apk))
            assertEquals(InstallOutcome.Committed, outcome)
            assertEquals(listOf(clear, set), policies.calls)
        }

    @Test
    fun aRefusedSetAgainDoesNotChangeTheOutcome() =
        runTest {
            policies.failing += "addUserRestriction"
            assertEquals(InstallOutcome.Committed, installer().install(verifiedOf(apk)))
        }

    @Test
    fun notBeingTheOwnerLeavesTheRestrictionAlone() =
        runTest {
            policies.owner = false
            assertEquals(InstallOutcome.Committed, installer().install(verifiedOf(apk)))
            assertEquals(emptyList<String>(), policies.calls)
        }
}
