package dev.nuagemagique.pilipos.debugtools

import android.app.Activity
import android.app.admin.DevicePolicyManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.android.EntryPointAccessors
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.kiosk.KioskController
import dev.nuagemagique.pilipos.provisioning.DeviceOwner

/**
 * Debug builds only: ends the session from adb (the emulator device check, P4-T12). Protected by the
 * DUMP permission, held by the shell user:
 * `adb shell am broadcast -a dev.nuagemagique.pilipos.debug.TEARDOWN` with `-n <package>/<this class>`.
 *
 * It releases the kiosk (Lock Task, suspended launchers, persistent HOME, flag) and then clears the
 * device owner, so the same emulator can be provisioned again.
 */
class DebugTeardownReceiver : BroadcastReceiver() {
    /**
     * Hilt access for the receiver. An `@AndroidEntryPoint` receiver would need `super.onReceive`, which
     * the Kotlin compiler rejects as an abstract call ("Abstract member cannot be accessed directly").
     */
    @EntryPoint
    @InstallIn(SingletonComponent::class)
    interface Access {
        /** The kiosk controller of the process. */
        fun kiosk(): KioskController

        /** The device-owner role of this app. */
        fun deviceOwner(): DeviceOwner
    }

    @Suppress("TooGenericExceptionCaught")
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        val access = EntryPointAccessors.fromApplication(context.applicationContext, Access::class.java)
        access.kiosk().release()
        val wasOwner = access.deviceOwner().isDeviceOwner()
        if (wasOwner) {
            try {
                clearOwner(context)
            } catch (e: Exception) {
                Log.w(TAG, "PILI_TEARDOWN clearDeviceOwnerApp failed", e)
            }
        }
        Log.i(TAG, "PILI_TEARDOWN done owner=$wasOwner")
        if (isOrderedBroadcast) resultCode = Activity.RESULT_OK
    }

    // clearDeviceOwnerApp is deprecated for apps but still works on API 33 for the owner itself; there
    // is no other way to drop the role from inside the app, and this receiver exists in debug builds only.
    @Suppress("DEPRECATION")
    private fun clearOwner(context: Context) {
        context.getSystemService(DevicePolicyManager::class.java).clearDeviceOwnerApp(context.packageName)
    }

    private companion object {
        const val TAG = "PiliKiosk"
    }
}
