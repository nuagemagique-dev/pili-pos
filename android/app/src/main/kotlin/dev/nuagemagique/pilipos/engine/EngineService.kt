package dev.nuagemagique.pilipos.engine

import android.app.Service
import android.content.Intent
import android.os.IBinder

/**
 * Foreground service that keeps the engine running while the till is open.
 *
 * Stub: P4-T5 starts the engine from the launch configuration and posts the foreground notification.
 */
class EngineService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int = START_NOT_STICKY
}
