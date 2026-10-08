package dev.nuagemagique.pilipos.provisioning

import dev.nuagemagique.pilipos.di.IoDispatcher
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.withContext
import timber.log.Timber
import uniffi.pos_ffi.BundleCheck
import uniffi.pos_ffi.PosException
import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest
import java.time.Clock
import javax.inject.Inject
import javax.inject.Singleton

/** Outcome of [BundleImporter.import]. */
sealed interface ImportResult {
    /** The bundle is installed and [record] binds it to this device. */
    data class Success(
        val record: ProvisioningRecord,
    ) : ImportResult

    /** Nothing changed: the previous bundle (if any) is still installed. */
    data class Failure(
        val problem: ProvisioningProblem,
    ) : ImportResult
}

/**
 * Imports a minisign-signed provisioning bundle (PD108) into app-private storage.
 *
 * The source is copied into a fresh staging directory under strict limits (regular files only, plain
 * names, bounded depth, count and size), verified there against the build-time public key, checked by the
 * engine (`pos.json`), and only then swapped in place of the current bundle. A failure at any step keeps
 * the previous bundle and leaves no staging directory behind.
 *
 * The engine must be closed while an import runs: the caller (P4-T11) coordinates that. A second import
 * started while one runs is refused with the problem `busy`.
 */
