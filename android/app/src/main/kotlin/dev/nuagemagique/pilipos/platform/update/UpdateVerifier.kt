package dev.nuagemagique.pilipos.platform.update

import android.content.Context
import android.content.pm.PackageManager
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.platform.about.Hasher
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import timber.log.Timber
import java.io.File
import java.io.IOException
import javax.inject.Inject
import javax.inject.Singleton

/** What the package manager says about an APK on disk. */
data class ArchiveInfo(
    val packageName: String,
    val versionCode: Long,
    val versionName: String?,
    /** SHA-256 of each signing certificate, lowercase hex. */
    val signerSha256: Set<String>,
)

/** The installed application, as far as the verifier needs it. */
data class InstalledApp(
    val packageName: String,
    val versionCode: Long,
    val signerSha256: Set<String>,
)

/** Reads the manifest and the signers of an APK file (a seam: the real one needs a package manager). */
fun interface ArchiveInspector {
    /** The facts of the APK at [path], or null when it cannot be parsed. */
    fun inspect(path: String): ArchiveInfo?
}

/**
 * The checks between "a source says there is a release" and "an APK is ready to install" (PD215, S4 section
 * 7). It only reads and verifies; it installs nothing.
 *
 * [verify] stops at the first refusal, in this order: a public key for this build; the descriptor; the
 * minisign signature of `declared.json` (only through [EngineHelpers.verifyMinisign]); the strict parse; the
 * package name; a `versionCode` strictly greater than the installed one (the OS accepts an equal one, so the
 * app must refuse it, U5); the download to `filesDir/platform/update/<asset>`; SHA-256 and SHA-1 of the file;
 * the archive info (package, `versionCode`, `versionName`) and the signers (equal to the installed set and
 * containing the declared `signerSha256`). A refusal, and any failure after the download began, deletes the
 * file. Audit rows (kind `update`): `check_started`, then `refused` or `verified`.
 */
