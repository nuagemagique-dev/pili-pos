package dev.nuagemagique.pilipos.provisioning

import android.app.admin.DeviceAdminReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import timber.log.Timber

/**
 * Device administration receiver; the device-owner role is granted at provisioning (adb or QR code).
 *
 * It only logs the lifecycle callbacks: the owner policies are applied by the kiosk code, not here.
 */
class PiliAdminReceiver : DeviceAdminReceiver() {
    override fun onEnabled(
        context: Context,
        intent: Intent,
    ) {
        Timber.i("provisioning: device admin enabled")
    }

    override fun onDisabled(
        context: Context,
        intent: Intent,
    ) {
        Timber.i("provisioning: device admin disabled")
    }

    companion object {
        /** The component of this receiver, as the device policy manager names the admin. */
        fun component(context: Context): ComponentName = ComponentName(context, PiliAdminReceiver::class.java)
    }
}
