package dev.nuagemagique.pilipos.platform.watchdog

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.android.EntryPointAccessors
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import timber.log.Timber

/** What the boot receiver needs from the Hilt graph; a receiver cannot be constructor-injected. */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface BootEntryPoint {
    fun platformAudit(): PlatformAudit

    @ApplicationScope
    fun applicationScope(): CoroutineScope
}

/**
 * Audits the boot and starts [MainActivity] as a fallback to the HOME role (PD207).
 *
 * Any other action is ignored. Nothing is thrown out of [onReceive]: the receiver may run in a process whose
 * graph is not ready, and a failure here must never crash the till.
 */
class BootReceiver : BroadcastReceiver() {
    /** Looks the Hilt objects up; null when the application has no graph (tests). */
    internal var resolver: (Context) -> BootEntryPoint? = { resolveEntryPoint(it) }

    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        Timber.d("BootReceiver: %s", intent.action)
        auditBoot(context)
        startMainActivity(context)
    }

    private fun auditBoot(context: Context) {
        try {
            val entry = resolver(context) ?: return
            val pending = goAsync()
            entry.applicationScope().launch {
                try {
                    entry.platformAudit().append("boot", "completed")
                } finally {
                    pending?.finish()
                }
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "BootReceiver: audit failed")
        }
    }

    private fun startMainActivity(context: Context) {
        try {
            context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "BootReceiver: could not start the main activity")
        }
    }

    private companion object {
        fun resolveEntryPoint(context: Context): BootEntryPoint? =
            try {
                EntryPointAccessors.fromApplication(context.applicationContext, BootEntryPoint::class.java)
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "BootReceiver: no Hilt graph")
                null
            }
    }
}
