package dev.nuagemagique.pilipos.provisioning

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import dagger.hilt.android.qualifiers.ApplicationContext
import java.io.File
import java.io.InputStream
import java.nio.file.Files
import javax.inject.Inject

/** Where a provisioning bundle is read from. */
sealed interface BundleSource {
    /** A folder the manager picked with the system folder picker (Storage Access Framework tree). */
    data class TreeUri(
        val uri: Uri,
    ) : BundleSource

    /** A plain directory (the debug inbox, tests). */
    data class Directory(
        val dir: File,
    ) : BundleSource
}

/** A file or folder of a [BundleSource], as the importer sees it. */
interface TreeNode {
    val name: String
    val isDirectory: Boolean
    val isFile: Boolean
    val isSymlink: Boolean

    /** Size in bytes, -1 when unknown. */
    val length: Long

    fun children(): List<TreeNode>

    /** The content of a file, or null when it cannot be opened. */
    fun open(): InputStream?
}

/** Opens the root of a Storage Access Framework tree; kept as a seam so the importer is testable. */
interface TreeReader {
    /** The root folder of [uri], or null when the tree is not readable. */
    fun root(uri: Uri): TreeNode?
}

/** [TreeReader] over `DocumentFile.fromTreeUri`. */
class SafTreeReader
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
    ) : TreeReader {
        override fun root(uri: Uri): TreeNode? = DocumentFile.fromTreeUri(context, uri)?.let { DocumentNode(context, it) }

        private class DocumentNode(
            private val context: Context,
            private val doc: DocumentFile,
        ) : TreeNode {
            override val name: String get() = doc.name.orEmpty()
            override val isDirectory: Boolean get() = doc.isDirectory
            override val isFile: Boolean get() = doc.isFile && !doc.isVirtual
            override val isSymlink: Boolean get() = false
            override val length: Long get() = doc.length()

            override fun children(): List<TreeNode> = doc.listFiles().map { DocumentNode(context, it) }

            override fun open(): InputStream? = context.contentResolver.openInputStream(doc.uri)
        }
    }

/** A [TreeNode] over a [File]; symbolic links are reported, never followed. */
internal class FileNode(
    private val file: File,
) : TreeNode {
    override val name: String get() = file.name
    override val isSymlink: Boolean get() = Files.isSymbolicLink(file.toPath())
    override val isDirectory: Boolean get() = !isSymlink && file.isDirectory
    override val isFile: Boolean get() = !isSymlink && file.isFile
    override val length: Long get() = file.length()

    override fun children(): List<TreeNode> = file.listFiles().orEmpty().sortedBy { it.name }.map { FileNode(it) }

    override fun open(): InputStream = file.inputStream()
}
