package dev.nuagemagique.pilipos.platform.export

import android.database.Cursor
import android.database.MatrixCursor
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import android.provider.DocumentsProvider
import java.io.File
import java.io.FileNotFoundException

/**
 * A DocumentsProvider over a directory, enough for `createDocument`, `openFileDescriptor` and
 * `openInputStream`. A document id is the path relative to [root]; the root itself is "root".
 */
class TestDocumentsProvider : DocumentsProvider() {
    override fun onCreate(): Boolean = true

    private fun fileOf(id: String): File = if (id == ROOT_ID) root else File(root, id)

    private fun idOf(file: File): String = file.relativeTo(root).path

    override fun queryRoots(projection: Array<out String>?): Cursor =
        MatrixCursor(projection ?: arrayOf(DocumentsContract.Root.COLUMN_ROOT_ID))

    override fun queryDocument(
        documentId: String,
        projection: Array<out String>?,
    ): Cursor {
        val file = fileOf(documentId)
        if (!file.exists()) throw FileNotFoundException(documentId)
        return MatrixCursor(COLUMNS).apply { add(row(documentId, file)) }
    }

    override fun queryChildDocuments(
        parentDocumentId: String,
        projection: Array<out String>?,
        sortOrder: String?,
    ): Cursor =
        MatrixCursor(COLUMNS).also { cursor ->
            fileOf(parentDocumentId).listFiles().orEmpty().forEach { cursor.addRow(row(idOf(it), it)) }
        }

    private fun MatrixCursor.add(row: Array<Any?>) = addRow(row)

    private fun row(
        id: String,
        file: File,
    ): Array<Any?> =
        arrayOf(
            id,
            file.name,
            if (file.isDirectory) Document.MIME_TYPE_DIR else "application/octet-stream",
            Document.FLAG_SUPPORTS_WRITE,
            file.length(),
        )

    override fun isChildDocument(
        parentDocumentId: String,
        documentId: String,
    ): Boolean = parentDocumentId == ROOT_ID || documentId == parentDocumentId || documentId.startsWith("$parentDocumentId/")

    override fun createDocument(
        parentDocumentId: String,
        mimeType: String,
        displayName: String,
    ): String {
        val file = File(fileOf(parentDocumentId), displayName)
        if (mimeType == Document.MIME_TYPE_DIR) file.mkdirs() else file.createNewFile()
        created += displayName
        return idOf(file)
    }

    override fun openDocument(
        documentId: String,
        mode: String,
        signal: CancellationSignal?,
    ): ParcelFileDescriptor = ParcelFileDescriptor.open(fileOf(documentId), ParcelFileDescriptor.parseMode(mode))

    companion object {
        const val AUTHORITY = "test.docs"
        const val ROOT_ID = "root"
        private val COLUMNS =
            arrayOf(
                Document.COLUMN_DOCUMENT_ID,
                Document.COLUMN_DISPLAY_NAME,
                Document.COLUMN_MIME_TYPE,
                Document.COLUMN_FLAGS,
                Document.COLUMN_SIZE,
            )

        /** The directory the provider serves; set by the test. */
        lateinit var root: File

        /** The display names asked for with createDocument. */
        val created = mutableListOf<String>()
    }
}
