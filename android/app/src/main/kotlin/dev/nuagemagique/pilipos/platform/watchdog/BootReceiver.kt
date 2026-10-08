package dev.nuagemagique.pilipos.platform.watchdog

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import timber.log.Timber

/**
 * Restarts the till after the device booted.
 *
 * Stub: only logs. Filled by P6 (platform lane).
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        Timber.d("BootReceiver: %s", intent.action)
    }
}
