package dev.nuagemagique.pilipos.shell

import dev.nuagemagique.pilipos.engine.CloseResult
import dev.nuagemagique.pilipos.engine.EngineHolder
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.provisioning.BundleImporter
import dev.nuagemagique.pilipos.provisioning.BundleSource
import dev.nuagemagique.pilipos.provisioning.ImportResult
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.withContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Runs a bundle import around the engine: close, import, open. The importer never runs while the engine is
 * open (its lock and database are replaced), and the engine is opened again whatever the outcome: with the
 * new bundle after a success, with the previous one (if any) after a failure.
 */
@Singleton
class ProvisioningCoordinator
    @Inject
    constructor(
        private val holder: EngineHolder,
        private val importer: BundleImporter,
    ) {
        /**
         * Imports [source]. When the engine refuses to close ([CloseResult.StillOpen]) nothing is imported and the
         * result is a failure with the problem `engine_open`.
         */
        suspend fun import(source: BundleSource): ImportResult {
            val wasReady = holder.state.value is EngineState.Ready
            val closed = holder.close()
            if (closed is CloseResult.StillOpen) {
                Timber.tag(TAG).w("import refused, the engine is still open: %s", closed.detail)
                return ImportResult.Failure(ProvisioningProblem.of("engine_open"))
            }
            val result =
                try {
                    importer.import(source)
                } finally {
                    // The engine must come back even if the caller is cancelled while the import runs.
                    withContext(NonCancellable) { holder.open() }
                }
            Timber.tag(TAG).i("import %s, engine reopened (wasReady=%s)", result::class.simpleName, wasReady)
            return result
        }

        private companion object {
            const val TAG = "PiliShell"
        }
    }
