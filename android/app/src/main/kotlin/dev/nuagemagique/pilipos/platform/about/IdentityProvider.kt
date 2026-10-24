package dev.nuagemagique.pilipos.platform.about

import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.StatFs
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import dev.nuagemagique.pilipos.platform.audit.SystemPlatformClock
import dev.nuagemagique.pilipos.provisioning.ProvisioningStore
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import timber.log.Timber
import java.io.File
import java.util.zip.ZipFile
import javax.inject.Inject
import javax.inject.Singleton

/**
 * What the installed app says about itself, read from the package manager (a seam so that tests use a fake
 * APK): [sourceDir] is the path of `base.apk`, [lastUpdateTime] changes on every install.
 */
internal data class AppFacts(
    val applicationId: String,
    val versionName: String,
    val versionCode: Long,
    val engineRef: String,
    val sourceDir: String,
    val lastUpdateTime: Long,
    val filesDir: File,
    val signerCertificates: List<ByteArray>,
)

/** The hashes of the installed files, computed once per `(sourceDir, lastUpdateTime)`. */
private data class FileHashes(
    val apkSha1: String?,
    val apkSha256: String?,
    val soSha1: String?,
    val soProblem: String?,
)

private data class CacheKey(
    val sourceDir: String,
    val lastUpdateTime: Long,
)

/**
 * Builds the [IdentityReport] (PD209). [report] never throws, apart from the cancellation of the caller: a
 * field that cannot be read is null (the engine settings while it is not ready, the library hash outside a
 * real APK).
 *
 * The APK and library hashes read tens of megabytes, so they run on the I/O dispatcher, streamed, and are
 * cached for the installed build. The library is read from inside the APK (it is stored uncompressed and is
 * not extracted), so the value is the one of the file the loader maps. To recompute on a host:
 * `unzip -p base.apk lib/arm64-v8a/libpos_ffi.so | shasum`.
 */
