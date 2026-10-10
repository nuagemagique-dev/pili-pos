package dev.nuagemagique.pilipos.platform.export

import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import dev.nuagemagique.pilipos.platform.PlatformOps
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.withContext
import timber.log.Timber
import uniffi.pos_ffi.UserRole
import java.io.IOException
import java.time.Instant
import java.time.format.DateTimeFormatter
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The SPF export (PD210, Q17): copies the files of the [ExportPlanner] to an [ExportTarget] with no staging
 * copy and checks every one of them by reading it back.
 *
 * Preconditions, checked again at every run (PD214): the engine is ready, the logged-in user is an
 * administrator, no order and no takeaway is open, and the target volume is still mounted. Then, in a folder
 * `pili-export-<yyyyMMdd-HHmmss>-<deviceId>` (Europe/Brussels) of the chosen folder: each file is hashed
 * (SHA-256) while it streams out in 1 MiB chunks, synced, read back through the target and hashed again; a
 * different hash fails the export (see [ExportCopier]). `MANIFEST.sha256` is written last (the same way), so
 * an export without it is incomplete.
 *
 * On any error the export stops, keeps what was written and reports the step, the file and the reason. It
 * deletes nothing itself, neither on the volume nor in the till (the engine's backup keeps its 7 newest copies). A write error on a file of 4 GiB or more is
 * reported as [ExportProblem.FAT32]. Each run is audited (kind `export`: started, done or failed; a refusal
 * is audited as `refused`), and runs inside [PlatformOps] as `export` so that the watchdog stays out of the way.
 */
@Singleton
class Exporter
    @Inject
    constructor(
        private val gateway: EngineGateway,
        private val planner: ExportPlanner,
        private val volumes: VolumeProbe,
        private val audit: PlatformAudit,
        private val ops: PlatformOps,
        @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
    ) {
        private val mutableProgress = MutableStateFlow(ExportProgress())

        /** Progress of the running export. */
        val progress: StateFlow<ExportProgress> = mutableProgress.asStateFlow()

        /**
         * Why the export cannot start for this user right now, or null: the engine is not ready, the user is not
         * an administrator, or an order or a takeaway is open.
         */
        suspend fun access(): ExportProblem? =
            try {
                gateway.call { engine ->
                    when {
                        engine.getCurrentUser()?.role != UserRole.ADMIN -> ExportProblem.NOT_MANAGER
                        engine.listOpenOrders().isNotEmpty() || engine.listOpenTakeaways().isNotEmpty() -> ExportProblem.ORDERS_OPEN
                        else -> null
                    }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: EngineUnavailableException) {
                Timber.w(e, "Exporter: the engine is not ready")
                ExportProblem.ENGINE_NOT_READY
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "Exporter: the engine did not answer the access check")
                ExportProblem.ENGINE_NOT_READY
            }

        /**
         * Runs one export into [target]; [folderName] is the name of the folder the manager chose (shown and
         * audited). Never throws apart from the cancellation of the caller.
         */
        suspend fun run(
            target: ExportTarget,
            folderName: String,
        ): ExportOutcome = ops.run(OPERATION) { execute(target, folderName) }

        private suspend fun execute(
            target: ExportTarget,
            folderName: String,
        ): ExportOutcome {
            mutableProgress.value = ExportProgress()
            val refusal = precheck(target)
            if (refusal != null) {
                audit.append(KIND, "refused", mapOf("problem" to refusal.name, "folder" to folderName))
                return ExportOutcome.Failed(ExportFailure(ExportStep.PRECHECK, null, refusal))
            }
            audit.append(KIND, "started", mapOf("folder" to folderName))
            return try {
                val result = withContext(ioDispatcher) { export(target, folderName) }
                val detail =
                    mapOf("files" to "${result.files}", "bytes" to "${result.bytes}", "folder" to "$folderName/${result.exportFolder}")
                audit.append(KIND, "done", detail)
                ExportOutcome.Done(result)
            } catch (e: CancellationException) {
                withContext(NonCancellable) { audit.append(KIND, "failed", mapOf("step" to "cancelled", "reason" to "cancelled")) }
                throw e
            } catch (e: ExportException) {
                failed(e.failure)
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.e(e, "Exporter: unexpected failure")
                failed(ExportFailure(ExportStep.COPY, null, ExportProblem.OTHER, e.message))
            }
        }

        private suspend fun failed(failure: ExportFailure): ExportOutcome {
            val detail =
                mapOf(
                    "step" to failure.step.code,
                    "file" to failure.file.orEmpty(),
                    "problem" to failure.problem.name,
                    "reason" to failure.detail.orEmpty(),
                )
            audit.append(KIND, "failed", detail)
            return ExportOutcome.Failed(failure)
        }

        private suspend fun precheck(target: ExportTarget): ExportProblem? {
            val uuid = target.volumeUuid
            val gone = uuid != null && volumes.mounted().none { it.uuid.equals(uuid, ignoreCase = true) }
            return if (gone) ExportProblem.NO_VOLUME else access()
        }

        private suspend fun export(
            target: ExportTarget,
            folderName: String,
        ): ExportResult {
            val plan = plan()
            val exportFolder = "pili-export-${STAMP.format(Instant.ofEpochMilli(plan.exportedAtMillis))}-${plan.deviceId}"
            mutableProgress.value = ExportProgress(filesTotal = plan.files.size + 1, bytesTotal = plan.totalBytes)
            val dirs = createDirectories(target, exportFolder, plan.directories)
            val copier =
                ExportCopier(target) { path, bytes -> mutableProgress.update { it.copy(bytesDone = it.bytesDone + bytes, current = path) } }
            val hashes = linkedMapOf<String, String>()
            for (file in plan.files) {
                hashes[file.path] = copier.copyAndVerify(dirs.getValue(file.path.substringBeforeLast('/', "")), file)
                mutableProgress.update { it.copy(filesDone = it.filesDone + 1) }
            }
            val manifest = Manifest.render(hashes).toByteArray(Charsets.UTF_8)
            mutableProgress.update { it.copy(bytesTotal = it.bytesTotal + manifest.size) }
            val manifestFile = PlannedFile(Manifest.NAME, manifest.size.toLong(), ExportSource.Bytes(manifest))
            val manifestSha = copier.copyAndVerify(dirs.getValue(""), manifestFile, ExportStep.MANIFEST)
            mutableProgress.update { it.copy(filesDone = it.filesDone + 1) }
            return ExportResult(folderName, exportFolder, plan.files.size + 1, plan.totalBytes + manifest.size, manifestSha)
        }

        private suspend fun plan(): ExportPlan =
            try {
                planner.plan()
            } catch (e: CancellationException) {
                throw e
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                throw exportFailure(e, ExportStep.PLAN, null, ExportProblem.PLAN_FAILED)
            }

        private fun createDirectories(
            target: ExportTarget,
            exportFolder: String,
            names: List<String>,
        ): Map<String, ExportDir> {
            val dirs = linkedMapOf<String, ExportDir>()
            try {
                val top = target.createDirectory(target.root, exportFolder)
                dirs[""] = top
                for (name in names) dirs[name] = target.createDirectory(top, name)
            } catch (e: IOException) {
                throw exportFailure(e, ExportStep.CREATE_DIR, null, ExportProblem.WRITE_FAILED)
            }
            return dirs
        }

        companion object {
            /** Operation name in [PlatformOps]. */
            const val OPERATION = "export"

            /** Audit kind of every row. */
            const val KIND = "export"

            private val STAMP: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyyMMdd-HHmmss").withZone(PlatformAudit.ZONE)
        }
    }
