package dev.nuagemagique.pilipos.engine

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import dagger.hilt.android.AndroidEntryPoint
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.di.ApplicationScope
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import timber.log.Timber
import javax.inject.Inject

/**
 * Foreground service that keeps the engine running while the till is open.
 *
 * It opens the engine through [EngineHolder] when it starts and closes it when it is destroyed. The work
 * runs in the application scope, which outlives the service.
 */
@AndroidEntryPoint
class EngineService : Service() {
    @Inject
    lateinit var holder: EngineHolder

    @Inject
    @field:ApplicationScope
    lateinit var applicationScope: CoroutineScope

    override fun onBind(intent: Intent?): IBinder? = null

    @Suppress("TooGenericExceptionCaught")
    override fun onCreate() {
        super.onCreate()
        createChannel()
        val notification = buildNotification()
        try {
            ServiceCompat.startForeground(this, NOTIFICATION_ID, notification, FOREGROUND_TYPE)
        } catch (e: Exception) {
            // Both forms worked on the device in spike S4; the plain form is the fallback.
            Timber.w(e, "Typed startForeground refused, falling back to the untyped form")
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        applicationScope.launch { holder.open() }
        return START_STICKY
    }

    override fun onDestroy() {
        applicationScope.launch { holder.close() }
        super.onDestroy()
    }

    private fun createChannel() {
        val manager = getSystemService(NotificationManager::class.java)
        val channel =
            NotificationChannel(
                CHANNEL_ID,
                getString(R.string.engine_channel_name),
                NotificationManager.IMPORTANCE_LOW,
            )
        manager.createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification =
        NotificationCompat
            .Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentTitle(getString(R.string.engine_notification_title))
            .setContentText(getString(R.string.engine_notification_text))
            .setOngoing(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .build()

    companion object {
        /** Id of the notification channel of the engine service. */
        const val CHANNEL_ID = "engine"

        /** Id of the ongoing notification. */
        const val NOTIFICATION_ID = 1

        @SuppressLint("InlinedApi")
        private const val FOREGROUND_TYPE = ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE

        /** Starts the service in the foreground; call it while the app is visible (Android 12+ rules). */
        fun start(context: Context) {
            ContextCompat.startForegroundService(context, Intent(context, EngineService::class.java))
        }
    }
}
