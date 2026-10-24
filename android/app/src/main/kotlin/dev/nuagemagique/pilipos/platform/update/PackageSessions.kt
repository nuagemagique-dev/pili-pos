package dev.nuagemagique.pilipos.platform.update

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.content.pm.PackageManager
import dagger.hilt.android.qualifiers.ApplicationContext
import java.io.File
import javax.inject.Inject

/** One open install session (a seam over [PackageInstaller.Session]: tests use a fake). */
interface InstallSession {
    /** The session id the system gave. */
    val id: Int

    /** Writes [apk] into the session and syncs it. */
    fun write(apk: File)

    /** Commits the session; the system reports to [InstallResultReceiver]. Not undoable. */
    fun commit()

    /** Releases the session without committing. */
    fun close()
}

/** Creates install sessions and abandons them (a seam over [PackageInstaller]). */
interface PackageSessions {
    /** Creates and opens a full-install session for this application, for an APK of [sizeBytes]. */
    fun create(sizeBytes: Long): InstallSession

    /** Abandons the session [id]; a session that is gone is not an error. */
    fun abandon(id: Int)
}

/**
 * The real [PackageSessions] (PD215, Q18): `MODE_FULL_INSTALL` for the own package, install reason POLICY and
 * `USER_ACTION_NOT_REQUIRED`, which a device owner may use for a silent update. The status goes to
 * [InstallResultReceiver] through an explicit, mutable broadcast `PendingIntent` (the system adds the
 * extras). The manifest declares no `REQUEST_INSTALL_PACKAGES`.
 */
class AndroidPackageSessions
    @Inject
    constructor(
        @ApplicationContext private val context: Context,
    ) : PackageSessions {
        private val installer: PackageInstaller get() = context.packageManager.packageInstaller

        override fun create(sizeBytes: Long): InstallSession {
            val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL)
            params.setAppPackageName(context.packageName)
            params.setInstallReason(PackageManager.INSTALL_REASON_POLICY)
            params.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
            params.setSize(sizeBytes)
            val id = installer.createSession(params)
            return try {
                RealSession(id, installer.openSession(id), sizeBytes)
            } catch (e: java.io.IOException) {
                abandon(id)
                throw e
            }
        }

        override fun abandon(id: Int) {
            try {
                installer.abandonSession(id)
            } catch (
                @Suppress("TooGenericExceptionCaught", "SwallowedException") e: RuntimeException,
            ) {
                // Already finished or abandoned: nothing left to release.
                timber.log.Timber.d(e, "AndroidPackageSessions: session %d is gone", id)
            }
        }

        private inner class RealSession(
            override val id: Int,
            private val session: PackageInstaller.Session,
            private val size: Long,
        ) : InstallSession {
            override fun write(apk: File) {
                apk.inputStream().use { input ->
                    session.openWrite(ENTRY_NAME, 0, size).use { out ->
                        input.copyTo(out, BUFFER_SIZE)
                        session.fsync(out)
                    }
                }
            }

            override fun commit() {
                val intent = Intent(context, InstallResultReceiver::class.java).setAction(InstallResultReceiver.ACTION_INSTALL_STATUS)
                val flags = PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
                session.commit(PendingIntent.getBroadcast(context, id, intent, flags).intentSender)
            }

            override fun close() = session.close()
        }

        private companion object {
            const val ENTRY_NAME = "base.apk"
            const val BUFFER_SIZE = 64 * 1024
        }
    }
