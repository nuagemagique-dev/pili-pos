package dev.nuagemagique.pilipos.platform.update

import android.app.Application
import android.content.Context
import android.content.pm.PackageManager
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.di.AppLifecycleHook
import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.platform.about.IdentityProvider
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import dev.nuagemagique.pilipos.platform.audit.SystemPlatformClock
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Closes the loop of an update in the process that starts after it (PD215): when `pending.json` exists,
 *
 * - the running `versionCode` equals the marker's `toVersionCode`: audit `done` (with the marker's versions
 *   and the SHA-1 of the installed APK, read from [IdentityProvider]) and delete the marker;
 * - it equals `fromVersionCode` and the marker is more than [STALE_MILLIS] old: the install did not happen,
 *   audit `not_completed` and delete the marker (younger than that, the install may still be running: the
 *   marker stays);
 * - it is neither: audit `not_completed` (reason `unexpected_version`) and delete the marker;
 * - the marker is not readable: audit `marker_corrupt` and delete it.
 *
 * Runs once at application start, in the application scope. Audit rows have the kind `update`.
 */
@Singleton
@Suppress("LongParameterList")
class UpdateCompletionHook internal constructor(
    private val markers: UpdateMarkerStore,
    private val audit: PlatformAudit,
    private val identity: suspend () -> String?,
    private val versionCode: () -> Long,
    private val clock: PlatformClock,
    private val scope: CoroutineScope,
    private val ioDispatcher: CoroutineDispatcher,
) : AppLifecycleHook {
    @Inject
    constructor(
        @ApplicationContext context: Context,
        markers: UpdateMarkerStore,
        audit: PlatformAudit,
        identity: IdentityProvider,
        @ApplicationScope scope: CoroutineScope,
        @IoDispatcher ioDispatcher: CoroutineDispatcher,
    ) : this(
        markers = markers,
        audit = audit,
        identity = { identity.report().apkSha1 },
        versionCode = { runningVersionCode(context) },
        clock = SystemPlatformClock,
        scope = scope,
        ioDispatcher = ioDispatcher,
    )

    override val order: Int = ORDER

    override fun onApplicationCreated(app: Application) {
        scope.launch(ioDispatcher) { complete() }
    }

    /** Looks at the marker and writes the row it calls for; never throws apart from cancellation. */
    internal suspend fun complete() {
        try {
            when (val read = withContext(ioDispatcher) { markers.read() }) {
                MarkerRead.Missing -> Unit
                is MarkerRead.Corrupt -> finish("marker_corrupt", mapOf("reason" to read.reason))
                is MarkerRead.Found -> judge(read.marker)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "UpdateCompletionHook: failed")
        }
    }

    private suspend fun judge(marker: UpdateMarker) {
        val running = withContext(ioDispatcher) { versionCode() }
        val detail = mapOf("from" to "${marker.fromVersionCode}", "to" to "${marker.toVersionCode}", "version" to marker.version)
        when {
            running == marker.toVersionCode -> {
                val apkSha1 = identity()
                finish("done", detail + mapOf("running" to "$running", "apkSha1" to apkSha1.orEmpty(), "declaredSha1" to marker.sha1))
            }
            running == marker.fromVersionCode && clock.nowMillis() - marker.startedAt <= STALE_MILLIS -> Unit
            running == marker.fromVersionCode -> finish("not_completed", detail + mapOf("reason" to "still_old_version"))
            else -> finish("not_completed", detail + mapOf("reason" to "unexpected_version", "running" to "$running"))
        }
    }

    private suspend fun finish(
        event: String,
        detail: Map<String, String>,
    ) {
        audit.append(UpdateVerifier.KIND, event, detail)
        withContext(ioDispatcher) { markers.delete() }
    }

    companion object {
        /** Position among the lifecycle hooks: after the policies (100) and the watchdog (200). */
        const val ORDER = 300

        /** A marker of the old version younger than this may belong to an install that is still running. */
        const val STALE_MILLIS: Long = 5L * 60L * 1000L

        private fun runningVersionCode(context: Context): Long =
            context.packageManager.getPackageInfo(context.packageName, PackageManager.PackageInfoFlags.of(0L)).longVersionCode
    }
}
