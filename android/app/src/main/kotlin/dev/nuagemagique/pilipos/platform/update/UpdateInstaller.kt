package dev.nuagemagique.pilipos.platform.update

import android.content.Context
import android.content.pm.PackageManager
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.engine.CloseResult
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineService
import dev.nuagemagique.pilipos.platform.PlatformOps
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import dev.nuagemagique.pilipos.platform.audit.SystemPlatformClock
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import timber.log.Timber
import java.io.File
import javax.inject.Inject
import javax.inject.Singleton

/** Reopens the engine after an update was given up (a seam: production starts [EngineService]). */
fun interface EngineRestarter {
    /** Asks for the engine to be opened again; never throws. */
    fun restart()
}

/** The production [EngineRestarter]: `EngineService.start`, which opens the engine through the holder. */
class ServiceEngineRestarter
    @Inject
    constructor(
        @ApplicationContext private val context: Context,
    ) : EngineRestarter {
        override fun restart() {
            try {
                EngineService.start(context)
            } catch (
                @Suppress("TooGenericExceptionCaught") e: RuntimeException,
            ) {
                Timber.w(e, "EngineRestarter: the engine service could not be started")
            }
        }
    }

/**
 * Tells [UpdateInstaller] that the system answered: the receiver completes it, so that the installer stops
 * holding the `update` operation (the watchdog keeps away while the install runs). The installer never
 * decides anything from it.
 */
@Singleton
class InstallSignal
    @Inject
    constructor() {
        @Volatile
        private var pending = CompletableDeferred<Unit>()

        /** Arms the signal for a new install. */
        fun begin() {
            pending = CompletableDeferred()
        }

        /** The system answered. */
        fun complete() {
            pending.complete(Unit)
        }

        /** Waits for [complete], at most [millis]. */
        suspend fun await(millis: Long) {
            if (millis > 0) withTimeoutOrNull(millis) { pending.await() }
        }
    }

/** What [UpdateInstaller.install] did. */
sealed interface InstallOutcome {
    /**
     * The session was committed. The process is normally killed by the install; the result is not awaited, so
     * this is never a promise that the new version runs.
     */
    data object Committed : InstallOutcome

    /** A precondition failed (nothing was touched). */
    data class Blocked(
        val failures: List<PreconditionFailure>,
    ) : InstallOutcome

    /** The install stopped for [reason]; the engine was reopened and the marker deleted. */
    data class Refused(
        val reason: InstallRefusal,
        val detail: String? = null,
    ) : InstallOutcome
}

/**
 * Installs a [VerifiedUpdate] over the running application (PD215, Q18, S4 section 7). In this order:
 *
 * 1. the `update` operation starts ([PlatformOps], so that the watchdog leaves the engine alone);
 * 2. the preconditions again ([UpdatePreconditions]; a failure is [InstallOutcome.Blocked]);
 * 3. `backupDatabase()` (a failure refuses with [InstallRefusal.BACKUP_FAILED]);
 * 4. `EngineGateway.close()` must give [CloseResult.Closed]; [CloseResult.StillOpen] refuses with
 *    [InstallRefusal.ENGINE_BUSY] and reopens the engine;
 * 5. the fsynced marker `pending.json` ([UpdateMarkerStore]);
 * 6. the audit row `started`;
 * 7. the PackageInstaller session ([PackageSessions]): create, write the APK, commit.
 *
 * Any exception from step 5 on, before the commit returns, abandons the session, deletes the marker, reopens
 * the engine and audits `failed`. After the commit the result is not awaited: a successful install kills this
 * process, and a failure arrives at [InstallResultReceiver]. The call only keeps the `update` operation
 * running for up to [holdMillis] (or until [InstallSignal] fires) so that the watchdog stays out meanwhile.
 *
 * Audit rows (kind `update`): `install_blocked`, `install_refused`, `started`, `failed`.
 */
