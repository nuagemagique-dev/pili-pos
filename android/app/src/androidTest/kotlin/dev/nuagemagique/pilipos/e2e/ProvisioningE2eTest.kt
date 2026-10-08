package dev.nuagemagique.pilipos.e2e

import android.app.ActivityManager
import android.app.admin.DevicePolicyManager
import android.content.Context
import android.os.Build
import android.os.SystemClock
import android.view.MotionEvent
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import dagger.hilt.android.EntryPointAccessors
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.debugtools.E2eEntryPoint
import dev.nuagemagique.pilipos.engine.EngineState
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.pos_ffi.PosEvent
import java.io.IOException
import java.net.InetSocketAddress
import java.net.Socket

/**
 * P4-T12 (PD113): the foundation of the app end to end on the emulator, with the real application, the real
 * Hilt graph and the host FDM mock reached through adb reverse. `scripts/android/app_e2e.sh` prepares the
 * device (device owner, development bundle in the debug inbox, adb reverse rules) and runs this class with
 * `am instrument`; it fails loudly, never skips, when a precondition is missing.
 *
 * Instrumentation arguments: `managerInsz` (default 94060242576) and `managerPin` (default 1111), the
 * seeded manager of src-tauri/seed_spf_test.sql, and `fdmPort` (default 19380), the device port of the FDM
 * mock.
 */
@RunWith(AndroidJUnit4::class)
class ProvisioningE2eTest {
    @get:Rule
    val compose = createAndroidComposeRule<MainActivity>()

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = instrumentation.targetContext
    private val arguments get() = InstrumentationRegistry.getArguments()
    private val insz: String get() = arguments.getString("managerInsz") ?: DEFAULT_INSZ
    private val pin: String get() = arguments.getString("managerPin") ?: DEFAULT_PIN
    private val fdmPort: Int get() = arguments.getString("fdmPort")?.toInt() ?: DEFAULT_FDM_PORT

    private val app: E2eEntryPoint
        get() = EntryPointAccessors.fromApplication(context.applicationContext, E2eEntryPoint::class.java)

    @Before
    fun theFdmMockIsReachable() {
        // A missing mock is a failure with a clear message, never a skipped test.
        try {
            Socket().use { it.connect(InetSocketAddress("127.0.0.1", fdmPort), CONNECT_TIMEOUT_MS) }
        } catch (e: IOException) {
            throw AssertionError(
                "FDM mock unreachable at 127.0.0.1:$fdmPort from the device (adb reverse missing?): ${e.message}",
                e,
            )
        }
    }

    @Test
    fun theDevelopmentBundleProvisionsTheTillAndTheManagerLogsIn() {
        try {
            provisionAndLogin()
        } finally {
            leaveLockTask()
        }
    }

    /**
     * The kiosk pins the activity in Lock Task: Android then ignores `finish()` of the root activity, so the
     * rule could not close its scenario. The app is on the allowlist, so it may end its own Lock Task.
     */
    private fun leaveLockTask() {
        compose.activityRule.scenario.onActivity { activity ->
            try {
                activity.stopLockTask()
            } catch (e: IllegalStateException) {
                // Not in Lock Task (the test failed before the kiosk locked): nothing to leave.
                android.util.Log.i("ProvisioningE2e", "stopLockTask: ${e.message}")
            }
        }
    }

