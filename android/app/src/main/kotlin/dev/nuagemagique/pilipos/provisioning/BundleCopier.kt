package dev.nuagemagique.pilipos.provisioning

import timber.log.Timber
import java.io.File
import java.io.IOException
import java.nio.file.Files
import java.nio.file.StandardOpenOption
import javax.inject.Inject

/** Thrown inside an import to stop it with [problem]; never escapes [BundleImporter]. */
internal class ImportAbort(
    val problem: ProvisioningProblem,
    cause: Throwable? = null,
) : Exception(problem.code, cause)

/** Stops the import with [problem]. */
internal fun abort(
    problem: ProvisioningProblem,
    cause: Throwable? = null,
): Nothing = throw ImportAbort(problem, cause)

/**
 * Copies the files of a [BundleSource] into a staging directory under strict limits: regular files only
 * (no symbolic link), path segments of `[A-Za-z0-9._-]`, depth at most 4, at most 64 files and at most
 * 32 MiB in total. Nothing is trusted yet: the copy only keeps hostile input from reaching the verifier.
 */
class BundleCopier
    @Inject
    constructor(
        private val treeReader: TreeReader,
    ) {
        private class Totals {
            var files = 0
            var bytes = 0L
        }

        /** Copies [source] into the new directory [staging]; stops with an [ImportAbort] on a refusal. */
        fun copy(
            source: BundleSource,
            staging: File,
        ) {
            val root =
                when (source) {
                    is BundleSource.Directory -> FileNode(source.dir)
                    is BundleSource.TreeUri -> treeReader.root(source.uri)
                }
            if (root == null || !root.isDirectory) {
                abort(ProvisioningProblem.of("copy_failed", "detail" to "the source folder cannot be read"))
            }
            try {
                staging.mkdirs()
                copyChildren(root, staging, emptyList(), Totals())
            } catch (e: IOException) {
                abort(ProvisioningProblem.of("copy_failed", "detail" to e.javaClass.simpleName), e)
            } catch (e: SecurityException) {
                abort(ProvisioningProblem.of("copy_failed", "detail" to e.javaClass.simpleName), e)
            }
            Timber.i("provisioning: source copied to staging")
        }

        private fun copyChildren(
            dir: TreeNode,
            target: File,
            parents: List<String>,
            totals: Totals,
        ) {
            for (node in dir.children()) {
                val segments = parents + node.name
                val rel = segments.joinToString("/")
                if (!isPlainName(node.name) || segments.size > MAX_DEPTH) {
                    abort(ProvisioningProblem.of("bundle_path_invalid", "path" to rel))
                }
                when {
                    node.isSymlink -> abort(ProvisioningProblem.of("symlink", "path" to rel))
                    node.isDirectory -> {
                        val sub = File(target, node.name)
                        if (!sub.mkdir()) throw IOException("cannot create a directory")
                        copyChildren(node, sub, segments, totals)
                    }
                    node.isFile -> copyFile(node, File(target, node.name), totals)
                    else -> abort(ProvisioningProblem.of("bundle_path_invalid", "path" to rel))
                }
            }
        }

        private fun isPlainName(name: String): Boolean = SEGMENT.matches(name) && name != "." && name != ".."

        private fun copyFile(
            node: TreeNode,
            target: File,
            totals: Totals,
        ) {
            totals.files += 1
            if (totals.files > MAX_FILES || (node.length > 0 && totals.bytes + node.length > MAX_BYTES)) {
                abort(ProvisioningProblem.of("too_large"))
            }
            val input = node.open() ?: throw IOException("cannot open a file")
            input.use { src ->
                Files.newOutputStream(target.toPath(), StandardOpenOption.CREATE_NEW, StandardOpenOption.WRITE).use { out ->
                    pump(src, out, totals)
                }
            }
        }

        private fun pump(
            src: java.io.InputStream,
            out: java.io.OutputStream,
            totals: Totals,
        ) {
            val buffer = ByteArray(BUFFER)
            var n = src.read(buffer)
            while (n >= 0) {
                totals.bytes += n
                if (totals.bytes > MAX_BYTES) abort(ProvisioningProblem.of("too_large"))
                out.write(buffer, 0, n)
                n = src.read(buffer)
            }
        }

        private companion object {
            const val MAX_DEPTH = 4
            const val MAX_FILES = 64
            const val MAX_BYTES = 32L * 1024 * 1024
            const val BUFFER = 8 * 1024
            val SEGMENT = Regex("[A-Za-z0-9._-]+")
        }
    }