@Singleton
@Suppress("LongParameterList")
class UpdateInstaller internal constructor(
    private val gateway: EngineGateway,
    private val preconditions: UpdatePreconditions,
    private val ops: PlatformOps,
    private val audit: PlatformAudit,
    private val markers: UpdateMarkerStore,
    private val sessions: PackageSessions,
    private val restarter: EngineRestarter,
    private val signal: InstallSignal,
    private val versionCode: () -> Long,
    private val clock: PlatformClock,
    private val ioDispatcher: CoroutineDispatcher,
    private val holdMillis: Long,
) {
    @Inject
    constructor(
        @ApplicationContext context: Context,
        gateway: EngineGateway,
        preconditions: UpdatePreconditions,
        ops: PlatformOps,
        audit: PlatformAudit,
        markers: UpdateMarkerStore,
        sessions: PackageSessions,
        restarter: EngineRestarter,
        signal: InstallSignal,
        @IoDispatcher ioDispatcher: CoroutineDispatcher,
    ) : this(
        gateway = gateway,
        preconditions = preconditions,
        ops = ops,
        audit = audit,
        markers = markers,
        sessions = sessions,
        restarter = restarter,
        signal = signal,
        versionCode = { installedVersionCode(context) },
        clock = SystemPlatformClock,
        ioDispatcher = ioDispatcher,
        holdMillis = HOLD_MILLIS,
    )

    /** Runs the whole sequence for [verified]; never throws apart from the cancellation of the caller. */
    suspend fun install(verified: VerifiedUpdate): InstallOutcome = ops.run(OPERATION) { execute(verified) }

    private suspend fun execute(verified: VerifiedUpdate): InstallOutcome {
        val failures = preconditions.check()
        return if (failures.isNotEmpty()) blocked(failures) else backup() ?: closeEngine() ?: commit(verified)
    }

    private suspend fun blocked(failures: List<PreconditionFailure>): InstallOutcome {
        audit.append(KIND, "install_blocked", mapOf("failures" to failures.joinToString(",") { it.id }))
        return InstallOutcome.Blocked(failures)
    }

    /** Null when the backup was made. */
    private suspend fun backup(): InstallOutcome? =
        try {
            gateway.call { it.backupDatabase() }
            null
        } catch (e: CancellationException) {
            throw e
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "UpdateInstaller: the backup failed")
            refused(InstallRefusal.BACKUP_FAILED, e.message)
        }

    /** Null when the engine is closed. */
    private suspend fun closeEngine(): InstallOutcome? {
        val result =
            try {
                gateway.close()
            } catch (e: CancellationException) {
                withContext(NonCancellable) { restarter.restart() }
                throw e
            }
        return when (result) {
            CloseResult.Closed -> null
            is CloseResult.StillOpen -> {
                restarter.restart()
                refused(InstallRefusal.ENGINE_BUSY, result.detail)
            }
        }
    }

    private suspend fun refused(
        reason: InstallRefusal,
        detail: String?,
    ): InstallOutcome {
        audit.append(KIND, "install_refused", mapOf("code" to reason.id, "detail" to detail.orEmpty()))
        return InstallOutcome.Refused(reason, detail)
    }

    private suspend fun commit(verified: VerifiedUpdate): InstallOutcome {
        var session: InstallSession? = null
        try {
            signal.begin()
            val marker =
                UpdateMarker(
                    fromVersionCode = withContext(ioDispatcher) { versionCode() },
                    toVersionCode = verified.release.versionCode,
                    version = verified.release.version,
                    sha1 = verified.sha1,
                    startedAt = clock.nowMillis(),
                )
            withContext(ioDispatcher) { markers.write(marker) }
            audit.append(KIND, "started", marker.auditDetail())
            val opened = withContext(ioDispatcher) { open(verified.file) }
            session = opened
            withContext(ioDispatcher) { opened.commit() }
        } catch (e: CancellationException) {
            withContext(NonCancellable) { rollback(session, "cancelled") }
            throw e
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.e(e, "UpdateInstaller: the install could not be committed")
            val detail = "${e.javaClass.simpleName}: ${e.message}"
            withContext(NonCancellable) { rollback(session, detail) }
            return InstallOutcome.Refused(InstallRefusal.INSTALL_FAILED, detail)
        } finally {
            session?.let { withContext(NonCancellable) { closeQuietly(it) } }
        }
        signal.await(holdMillis)
        return InstallOutcome.Committed
    }

    private fun open(apk: File): InstallSession {
        val session = sessions.create(apk.length())
        try {
            session.write(apk)
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            closeQuietly(session)
            sessions.abandon(session.id)
            throw e
        }
        return session
    }

    /** Undoes what the install did before the commit: session, marker, engine, and one audit row. */
    private suspend fun rollback(
        session: InstallSession?,
        detail: String,
    ) {
        withContext(ioDispatcher) {
            session?.let { sessions.abandon(it.id) }
            markers.delete()
        }
        restarter.restart()
        audit.append(KIND, "failed", mapOf("code" to InstallRefusal.INSTALL_FAILED.id, "detail" to detail))
    }

    private fun closeQuietly(session: InstallSession) {
        try {
            session.close()
        } catch (
            @Suppress("TooGenericExceptionCaught") e: RuntimeException,
        ) {
            Timber.d(e, "UpdateInstaller: the session could not be closed")
        }
    }

    companion object {
        /** Name of the operation in [PlatformOps]. */
        const val OPERATION = "update"

        /** The audit `kind`. */
        const val KIND = UpdateVerifier.KIND

        /** How long a committed install keeps the operation open at most. */
        const val HOLD_MILLIS = 90_000L

        private fun installedVersionCode(context: Context): Long =
            context.packageManager.getPackageInfo(context.packageName, PackageManager.PackageInfoFlags.of(0L)).longVersionCode
    }
}
