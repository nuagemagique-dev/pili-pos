package dev.nuagemagique.pilipos.kiosk

import android.app.Activity
import android.app.admin.DevicePolicyManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.test.core.app.ApplicationProvider
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.debugtools.DebugTeardownReceiver
import dev.nuagemagique.pilipos.provisioning.DeviceOwner
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowBroadcastPendingResult
import org.robolectric.shadows.ShadowLog
import javax.inject.Inject

@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class DebugTeardownReceiverTest {
    @get:Rule
    val hiltRule = HiltAndroidRule(this)

    @Inject
    lateinit var kiosk: KioskController

    @Inject
    lateinit var deviceOwner: DeviceOwner

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val manager: DevicePolicyManager = context.getSystemService(DevicePolicyManager::class.java)

    private fun orderedResult(): BroadcastReceiver.PendingResult =
        ShadowBroadcastPendingResult::class.java
            .getDeclaredMethod("create", Int::class.java, String::class.java, Bundle::class.java, Boolean::class.java)
            .apply { isAccessible = true }
            .invoke(null, Activity.RESULT_CANCELED, null, null, true) as BroadcastReceiver.PendingResult

    @Before
    fun inject() {
        hiltRule.inject()
    }

    private fun broadcast(): DebugTeardownReceiver {
        val receiver = DebugTeardownReceiver()
        BroadcastReceiver::class.java
            .getMethod("setPendingResult", BroadcastReceiver.PendingResult::class.java)
            .invoke(receiver, orderedResult())
        receiver.onReceive(context, Intent("dev.nuagemagique.pilipos.debug.TEARDOWN"))
        return receiver
    }

    @Test
    fun theOwnerIsReleasedAndTheResultIsOk() {
        shadowOf(manager).setDeviceOwner(deviceOwner.adminComponent)
        kiosk.prepare()
        val prefs = context.getSharedPreferences("kiosk", Context.MODE_PRIVATE)
        assertTrue(prefs.getBoolean("enabled", false))
        val receiver = broadcast()
        assertFalse(prefs.getBoolean("enabled", true))
        assertEquals(0, manager.getLockTaskPackages(deviceOwner.adminComponent).size)
        assertEquals(Activity.RESULT_OK, receiver.resultCode)
        assertTrue(ShadowLog.getLogsForTag("PiliKiosk").any { it.msg.contains("PILI_TEARDOWN done owner=true") })
    }

    @Test
    fun withoutTheOwnerRoleTheTeardownStillAnswersOk() {
        val receiver = broadcast()
        assertEquals(Activity.RESULT_OK, receiver.resultCode)
        assertTrue(ShadowLog.getLogsForTag("PiliKiosk").any { it.msg.contains("PILI_TEARDOWN done owner=false") })
    }
}
