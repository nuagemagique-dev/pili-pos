package dev.nuagemagique.pilipos.platform.update

import android.content.ContentResolver
import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.io.InputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import kotlin.coroutines.coroutineContext
import kotlinx.coroutines.ensureActive as ensureContextActive

/** The files of the folder chosen on the stick (a seam: production reads a SAF tree, tests a directory). */
interface UsbFolder {
    /** The size of the file called [name], or null when the folder has no such file. */
    fun length(name: String): Long?

    /** Opens the file called [name]; null when the folder has no such file. */
    fun open(name: String): InputStream?
}

/** A SAF folder: the files are found by display name and read through the content resolver. */
class DocumentUsbFolder(
    private val resolver: ContentResolver,
    private val folder: DocumentFile,
) : UsbFolder {
    private fun find(name: String): DocumentFile? = folder.findFile(name)?.takeIf { it.isFile }

    override fun length(name: String): Long? = find(name)?.length()

    override fun open(name: String): InputStream? = find(name)?.let { resolver.openInputStream(it.uri) }
}

/**
 * The same three files as the registry (`declared.json`, `declared.json.minisig`, the asset) from a folder
 * picked through the Storage Access Framework (Q8). A file that is absent is `usb_missing_file`.
 */
class UsbSource(
    private val folder: UsbFolder,
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO,
) : UpdateSource {
    /** The folder at [treeUri], for example the result of `ACTION_OPEN_DOCUMENT_TREE`. */
    constructor(context: Context, treeUri: Uri, ioDispatcher: CoroutineDispatcher = Dispatchers.IO) : this(
        DocumentUsbFolder(
            context.contentResolver,
            DocumentFile.fromTreeUri(context, treeUri)
                ?: refuse(UpdateRefusalCode.USB_MISSING_FILE, "the chosen folder cannot be read"),
        ),
        ioDispatcher,
    )

    override val label: String = "usb"

    override suspend fun fetchDescriptor(): DescriptorFiles {
        val ctx = coroutineContext
        return withContext(ioDispatcher) {
            val json = readSmall(UpdateSource.DESCRIPTOR_NAME)
            val signature = readSmall(UpdateSource.SIGNATURE_NAME)
            ctx.ensureContextActive()
            DescriptorFiles(json, String(signature, Charsets.UTF_8))
        }
    }

    override suspend fun fetchAsset(
        name: String,
        dest: File,
        maxBytes: Long,
    ) {
        val ctx = coroutineContext
        val partial = File(dest.parentFile, dest.name + ".part")
        withContext(ioDispatcher) {
            try {
                val size = folder.length(name) ?: missing(name)
                refuseUnless(size <= maxBytes, UpdateRefusalCode.TOO_LARGE) { "$name is $size bytes" }
                dest.parentFile?.mkdirs()
                dest.delete()
                val input = openFile(name)
                input.use { copy(name, it, partial, maxBytes) { ctx.ensureContextActive() } }
                Files.move(partial.toPath(), dest.toPath(), StandardCopyOption.REPLACE_EXISTING)
            } finally {
                partial.delete()
            }
        }
    }

    private fun readSmall(name: String): ByteArray {
        val size = folder.length(name) ?: missing(name)
        refuseUnless(size <= DeclaredParser.MAX_BYTES, UpdateRefusalCode.TOO_LARGE) { "$name is $size bytes" }
        val out = ByteArrayOutputStream()
        openFile(name).use { input ->
            val buffer = ByteArray(BUFFER_SIZE)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                out.write(buffer, 0, read)
                refuseUnless(out.size() <= DeclaredParser.MAX_BYTES, UpdateRefusalCode.TOO_LARGE) { "$name is larger than 64 KiB" }
            }
        }
        return out.toByteArray()
    }

    private fun openFile(name: String): InputStream =
        try {
            folder.open(name) ?: missing(name)
        } catch (e: IOException) {
            refuse(UpdateRefusalCode.USB_MISSING_FILE, "$name cannot be opened", e)
        } catch (e: SecurityException) {
            refuse(UpdateRefusalCode.USB_MISSING_FILE, "$name cannot be opened", e)
        }

    private fun copy(
        name: String,
        input: InputStream,
        target: File,
        maxBytes: Long,
        active: () -> Unit,
    ) {
        target.outputStream().use { out ->
            val buffer = ByteArray(BUFFER_SIZE)
            var total = 0L
            while (true) {
                active()
                val read = input.read(buffer)
                if (read < 0) break
                total += read
                refuseUnless(total <= maxBytes, UpdateRefusalCode.TOO_LARGE) { "$name is larger than $maxBytes bytes" }
                out.write(buffer, 0, read)
            }
        }
    }

    private fun missing(name: String): Nothing = refuse(UpdateRefusalCode.USB_MISSING_FILE, "$name is not in the chosen folder")

    private companion object {
        const val BUFFER_SIZE = 64 * 1024
    }
}