@Singleton
@Suppress("LongParameterList")
class UpdateVerifier internal constructor(
    private val keys: () -> UpdateKeys,
    private val helpers: EngineHelpers,
    private val installed: () -> InstalledApp,
    private val archive: ArchiveInspector,
    private val updateDir: File,
    private val audit: PlatformAudit,
    private val ioDispatcher: CoroutineDispatcher,
) {
    @Inject
    constructor(
        @ApplicationContext context: Context,
        helpers: EngineHelpers,
        audit: PlatformAudit,
        @IoDispatcher ioDispatcher: CoroutineDispatcher,
    ) : this(
        keys = { UpdateKeys.load(context) },
        helpers = helpers,
        installed = { installedApp(context) },
        archive = PackageManagerInspector(context),
        updateDir = File(context.filesDir, UPDATE_DIR),
        audit = audit,
        ioDispatcher = ioDispatcher,
    )

    private val mutex = Mutex()

    /** Runs every check against [source]; never throws apart from the cancellation of the caller. */
    suspend fun verify(source: UpdateSource): VerificationResult =
        mutex.withLock {
            audit.append(KIND, "check_started", mapOf("source" to source.label))
            val result = run(source)
            when (result) {
                is UpdateRefusal -> audit.append(KIND, "refused", mapOf("code" to result.code.id, "detail" to result.detail))
                is VerifiedUpdate ->
                    audit.append(KIND, "verified", mapOf("version" to result.release.version, "sha1" to result.sha1))
            }
            result
        }

    private suspend fun run(source: UpdateSource): VerificationResult {
        var downloaded: File? = null
        var done = false
        return try {
            val release = checkDescriptor(source)
            val file = File(updateDir, release.asset)
            downloaded = file
            val verified = checkAsset(source, release, file)
            done = true
            verified
        } catch (e: UpdateException) {
            UpdateRefusal(e.code, e.detail)
        } catch (e: IOException) {
            UpdateRefusal(UpdateRefusalCode.NETWORK, "${e.javaClass.simpleName}: ${e.message}")
        } finally {
            // A refusal, an error or a cancellation after the download began leaves nothing behind.
            if (!done) withContext(NonCancellable + ioDispatcher) { deleteQuietly(downloaded) }
        }
    }

    private suspend fun checkDescriptor(source: UpdateSource): DeclaredRelease {
        val key = keys().current() ?: refuse(UpdateRefusalCode.KEY_MISSING, "no public key for this build")
        val files = source.fetchDescriptor()
        val check = helpers.verifyMinisign(key, files.jsonBytes, files.minisigText)
        if (!check.valid) {
            val badKey = check.problem?.startsWith(KEY_INVALID) == true
            val code = if (badKey) UpdateRefusalCode.KEY_MISSING else UpdateRefusalCode.BAD_SIGNATURE
            refuse(code, check.problem ?: "the signature is not valid")
        }
        val release = DeclaredParser.parse(files.jsonBytes)
        val app = withContext(ioDispatcher) { installed() }
        refuseUnless(release.packageName == app.packageName, UpdateRefusalCode.WRONG_PACKAGE) {
            "${release.packageName} is not ${app.packageName}"
        }
        refuseUnless(release.versionCode > app.versionCode, UpdateRefusalCode.NOT_NEWER) {
            "${release.versionCode} is not greater than ${app.versionCode}"
        }
        return release
    }

    private suspend fun checkAsset(
        source: UpdateSource,
        release: DeclaredRelease,
        file: File,
    ): VerifiedUpdate {
        withContext(ioDispatcher) {
            updateDir.mkdirs()
            file.delete()
        }
        source.fetchAsset(release.asset, file)
        return withContext(ioDispatcher) {
            val (sha1, sha256) = Hasher.sha1AndSha256(file)
            refuseUnless(sha256 == release.sha256, UpdateRefusalCode.SHA256_MISMATCH) { "SHA-256 is $sha256" }
            refuseUnless(sha1 == release.sha1, UpdateRefusalCode.SHA1_MISMATCH) { "SHA-1 is $sha1" }
            checkArchive(release, file)
            VerifiedUpdate(release, file, sha1, sha256)
        }
    }

    private fun checkArchive(
        release: DeclaredRelease,
        file: File,
    ) {
        val info = archive.inspect(file.path) ?: refuse(UpdateRefusalCode.ARCHIVE_MISMATCH, "the APK cannot be parsed")
        val mismatch = UpdateRefusalCode.ARCHIVE_MISMATCH
        refuseUnless(info.packageName == release.packageName, mismatch) { "the APK package is ${info.packageName}" }
        refuseUnless(info.versionCode == release.versionCode, mismatch) { "the APK versionCode is ${info.versionCode}" }
        refuseUnless(info.versionName == release.version, mismatch) { "the APK versionName is ${info.versionName}" }
        val signers = UpdateRefusalCode.SIGNER_MISMATCH
        refuseUnless(info.signerSha256.isNotEmpty() && info.signerSha256 == installed().signerSha256, signers) {
            "the APK signers are not the installed ones"
        }
        refuseUnless(release.signerSha256 in info.signerSha256, signers) { "the declared signer is not among the APK signers" }
    }

    private fun deleteQuietly(file: File?) {
        if (file == null) return
        file.delete()
        File(file.path + ".part").delete()
    }

    companion object {
        /** Directory of the downloaded APK under `filesDir`. */
        const val UPDATE_DIR = "platform/update"

        /** The `kind` of the audit rows. */
        const val KIND = "update"

        private const val KEY_INVALID = "minisign_key_invalid"

        private fun installedApp(context: Context): InstalledApp {
            val info =
                context.packageManager.getPackageInfo(
                    context.packageName,
                    PackageManager.PackageInfoFlags.of(PackageManager.GET_SIGNING_CERTIFICATES.toLong()),
                )
            return InstalledApp(context.packageName, info.longVersionCode, signerSet(info.signingInfo?.apkContentsSigners))
        }

        private fun signerSet(signatures: Array<android.content.pm.Signature>?): Set<String> =
            signatures.orEmpty().map { Hasher.sha256(it.toByteArray()) }.toSet()
    }

    /** The real inspector: `PackageManager.getPackageArchiveInfo` with the signing certificates. */
    internal class PackageManagerInspector(
        private val context: Context,
    ) : ArchiveInspector {
        override fun inspect(path: String): ArchiveInfo? =
            try {
                val info =
                    context.packageManager.getPackageArchiveInfo(
                        path,
                        PackageManager.PackageInfoFlags.of(PackageManager.GET_SIGNING_CERTIFICATES.toLong()),
                    )
                info?.let {
                    ArchiveInfo(it.packageName.orEmpty(), it.longVersionCode, it.versionName, signerSet(it.signingInfo?.apkContentsSigners))
                }
            } catch (e: CancellationException) {
                throw e
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "UpdateVerifier: the APK cannot be inspected")
                null
            }
    }
}
