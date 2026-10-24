package dev.nuagemagique.pilipos.platform.update

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import dagger.hilt.android.EntryPointAccessors
import kotlinx.coroutines.launch
import timber.log.Timber
import javax.inject.Inject

/**
 * Acts on the PackageInstaller status of an update session (PD215): audits it and, when the install did not
 * go through, deletes the marker and reopens the engine. Separate from the receiver so that tests call it
 * without a broadcast.
 */
class InstallStatusHandler
    @Inject
    constructor(
        private val audit: dev.nuagemagique.pilipos.platform.audit.PlatformAudit,
        private val sessions: PackageSessions,
        private val restarter: EngineRestarter,
        private val signal: InstallSignal,
        private val markers: UpdateMarkerStore,
    ) {
        /** Handles one status. */
        suspend fun handle(
            status: Int,
            sessionId: Int,
            message: String?,
        ) {
            signal.complete()
            when (status) {
                PackageInstaller.STATUS_SUCCESS -> audit.append(KIND, "status_success", mapOf("session" to sessionId.toString()))
                PackageInstaller.STATUS_PENDING_USER_ACTION -> {
                    // A silent install was refused: the confirmation screen is never shown on the till.
                    sessions.abandon(sessionId)
                    failed(InstallRefusal.SILENT_INSTALL_REFUSED.id, status, message)
                }
                else -> failed(InstallRefusal.INSTALL_FAILED.id, status, message)
            }
        }

        private suspend fun failed(
            code: String,
            status: Int,
            message: String?,
        ) {
            audit.append(KIND, "failed", mapOf("code" to code, "status" to status.toString(), "message" to message.orEmpty()))
            markers.delete()
            restarter.restart()
        }

        private companion object {
            const val KIND = UpdateVerifier.KIND
        }
    }

/**
 * Receives the PackageInstaller status of an update session.
 *
 * Only [ACTION_INSTALL_STATUS] is handled; any other action is ignored. The work runs in the application
 * scope under `goAsync`. Nothing is thrown out of [onReceive]: the graph may not be ready, and a failure here
 * must never crash the till.
 */
class InstallResultReceiver : BroadcastReceiver() {
    /** Looks the Hilt objects up; null when the application has no graph (tests). */
    internal var resolver: (Context) -> UpdateEntryPoint? = { resolveUpdateEntryPoint(it, TAG) }

    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        if (intent.action != ACTION_INSTALL_STATUS) return
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
        val sessionId = intent.getIntExtra(PackageInstaller.EXTRA_SESSION_ID, -1)
        val message = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE)
        Timber.d("%s: status %d session %d", TAG, status, sessionId)
        try {
            val entry = resolver(context) ?: return
            val pending = goAsync()
            entry.applicationScope().launch {
                try {
                    entry.statusHandler().handle(status, sessionId, message)
                } catch (
                    @Suppress("TooGenericExceptionCaught") e: Exception,
                ) {
                    Timber.w(e, "%s: the status could not be handled", TAG)
                } finally {
                    pending?.finish()
                }
            }
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "%s: failed", TAG)
        }
    }

    companion object {
        /** The action of the status broadcast the installer session reports to. */
        const val ACTION_INSTALL_STATUS = "dev.nuagemagique.pilipos.platform.update.INSTALL_STATUS"

        private const val TAG = "InstallResultReceiver"
    }
}

internal fun resolveUpdateEntryPoint(
    context: Context,
    tag: String,
): UpdateEntryPoint? =
    try {
        EntryPointAccessors.fromApplication(context.applicationContext, UpdateEntryPoint::class.java)
    } catch (
        @Suppress("TooGenericExceptionCaught") e: Exception,
    ) {
        Timber.w(e, "%s: no Hilt graph", tag)
        null
    }
