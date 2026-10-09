package dev.nuagemagique.pilipos.platform.export

import dev.nuagemagique.pilipos.platform.about.Hasher
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.security.DigestOutputStream
import java.security.MessageDigest

/** An export step that failed, carried up to [Exporter.run], which turns it into an [ExportOutcome.Failed]. */
internal class ExportException(
    val failure: ExportFailure,
    cause: Throwable? = null,
) : Exception("${failure.step.code}: ${failure.problem} ${failure.file.orEmpty()} ${failure.detail.orEmpty()}", cause)

/** An [ExportException] for the I/O error [cause] at [step] on [file]. */
internal fun exportFailure(
    cause: Throwable,
    step: ExportStep,
    file: String?,
    problem: ExportProblem,
): ExportException = ExportException(ExportFailure(step, file, problem, cause.message), cause)

/**
 * Writes one file of an export to [target], checks it, and reports the bytes written to [onBytes].
 *
 * The file is streamed in [CHUNK_BYTES] chunks through a SHA-256 [DigestOutputStream], synced, closed, opened
 * again through the target (a second reader's view) and hashed again: a different hash is a failure. Every
 * problem is an [ExportException] naming the step, the file and the reason; nothing is ever deleted.
 */
internal class ExportCopier(
    private val target: ExportTarget,
    private val onBytes: (path: String, bytes: Int) -> Unit,
) {
    /** Copies [file] into [dir] and returns its SHA-256; [writeStep] names the step in a failure. */
    fun copyAndVerify(
        dir: ExportDir,
        file: PlannedFile,
        writeStep: ExportStep = ExportStep.COPY,
    ): String {
        val problem = if (file.length >= FAT32_LIMIT) ExportProblem.FAT32 else ExportProblem.WRITE_FAILED
        val output = create(dir, file, writeStep, problem)
        val written = write(output, file, writeStep, problem)
        verify(output.file, file.path, written)
        return written
    }

    private fun create(
        dir: ExportDir,
        file: PlannedFile,
        step: ExportStep,
        problem: ExportProblem,
    ): ExportOutput =
        try {
            target.createFile(dir, file.path.substringAfterLast('/'), EXPORT_MIME)
        } catch (e: IOException) {
            throw exportFailure(e, step, file.path, problem)
        }

    private fun write(
        output: ExportOutput,
        file: PlannedFile,
        step: ExportStep,
        problem: ExportProblem,
    ): String {
        val digest = MessageDigest.getInstance("SHA-256")
        openSource(file, output, step).use { input ->
            DigestOutputStream(output, digest).use { out ->
                pump(input, out, file, step, problem)
                flushAndSync(out, output, file, step, problem)
            }
        }
        return Hasher.hex(digest.digest())
    }

    private fun openSource(
        file: PlannedFile,
        output: ExportOutput,
        step: ExportStep,
    ): InputStream =
        try {
            file.open()
        } catch (e: IOException) {
            output.close()
            throw exportFailure(e, step, file.path, ExportProblem.READ_FAILED)
        }

    private fun pump(
        input: InputStream,
        out: OutputStream,
        file: PlannedFile,
        step: ExportStep,
        problem: ExportProblem,
    ) {
        val buffer = ByteArray(CHUNK_BYTES)
        var remaining = file.length
        while (remaining > 0) {
            val read = readChunk(input, buffer, minOf(remaining, buffer.size.toLong()).toInt(), file.path, step)
            try {
                out.write(buffer, 0, read)
            } catch (e: IOException) {
                throw exportFailure(e, step, file.path, problem)
            }
            remaining -= read
            onBytes(file.path, read)
        }
    }

    private fun readChunk(
        input: InputStream,
        buffer: ByteArray,
        wanted: Int,
        path: String,
        step: ExportStep,
    ): Int {
        val read =
            try {
                input.read(buffer, 0, wanted)
            } catch (e: IOException) {
                throw exportFailure(e, step, path, ExportProblem.READ_FAILED)
            }
        return if (read > 0) read else throw exportFailure(IOException("the source ended early"), step, path, ExportProblem.READ_FAILED)
    }

    private fun flushAndSync(
        out: OutputStream,
        output: ExportOutput,
        file: PlannedFile,
        step: ExportStep,
        problem: ExportProblem,
    ) {
        try {
            out.flush()
            output.sync()
        } catch (e: IOException) {
            throw exportFailure(e, step, file.path, problem)
        }
    }

    private fun verify(
        written: ExportFile,
        path: String,
        expected: String,
    ) {
        val readBack =
            try {
                target.openForRead(written).use { Hasher.sha256(it) }
            } catch (e: IOException) {
                throw exportFailure(e, ExportStep.VERIFY, path, ExportProblem.READ_FAILED)
            }
        if (readBack != expected) {
            val detail = IOException("written $expected, read back $readBack")
            throw exportFailure(detail, ExportStep.VERIFY, path, ExportProblem.READ_BACK_MISMATCH)
        }
    }

    companion object {
        /** Size of the copy buffer: 1 MiB. */
        const val CHUNK_BYTES: Int = 1024 * 1024

        /** A file of this size or more cannot be written to a FAT32 volume: 4 GiB. */
        const val FAT32_LIMIT: Long = 4L * 1024L * 1024L * 1024L
    }
}
