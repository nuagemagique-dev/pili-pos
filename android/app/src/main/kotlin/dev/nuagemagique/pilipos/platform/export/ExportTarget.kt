package dev.nuagemagique.pilipos.platform.export

import android.content.ContentResolver
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import java.io.File
import java.io.FileInputStream
import java.io.FileNotFoundException
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream

/** A directory of an [ExportTarget]; only the target that made it can use it. */
interface ExportDir

/** A file of an [ExportTarget]; only the target that made it can use it. */
interface ExportFile

/** An output stream whose bytes can be forced to the medium, and that knows the file it writes. */
abstract class ExportOutput : OutputStream() {
    /** The file this stream writes. */
    abstract val file: ExportFile

    /** Forces what was written to the medium (`fsync`). */
    abstract fun sync()
}

/**
 * Where an export is written. All export logic stays above this interface; the two implementations are
 * [SafTarget] (a persisted tree of the Storage Access Framework) and [FileTarget] (plain files).
 *
 * Nothing in this interface deletes anything.
 */
interface ExportTarget {
    /** The folder the manager chose, parent of the export folder. */
    val root: ExportDir

    /** The uuid of the volume the target lives on, null when it is not tied to a removable volume. */
    val volumeUuid: String?

    /** Creates the directory [name] in [parent]. @throws IOException when it cannot be created */
    fun createDirectory(
        parent: ExportDir,
        name: String,
    ): ExportDir

    /** Creates the file [name] in [dir] and opens it for writing. @throws IOException when it cannot be created */
    fun createFile(
        dir: ExportDir,
        name: String,
        mime: String,
    ): ExportOutput

    /** Opens [file] for reading, as a second reader would see it. @throws IOException when it cannot be opened */
    fun openForRead(file: ExportFile): InputStream
}

/** MIME type of every exported file: the provider then never adds an extension to a name such as `MANIFEST.sha256`. */
const val EXPORT_MIME: String = "application/octet-stream"

private class SafDir(
    val uri: Uri,
) : ExportDir

private class SafFile(
    val uri: Uri,
) : ExportFile

private class SafOutput(
    override val file: ExportFile,
    private val descriptor: ParcelFileDescriptor,
) : ExportOutput() {
    private val stream = FileOutputStream(descriptor.fileDescriptor)

    override fun write(b: Int) = stream.write(b)

    override fun write(
        b: ByteArray,
        off: Int,
        len: Int,
    ) = stream.write(b, off, len)

    override fun flush() = stream.flush()

    override fun sync() {
        stream.flush()
        descriptor.fileDescriptor.sync()
    }

    override fun close() {
        try {
            stream.close()
        } finally {
            descriptor.close()
        }
    }
}

/**
 * An export into a persisted SAF [tree] (S4 section 4): directories and files are made with
 * `DocumentsContract.createDocument`, written through `openFileDescriptor("w")` and synced with the descriptor,
 * and read back through `ContentResolver.openInputStream`.
 */
class SafTarget(
    private val resolver: ContentResolver,
    tree: Uri,
    override val volumeUuid: String?,
) : ExportTarget {
    override val root: ExportDir = SafDir(DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree)))

    override fun createDirectory(
        parent: ExportDir,
        name: String,
    ): ExportDir = SafDir(create((parent as SafDir).uri, DocumentsContract.Document.MIME_TYPE_DIR, name))

    override fun createFile(
        dir: ExportDir,
        name: String,
        mime: String,
    ): ExportOutput {
        val uri = create((dir as SafDir).uri, mime, name)
        val descriptor = resolver.openFileDescriptor(uri, "w") ?: throw IOException("no descriptor for $name")
        return SafOutput(SafFile(uri), descriptor)
    }

    override fun openForRead(file: ExportFile): InputStream =
        resolver.openInputStream((file as SafFile).uri) ?: throw FileNotFoundException("cannot read ${file.uri}")

    private fun create(
        parent: Uri,
        mime: String,
        name: String,
    ): Uri {
        val created =
            try {
                DocumentsContract.createDocument(resolver, parent, mime, name)
            } catch (
                @Suppress("TooGenericExceptionCaught") e: RuntimeException,
            ) {
                // The provider throws an IllegalStateException or SecurityException when the volume is gone.
                throw IOException("cannot create $name: ${e.message}", e)
            }
        return created ?: throw IOException("cannot create $name")
    }
}

private class FileDir(
    val dir: File,
) : ExportDir

private class FileEntry(
    val file: File,
) : ExportFile

private class FileOutput(
    override val file: ExportFile,
    private val stream: FileOutputStream,
) : ExportOutput() {
    override fun write(b: Int) = stream.write(b)

    override fun write(
        b: ByteArray,
        off: Int,
        len: Int,
    ) = stream.write(b, off, len)

    override fun flush() = stream.flush()

    override fun sync() {
        stream.flush()
        stream.fd.sync()
    }

    override fun close() = stream.close()
}

/**
 * An export into a plain directory [rootDir]: used by unit tests and by the device test on the app-specific
 * directory of the removable volume. A name that exists is an error: nothing is overwritten.
 */
class FileTarget(
    private val rootDir: File,
    override val volumeUuid: String? = null,
) : ExportTarget {
    override val root: ExportDir = FileDir(rootDir)

    override fun createDirectory(
        parent: ExportDir,
        name: String,
    ): ExportDir {
        val dir = File((parent as FileDir).dir, name)
        if (dir.exists() || !dir.mkdirs()) throw IOException("cannot create directory $dir")
        return FileDir(dir)
    }

    override fun createFile(
        dir: ExportDir,
        name: String,
        mime: String,
    ): ExportOutput {
        val file = File((dir as FileDir).dir, name)
        if (file.exists()) throw IOException("$file exists")
        return FileOutput(FileEntry(file), FileOutputStream(file))
    }

    override fun openForRead(file: ExportFile): InputStream = FileInputStream((file as FileEntry).file)
}