@Singleton
class IdentityProvider internal constructor(
    private val facts: () -> AppFacts,
    private val provisioning: ProvisioningStore,
    private val gateway: EngineGateway,
    private val helpers: EngineHelpers,
    private val ioDispatcher: CoroutineDispatcher,
    private val clock: PlatformClock,
) {
    @Inject
    constructor(
        @ApplicationContext context: Context,
        provisioning: ProvisioningStore,
        gateway: EngineGateway,
        helpers: EngineHelpers,
        @IoDispatcher ioDispatcher: CoroutineDispatcher,
    ) : this({ readFacts(context) }, provisioning, gateway, helpers, ioDispatcher, SystemPlatformClock)

    private val lock = Any()
    private var cacheKey: CacheKey? = null
    private var cached: FileHashes? = null

    /** The current report. */
    suspend fun report(): IdentityReport {
        val app = withContext(ioDispatcher) { facts() }
        val hashes = withContext(ioDispatcher) { hashesOf(app) }
        val storage = withContext(ioDispatcher) { storageOf(app.filesDir) }
        val settings = readSettings()
        return IdentityReport(
            applicationId = app.applicationId,
            versionName = app.versionName,
            versionCode = app.versionCode,
            posSwVersion = safely("posSwVersion") { helpers.posSwVersion() },
            engineRef = app.engineRef,
            apkSha1 = hashes.apkSha1,
            apkSha256 = hashes.apkSha256,
            soSha1 = hashes.soSha1,
            soProblem = hashes.soProblem,
            signerSha256 = app.signerCertificates.map { Hasher.sha256(it) },
            deviceId = readDeviceId(),
            posId = settings?.first,
            terminalId = settings?.second,
            androidRelease = Build.VERSION.RELEASE.orEmpty(),
            sdkInt = Build.VERSION.SDK_INT,
            fingerprint = Build.FINGERPRINT.orEmpty(),
            manufacturer = Build.MANUFACTURER.orEmpty(),
            model = Build.MODEL.orEmpty(),
            freeInternalBytes = storage.free,
            backupsCount = storage.backupsCount,
            backupsBytes = storage.backupsBytes,
            generatedAt = PlatformAudit.format(clock.nowMillis()),
        )
    }

    private fun hashesOf(app: AppFacts): FileHashes {
        val key = CacheKey(app.sourceDir, app.lastUpdateTime)
        synchronized(lock) {
            val hit = cached
            if (hit != null && cacheKey == key) return hit
        }
        val fresh = computeHashes(app.sourceDir)
        // A failed read is not cached: the next report tries again.
        if (fresh.apkSha1 != null) {
            synchronized(lock) {
                cacheKey = key
                cached = fresh
            }
        }
        return fresh
    }

    private fun computeHashes(sourceDir: String): FileHashes {
        val apk = File(sourceDir)
        val (sha1, sha256) =
            try {
                Hasher.sha1AndSha256(apk)
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "IdentityProvider: the APK cannot be hashed")
                return FileHashes(null, null, null, "apk unreadable: ${e.javaClass.simpleName}")
            }
        val (so, problem) = hashLibrary(apk)
        return FileHashes(sha1, sha256, so, problem)
    }

    private fun hashLibrary(apk: File): Pair<String?, String?> =
        try {
            ZipFile(apk).use { zip -> hashEntry(zip) }
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "IdentityProvider: the library entry cannot be hashed")
            null to "the APK is not a readable zip: ${e.javaClass.simpleName}"
        }

    private fun hashEntry(zip: ZipFile): Pair<String?, String?> {
        val entry = zip.getEntry(LIBRARY_ENTRY) ?: return null to "$LIBRARY_ENTRY is not in the APK"
        return zip.getInputStream(entry).use { Hasher.sha1(it) } to null
    }

    private suspend fun readDeviceId(): String? =
        safely("deviceId") { provisioning.record.value?.deviceId ?: provisioning.refresh()?.deviceId }

    private suspend fun readSettings(): Pair<String, String>? =
        if (!gateway.ready.value) {
            null
        } else {
            safely("settings") { gateway.call { it.getSettings() }.let { it.posId to it.terminalId } }
        }

    private data class Storage(
        val free: Long,
        val backupsCount: Int,
        val backupsBytes: Long,
    )

    private fun storageOf(filesDir: File): Storage {
        val free =
            try {
                StatFs(filesDir.path).availableBytes
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "IdentityProvider: the free space is unavailable")
                0L
            }
        val backups =
            File(filesDir, BACKUPS_DIR).listFiles { f -> f.isFile && f.name.endsWith(".db") }.orEmpty()
        return Storage(free, backups.size, backups.sumOf { it.length() })
    }

    /** Runs [block]; a failure other than a cancellation is logged and read as null. */
    private suspend fun <T> safely(
        what: String,
        block: suspend () -> T?,
    ): T? =
        try {
            block()
        } catch (e: CancellationException) {
            throw e
        } catch (
            // UnsatisfiedLinkError (no native library on the host) is an Error, not an Exception.
            @Suppress("TooGenericExceptionCaught") e: Throwable,
        ) {
            Timber.w(e, "IdentityProvider: %s unavailable", what)
            null
        }

    companion object {
        /** Entry of the engine library inside `base.apk`. */
        const val LIBRARY_ENTRY = "lib/arm64-v8a/libpos_ffi.so"

        /** Backup directory under `filesDir`. */
        const val BACKUPS_DIR = "engine/backups"

        private fun readFacts(context: Context): AppFacts {
            val info =
                context.packageManager.getPackageInfo(
                    context.packageName,
                    PackageManager.PackageInfoFlags.of(PackageManager.GET_SIGNING_CERTIFICATES.toLong()),
                )
            val signers =
                try {
                    info.signingInfo?.apkContentsSigners.orEmpty().map { it.toByteArray() }
                } catch (
                    @Suppress("TooGenericExceptionCaught") e: Exception,
                ) {
                    Timber.w(e, "IdentityProvider: the signing certificates are unavailable")
                    emptyList()
                }
            return AppFacts(
                applicationId = context.packageName,
                versionName = info.versionName.orEmpty(),
                versionCode = info.longVersionCode,
                engineRef = context.getString(R.string.plat_about_engine_ref),
                sourceDir = context.applicationInfo.sourceDir,
                lastUpdateTime = info.lastUpdateTime,
                filesDir = context.filesDir,
                signerCertificates = signers,
            )
        }
    }
}
