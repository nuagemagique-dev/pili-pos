package dev.nuagemagique.pilipos.platform.update

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import timber.log.Timber

/**
 * Receives the PackageInstaller status of an update session.
 *
 * Stub: only logs. Filled by P6 (platform lane).
 */
class InstallResultReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        Timber.d("InstallResultReceiver: %s", intent.action)
    }
}
