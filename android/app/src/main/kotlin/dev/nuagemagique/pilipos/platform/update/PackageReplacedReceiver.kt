package dev.nuagemagique.pilipos.platform.update

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import dev.nuagemagique.pilipos.MainActivity
import kotlinx.coroutines.launch
import timber.log.Timber

/**
 * Brings the till back after the application package was replaced (Q18, third way back into Lock Task): it
 * starts [MainActivity] in a new task and audits `package_replaced` (kind `update`).
 *
 * Any other action is ignored. Nothing is thrown out of [onReceive].
 */
class PackageReplacedReceiver : BroadcastReceiver() {
    /** Looks the Hilt objects up; null when the application has no graph (tests). */
    internal var resolver: (Context) -> UpdateEntryPoint? = { resolveUpdateEntryPoint(it, TAG) }

    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        if (intent.action != Intent.ACTION_MY_PACKAGE_REPLACED) return
        Timber.d("%s: %s", TAG, intent.action)
        startMainActivity(context)
        audit(context)
    }

    private fun startMainActivity(context: Context) {
        try {
            context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "%s: could not start the main activity", TAG)
        }
    }

    private fun audit(context: Context) {
        try {
            val entry = resolver(context) ?: return
            val pending = goAsync()
            entry.applicationScope().launch {
                try {
                    entry.platformAudit().append(UpdateVerifier.KIND, "package_replaced")
                } finally {
                    pending?.finish()
                }
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "%s: audit failed", TAG)
        }
    }

    private companion object {
        const val TAG = "PackageReplacedReceiver"
    }
}
