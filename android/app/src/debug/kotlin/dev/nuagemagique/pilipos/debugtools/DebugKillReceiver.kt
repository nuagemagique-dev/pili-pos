package dev.nuagemagique.pilipos.debugtools

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Process
import android.util.Log

/**
 * Debug builds only: ends the app process with SIGKILL from adb, to test a restart (P4-T12). Protected by
 * the DUMP permission, held by the shell user:
 * `adb shell am broadcast -a dev.nuagemagique.pilipos.debug.KILL -n <package>/<this class>`.
 *
 * Nothing else can do it on API 33: `am force-stop` and `am crash` are ignored for the device-owner
 * package ("protected package"), and `run-as` has its own pid namespace.
 */
class DebugKillReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        Log.i(TAG, "PILI_KILL pid=${Process.myPid()}")
        Process.killProcess(Process.myPid())
    }

    private companion object {
        const val TAG = "PiliKiosk"
    }
}
