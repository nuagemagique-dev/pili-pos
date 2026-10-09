package dev.nuagemagique.pilipos.platform.export

import androidx.annotation.StringRes
import dev.nuagemagique.pilipos.R
import java.io.File
import java.io.InputStream

/** Why an export or a folder choice was refused or failed; each value has its text in `strings_plat_export.xml`. */
enum class ExportProblem(
    @StringRes val textRes: Int,
) {
    /** The chosen tree is the root of the volume. */
    ROOT(R.string.plat_export_err_root),

    /** The chosen tree is on another volume than the one the screen works with. */
    WRONG_VOLUME(R.string.plat_export_err_wrong_volume),

    /** The logged-in user is not an administrator. */
    NOT_MANAGER(R.string.plat_export_not_manager),

    /** An order or a takeaway is open. */
    ORDERS_OPEN(R.string.plat_export_err_open_orders),

    /** The engine is not ready or does not answer. */
    ENGINE_NOT_READY(R.string.plat_export_err_engine),

    /** No removable volume is mounted, or the target volume left. */
    NO_VOLUME(R.string.plat_export_err_no_volume),

    /** No folder was chosen on the volume. */
    NO_FOLDER(R.string.plat_export_err_no_folder),

    /** The fresh copy of the database or the list of files could not be made. */
    PLAN_FAILED(R.string.plat_export_err_plan),

    /** A file could not be written (or the folder could not be created). */
    WRITE_FAILED(R.string.plat_export_err_write),

    /** A file of 4 GiB or more could not be written: the stick is probably FAT32. */
    FAT32(R.string.plat_export_err_fat32_hint),

    /** The file read back from the volume has another SHA-256 than the one written. */
    READ_BACK_MISMATCH(R.string.plat_export_err_mismatch),

    /** The file could not be read back or its source could not be read. */
    READ_FAILED(R.string.plat_export_err_read),

    /** Anything else. */
    OTHER(R.string.plat_export_err_other),
}

/** The step an export was in. */
enum class ExportStep(
    @StringRes val textRes: Int,
    val code: String,
) {
    PRECHECK(R.string.plat_export_step_precheck, "precheck"),
    PLAN(R.string.plat_export_step_plan, "plan"),
    CREATE_DIR(R.string.plat_export_step_create_dir, "create_dir"),
    COPY(R.string.plat_export_step_copy, "copy"),
    VERIFY(R.string.plat_export_step_verify, "verify"),
    MANIFEST(R.string.plat_export_step_manifest, "manifest"),
}

/**
 * A failed or refused export: the [step], the [file] (relative path in the export folder, null when none),
 * the [problem] and the technical [detail] (an exception message, never shown as the only text).
 */
data class ExportFailure(
    val step: ExportStep,
    val file: String?,
    val problem: ExportProblem,
    val detail: String? = null,
)

/** What a finished export wrote. */
data class ExportResult(
    /** Name of the folder the manager chose on the volume. */
    val chosenFolder: String,
    /** The `pili-export-...` folder created in it. */
    val exportFolder: String,
    /** Number of files written, `MANIFEST.sha256` included. */
    val files: Int,
    /** Total bytes written, `MANIFEST.sha256` included. */
    val bytes: Long,
    /** SHA-256 of `MANIFEST.sha256`. */
    val manifestSha256: String,
)

/** How an export ended. */
sealed interface ExportOutcome {
    data class Done(
        val result: ExportResult,
    ) : ExportOutcome

    data class Failed(
        val failure: ExportFailure,
    ) : ExportOutcome
}

/** Progress of a running export: bytes are counted as they are written to the volume. */
data class ExportProgress(
    val filesDone: Int = 0,
    val filesTotal: Int = 0,
    val bytesDone: Long = 0,
    val bytesTotal: Long = 0,
    val current: String? = null,
)

/** Where the bytes of a planned file come from. */
sealed interface ExportSource {
    /** Content built in memory. */
    class Bytes(
        val bytes: ByteArray,
    ) : ExportSource

    /** A file of the till, copied up to the size it had when the plan was made. */
    class Disk(
        val file: File,
    ) : ExportSource
}

/** A file of the export: its [path] relative to the export folder, its [length] and its [source]. */
class PlannedFile(
    val path: String,
    val length: Long,
    val source: ExportSource,
) {
    /** Opens the source. */
    fun open(): InputStream =
        when (source) {
            is ExportSource.Bytes -> source.bytes.inputStream()
            is ExportSource.Disk -> source.file.inputStream()
        }
}

/**
 * What an export writes, in this order, before `MANIFEST.sha256`: [directories] are created first.
 *
 * @property deviceId the serial used in the folder name
 * @property exportedAtMillis the time of the plan, used for the folder name and the read-me
 */
class ExportPlan(
    val deviceId: String,
    val exportedAtMillis: Long,
    val directories: List<String>,
    val files: List<PlannedFile>,
) {
    /** Sum of the lengths of [files]. */
    val totalBytes: Long get() = files.sumOf { it.length }
}
