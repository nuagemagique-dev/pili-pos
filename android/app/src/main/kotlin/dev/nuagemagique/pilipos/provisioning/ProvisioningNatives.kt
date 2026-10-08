package dev.nuagemagique.pilipos.provisioning

import dev.nuagemagique.pilipos.di.IoDispatcher
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import uniffi.pos_ffi.BundleCheck
import javax.inject.Inject

/**
 * The engine helpers provisioning needs, behind an interface so tests use a fake instead of the native
 * library. All calls take absolute paths and run on the I/O dispatcher.
 */
interface ProvisioningNatives {
    /** Verifies the signed bundle in [dir] against [publicKey]; failures are values (PD61). */
    suspend fun verifyBundle(
        dir: String,
        publicKey: String,
    ): BundleCheck

    /** The problems the engine finds in `pos.json`; empty when it is usable. */
    suspend fun checkConfigBundle(configBundle: String): List<String>

    /** Creates or migrates `<dataDir>/pos.db`, loading [seedSqlPath] when given. */
    suspend fun installInitDb(
        dataDir: String,
        seedSqlPath: String?,
    )
}

/** The UniFFI implementation of [ProvisioningNatives]. */
class UniffiProvisioningNatives
    @Inject
    constructor(
        @param:IoDispatcher private val io: CoroutineDispatcher,
    ) : ProvisioningNatives {
        override suspend fun verifyBundle(
            dir: String,
            publicKey: String,
        ): BundleCheck = withContext(io) { uniffi.pos_ffi.verifyBundle(dir, publicKey) }

        override suspend fun checkConfigBundle(configBundle: String): List<String> =
            withContext(io) { uniffi.pos_ffi.checkConfigBundle(configBundle) }

        override suspend fun installInitDb(
            dataDir: String,
            seedSqlPath: String?,
        ) {
            withContext(io) { uniffi.pos_ffi.installInitDb(dataDir, seedSqlPath) }
        }
    }
