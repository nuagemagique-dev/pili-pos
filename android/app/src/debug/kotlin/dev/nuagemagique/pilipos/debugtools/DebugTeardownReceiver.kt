package dev.nuagemagique.pilipos.debugtools

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import timber.log.Timber

/**
 * Debug builds only: ends the session from adb. Protected by the DUMP permission, held by the shell.
 *
 * Stub: only logs. P4-T10 stops the engine and leaves the kiosk.
 */
class DebugTeardownReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        Timber.d("DebugTeardownReceiver: %s", intent.action)
    }
}
