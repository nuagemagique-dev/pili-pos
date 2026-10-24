package dev.nuagemagique.pilipos.platform.update

import android.app.Application
import android.content.Intent
import android.content.pm.PackageInstaller
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.currentTime
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class UpdateReceiversTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val app = ApplicationProvider.getApplicationContext<Application>()
    private val log = mutableListOf<String>()
    private val sessions = FakeSessions(log)
    private val signal = InstallSignal()
    private var restarts = 0
    private lateinit var markers: UpdateMarkerStore
    private lateinit var audit: PlatformAudit
    private val platformDir get() = File(tmp.root, "platform")

    @Before
    fun setUp() {
        audit = PlatformAudit(platformDir, { 1_000L }, UnconfinedTestDispatcher(), { 1L to "t" }, { 1 })
        markers = UpdateMarkerStore(File(platformDir, "update"))
        markers.write(UpdateMarker(101, 102, "1.2.0", "a".repeat(40), 5L))
    }

    private fun handler() = InstallStatusHandler(audit, sessions, { restarts++ }, signal, markers)

    private fun entryPoint(): UpdateEntryPoint {
        val handler = handler()
        val scope = CoroutineScope(Dispatchers.Unconfined)
        return object : UpdateEntryPoint {
            override fun platformAudit() = audit

            override fun statusHandler() = handler

            override fun applicationScope() = scope
        }
    }

    private fun events(): List<Pair<String, JSONObject>> {
        val file = File(platformDir, "audit.jsonl")
        if (!file.exists()) return emptyList()
        return file.readLines().map { JSONObject(it) }.map {
            it.getString("kind") + "/" + it.getString("event") to it.getJSONObject("detail")
        }
    }

    private fun status(
        code: Int,
        session: Int = 9,
        message: String? = null,
    ) = Intent(InstallResultReceiver.ACTION_INSTALL_STATUS)
        .putExtra(PackageInstaller.EXTRA_STATUS, code)
        .putExtra(PackageInstaller.EXTRA_SESSION_ID, session)
        .apply { message?.let { putExtra(PackageInstaller.EXTRA_STATUS_MESSAGE, it) } }

    private fun deliver(intent: Intent) = InstallResultReceiver().apply { resolver = { entryPoint() } }.onReceive(app, intent)

    @Test
    fun pendingUserActionAbandonsTheSessionAndAuditsASilentInstallRefusal() {
        deliver(status(PackageInstaller.STATUS_PENDING_USER_ACTION, session = 9))

        assertEquals(listOf(9), sessions.abandoned)
        val (name, detail) = events().single()
        assertEquals("update/failed", name)
        assertEquals("silent_install_refused", detail.getString("code"))
        assertEquals(PackageInstaller.STATUS_PENDING_USER_ACTION.toString(), detail.getString("status"))
        assertFalse(markers.exists())
        assertEquals(1, restarts)
    }

    @Test
    fun aFailureStatusAuditsItsMessageDeletesTheMarkerAndRestartsTheEngine() {
        deliver(status(PackageInstaller.STATUS_FAILURE_INCOMPATIBLE, message = "INSTALL_FAILED_UPDATE_INCOMPATIBLE"))

        val (name, detail) = events().single()
        assertEquals("update/failed", name)
        assertEquals("install_failed", detail.getString("code"))
        assertEquals("INSTALL_FAILED_UPDATE_INCOMPATIBLE", detail.getString("message"))
        assertEquals(PackageInstaller.STATUS_FAILURE_INCOMPATIBLE.toString(), detail.getString("status"))
        assertTrue(sessions.abandoned.isEmpty())
        assertFalse(markers.exists())
        assertEquals(1, restarts)
    }

    @Test
    fun everyFailureStatusIsAFailure() {
        listOf(
            PackageInstaller.STATUS_FAILURE,
            PackageInstaller.STATUS_FAILURE_ABORTED,
            PackageInstaller.STATUS_FAILURE_BLOCKED,
            PackageInstaller.STATUS_FAILURE_CONFLICT,
            PackageInstaller.STATUS_FAILURE_INVALID,
            PackageInstaller.STATUS_FAILURE_STORAGE,
        ).forEachIndexed { index, code ->
            markers.write(UpdateMarker(101, 102, "1.2.0", "a", 5L))
            deliver(status(code))
            assertFalse("status $code", markers.exists())
            assertEquals(index + 1, restarts)
        }
        assertEquals(6, events().count { it.first == "update/failed" })
    }

    @Test
    fun aSuccessIsAuditedAndKeepsTheMarkerForTheNewProcess() {
        deliver(status(PackageInstaller.STATUS_SUCCESS))

        assertEquals(listOf("update/status_success"), events().map { it.first })
        assertTrue(markers.exists())
        assertEquals(0, restarts)
    }

    @Test
    fun theStatusCompletesTheSignal() =
        runTest {
            signal.begin()
            handler().handle(PackageInstaller.STATUS_FAILURE, 1, null)
            val before = currentTime
            signal.await(60_000)
            assertEquals("the signal was already complete", 0L, currentTime - before)
        }

    @Test
    fun anotherActionIsIgnoredByTheStatusReceiver() {
        deliver(Intent(Intent.ACTION_BOOT_COMPLETED))
        deliver(Intent("com.example.OTHER"))
        deliver(Intent(Intent.ACTION_MY_PACKAGE_REPLACED))

        assertTrue(events().isEmpty())
        assertTrue(markers.exists())
        assertEquals(0, restarts)
    }

    @Test
    fun withoutAHiltGraphTheStatusReceiverDoesNotThrow() {
        InstallResultReceiver().onReceive(app, status(PackageInstaller.STATUS_FAILURE))
    }

    @Test
    fun aFailingLookupIsSwallowed() {
        InstallResultReceiver().apply { resolver = { error("no graph") } }.onReceive(app, status(PackageInstaller.STATUS_FAILURE))
    }

    @Test
    fun aHandlerThatFailsIsSwallowed() {
        val broken =
            object : UpdateEntryPoint {
                override fun platformAudit() = audit

                override fun statusHandler(): InstallStatusHandler = error("broken")

                override fun applicationScope() = CoroutineScope(Dispatchers.Unconfined)
            }
        InstallResultReceiver().apply { resolver = { broken } }.onReceive(app, status(PackageInstaller.STATUS_FAILURE))
        assertTrue(events().isEmpty())
    }

    @Test
    fun packageReplacedStartsTheMainActivityAndAudits() {
        PackageReplacedReceiver().apply { resolver = { entryPoint() } }.onReceive(app, Intent(Intent.ACTION_MY_PACKAGE_REPLACED))

        assertEquals(listOf("update/package_replaced"), events().map { it.first })
        val started = shadowOf(app).nextStartedActivity
        assertNotNull(started)
        assertEquals(MainActivity::class.java.name, started.component?.className)
        assertTrue(started.flags and Intent.FLAG_ACTIVITY_NEW_TASK != 0)
    }

    @Test
    fun packageReplacedIgnoresOtherActions() {
        val receiver = PackageReplacedReceiver().apply { resolver = { entryPoint() } }
        receiver.onReceive(app, Intent(Intent.ACTION_BOOT_COMPLETED))
        receiver.onReceive(app, Intent("com.example.OTHER"))

        assertTrue(events().isEmpty())
        assertNull(shadowOf(app).nextStartedActivity)
    }

    @Test
    fun packageReplacedStillStartsTheActivityWithoutAGraph() {
        PackageReplacedReceiver().apply { resolver = { null } }.onReceive(app, Intent(Intent.ACTION_MY_PACKAGE_REPLACED))
        assertNotNull(shadowOf(app).nextStartedActivity)
    }

    @Test
    fun packageReplacedSwallowsAFailingLookup() {
        PackageReplacedReceiver().apply { resolver = { error("no graph") } }.onReceive(app, Intent(Intent.ACTION_MY_PACKAGE_REPLACED))
        assertNotNull(shadowOf(app).nextStartedActivity)
    }

    @Test
    fun theStatusActionIsOwnedByTheApplication() {
        assertEquals("dev.nuagemagique.pilipos.platform.update.INSTALL_STATUS", InstallResultReceiver.ACTION_INSTALL_STATUS)
    }
}