    private fun provisionAndLogin() {
        // 1. The device is device owner and not provisioned: the provisioning screen shows.
        assertTrue("the app must be the device owner (dpm set-device-owner)", isDeviceOwner())
        compose.waitUntil(UI_TIMEOUT_MS) { nodeExists("shell_provisioning") }
        compose.onNodeWithTag("shell_provisioning").assertIsDisplayed()

        // 2. Import from the debug inbox: the engine becomes Ready with the software version of this build.
        compose.onNodeWithTag("prov_import_inbox").performClick()
        val ready = awaitReady()
        assertEquals("pili-pos-android/1.0.0-dev", ready.posSwVersion)

        // 3. The record binds the bundle to this device: deviceId == Build.getSerial().
        val record = runBlocking { app.provisioning().refresh() } ?: throw AssertionError("no provisioning record after the import")
        assertTrue("blank deviceId", record.deviceId.isNotBlank())
        assertEquals("deviceId must be the hardware serial", serial(), record.deviceId)

        // 4. Lock Task is LOCKED (the kiosk locks on resume once the owner policies are applied).
        waitFor("lock task mode LOCKED", LOCK_TIMEOUT_MS) {
            context.getSystemService(ActivityManager::class.java).lockTaskModeState == ActivityManager.LOCK_TASK_MODE_LOCKED
        }

        // 5. A real touch on the shell raises the user activity count (Activity.onUserInteraction).
        val before = app.forwarder().forwardedCount
        touchShellRoot()
        waitFor("user activity forwarded (count was $before)", ACTIVITY_TIMEOUT_MS) { app.forwarder().forwardedCount >= before + 1 }

        // 6. Subscribe to the event bus first, then log in: an FdmStatus event with connected=true arrives.
        val status = runBlocking(Dispatchers.IO) { loginAndAwaitFdmStatus() }
        assertTrue("the FDM mock must be connected: ${status.health}", status.health.connected)

        // 7. Logout.
        runBlocking { app.gateway().call { it.logout() } }
    }

    private suspend fun loginAndAwaitFdmStatus(): PosEvent.FdmStatus {
        val events = app.events().events
        val pending = CompletableDeferred<PosEvent.FdmStatus>()
        // UNDISPATCHED: the subscription exists before login is called (the bus has no replay).
        val collector =
            CoroutineScope(Dispatchers.IO).launch(start = CoroutineStart.UNDISPATCHED) {
                events.collect { if (it is PosEvent.FdmStatus && it.health.connected) pending.complete(it) }
            }
        try {
            val user = app.gateway().call { it.login(insz, pin) }
            assertEquals(insz, user.insz)
            return withTimeout(FDM_STATUS_TIMEOUT_MS) { pending.await() }
        } finally {
            collector.cancel()
        }
    }

    private fun awaitReady(): EngineState.Ready {
        val state =
            runBlocking {
                withTimeout(IMPORT_TIMEOUT_MS) {
                    app.holder().state.first { it is EngineState.Ready || it is EngineState.Fatal }
                }
            }
        return state as? EngineState.Ready ?: throw AssertionError("the engine did not become Ready: $state")
    }

    private fun nodeExists(tag: String): Boolean = compose.onAllNodesWithTag(tag).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(
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

    private fun isDeviceOwner(): Boolean =
        context.getSystemService(DevicePolicyManager::class.java).isDeviceOwnerApp(context.packageName)

    @Suppress("HardwareIds", "MissingPermission")
    private fun serial(): String = Build.getSerial()

    /** A tap in the middle of the screen, injected through the window so that Activity.onUserInteraction runs. */
    private fun touchShellRoot() {
        compose.onNodeWithTag("shell_root").assertIsDisplayed()
        val metrics = context.resources.displayMetrics
        val x = metrics.widthPixels / 2f
        val y = metrics.heightPixels / 2f
        val down = SystemClock.uptimeMillis()
        val downEvent = MotionEvent.obtain(down, down, MotionEvent.ACTION_DOWN, x, y, 0)
        val upEvent = MotionEvent.obtain(down, down + TAP_MS, MotionEvent.ACTION_UP, x, y, 0)
        try {
            instrumentation.sendPointerSync(downEvent)
            instrumentation.sendPointerSync(upEvent)
        } finally {
            downEvent.recycle()
            upEvent.recycle()
        }
    }

    private companion object {
        const val DEFAULT_INSZ = "94060242576"
        const val DEFAULT_PIN = "1111"
        const val DEFAULT_FDM_PORT = 19380
        const val CONNECT_TIMEOUT_MS = 3_000
        const val UI_TIMEOUT_MS = 20_000L
        const val IMPORT_TIMEOUT_MS = 90_000L
        const val LOCK_TIMEOUT_MS = 15_000L
        const val ACTIVITY_TIMEOUT_MS = 3_000L
        const val FDM_STATUS_TIMEOUT_MS = 20_000L
        const val POLL_MS = 100L
        const val TAP_MS = 50L
    }
}