@Singleton
class BundleImporter
    @Suppress("LongParameterList")
    @Inject
    constructor(
        private val paths: ProvisioningPaths,
        private val owner: DeviceOwner,
        private val deviceIds: DeviceIdReader,
        private val natives: ProvisioningNatives,
        private val records: ProvisioningRecordStore,
        private val copier: BundleCopier,
        @param:ProvisioningPublicKey private val publicKey: String,
        @param:ProvisioningClock private val clock: Clock,
        @param:IoDispatcher private val io: CoroutineDispatcher,
    ) {
        private val mutex = Mutex()

        /** Imports the bundle of [source]; never throws except for cancellation. */
        suspend fun import(source: BundleSource): ImportResult {
            if (!mutex.tryLock()) {
                Timber.w("provisioning: import refused, another one is running")
                return ImportResult.Failure(ProvisioningProblem.of("busy"))
            }
            try {
                return withContext(io) { run(source) }
            } finally {
                mutex.unlock()
            }
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun run(source: BundleSource): ImportResult {
            val staging = paths.stagingDir(clock.millis())
            return try {
                ImportResult.Success(importInto(source, staging))
            } catch (abort: ImportAbort) {
                Timber.w("provisioning: import refused (%s)", abort.problem.code)
                ImportResult.Failure(abort.problem)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.e("provisioning: import failed unexpectedly (%s)", e.javaClass.simpleName)
                ImportResult.Failure(ProvisioningProblem.unexpected("${e.javaClass.simpleName}: ${e.message.orEmpty()}"))
            } finally {
                withContext(NonCancellable) { staging.deleteRecursively() }
            }
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun importInto(
            source: BundleSource,
            staging: File,
        ): ProvisioningRecord {
            val deviceId = checkPreconditions()
            Timber.i("provisioning: import started")
            recoverInterruptedSwap()
            staging.deleteRecursively()
            copier.copy(source, staging)
            val check = verify(staging)
            checkConfig(staging)
            val dbExisted = paths.databaseFile.exists()
            try {
                installDatabase(staging, check, dbExisted)
                return swap(staging, check, deviceId)
            } catch (e: Throwable) {
                if (!dbExisted) removeDatabaseFiles()
                throw e
            }
        }

        private fun checkPreconditions(): String {
            if (!owner.isDeviceOwner()) abort(ProvisioningProblem.of("not_device_owner"))
            if (publicKey.isBlank()) abort(ProvisioningProblem.of("key_missing"))
            return when (val id = deviceIds.read()) {
                is DeviceIdResult.Available -> id.serial
                is DeviceIdResult.Unavailable -> abort(ProvisioningProblem.of("device_id_unavailable", "detail" to id.detail))
            }
        }

        /** A crash between the two renames of a swap leaves no bundle and a bundle-old: put it back. */
        private fun recoverInterruptedSwap() {
            paths.staleStagingDirs().forEach { it.deleteRecursively() }
            val olds = paths.staleOldBundles().sortedByDescending { it.name }
            if (!paths.bundleDir.exists() && olds.isNotEmpty()) {
                move(olds.first(), paths.bundleDir)
                Timber.w("provisioning: restored the bundle of an interrupted swap")
            }
            olds.filter { it.exists() }.forEach { it.deleteRecursively() }
        }

        private suspend fun verify(staging: File): BundleCheck {
            val check =
                try {
                    natives.verifyBundle(staging.absolutePath, publicKey)
                } catch (e: PosException) {
                    abort(ProvisioningProblem.unexpected("verifyBundle: ${e.javaClass.simpleName}"), e)
                }
            if (!check.valid) {
                abort(check.problem?.let { ProvisioningProblem.parse(it) } ?: ProvisioningProblem.unexpected("invalid bundle"))
            }
            if (check.files.none { it.path == POS_JSON }) {
                abort(ProvisioningProblem.of("bundle_file_missing", "path" to POS_JSON))
            }
            Timber.i("provisioning: bundle signature and files verified (%d files)", check.files.size)
            return check
        }

        private suspend fun checkConfig(staging: File) {
            val problems =
                try {
                    natives.checkConfigBundle(File(staging, POS_JSON).absolutePath)
                } catch (e: PosException) {
                    abort(ProvisioningProblem.unexpected("checkConfigBundle: ${e.javaClass.simpleName}"), e)
                }
            if (problems.isNotEmpty()) {
                Timber.w("provisioning: pos.json has %d problem(s)", problems.size)
                abort(ProvisioningProblem.of("config_invalid", "detail" to problems.joinToString("; ")))
            }
        }

        /** Creates the database with the bundle seed only when none exists; an existing one is migrated by the engine. */
        private suspend fun installDatabase(
            staging: File,
            check: BundleCheck,
            dbExisted: Boolean,
        ) {
            val hasSeed = check.files.any { it.path == SEED_SQL }
            if (dbExisted) {
                if (hasSeed) Timber.i("provisioning: seed.sql ignored, the database already exists")
                return
            }
            paths.dataDir.mkdirs()
            val seed = if (hasSeed) File(staging, SEED_SQL).absolutePath else null
            try {
                natives.installInitDb(paths.dataDir.absolutePath, seed)
            } catch (e: PosException.Internal) {
                if (e.detail.contains("engine already open")) abort(ProvisioningProblem.of("engine_open"), e)
                abort(ProvisioningProblem.unexpected("installInitDb: ${e.detail}"), e)
            } catch (e: PosException) {
                abort(ProvisioningProblem.unexpected("installInitDb: ${e.javaClass.simpleName}"), e)
            }
            Timber.i("provisioning: database created (seed: %s)", hasSeed)
        }

        @Suppress("TooGenericExceptionCaught")
        private fun swap(
            staging: File,
            check: BundleCheck,
            deviceId: String,
        ): ProvisioningRecord {
            val record =
                ProvisioningRecord(
                    deviceId = deviceId,
                    manifestSha256 = sha256(File(staging, MANIFEST).readBytes()),
                    trustedComment = check.trustedComment.orEmpty(),
                    importedAt = clock.instant().toString(),
                )
            val bundle = paths.bundleDir
            val old = paths.oldBundleDir(clock.millis())
            old.deleteRecursively()
            val hadBundle = bundle.exists()
            if (hadBundle) move(bundle, old)
            try {
                move(staging, bundle)
                records.write(record)
            } catch (e: Exception) {
                Timber.e("provisioning: swap failed (%s), restoring the previous bundle", e.javaClass.simpleName)
                bundle.deleteRecursively()
                if (hadBundle) move(old, bundle)
                abort(ProvisioningProblem.of("copy_failed", "detail" to e.javaClass.simpleName), e)
            }
            old.deleteRecursively()
            Timber.i("provisioning: bundle installed")
            return record
        }

        private fun removeDatabaseFiles() {
            listOf("pos.db", "pos.db-wal", "pos.db-shm", "pos.db-journal").forEach { File(paths.dataDir, it).delete() }
        }

        private companion object {
            const val MANIFEST = "MANIFEST.json"
            const val POS_JSON = "pos.json"
            const val SEED_SQL = "seed.sql"
        }
    }

private fun move(
    from: File,
    to: File,
) {
    Files.move(from.toPath(), to.toPath(), StandardCopyOption.ATOMIC_MOVE)
}

private fun sha256(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
