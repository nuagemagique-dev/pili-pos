package dev.nuagemagique.pilipos.platform.update

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import timber.log.Timber

/**
 * Restarts the till after the application package was replaced.
 *
 * Stub: only logs. Filled by P6 (platform lane).
 */
class PackageReplacedReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        Timber.d("PackageReplacedReceiver: %s", intent.action)
    }
}
