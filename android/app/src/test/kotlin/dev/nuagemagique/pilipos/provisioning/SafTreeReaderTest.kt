package dev.nuagemagique.pilipos.provisioning

import android.content.ContentProvider
import android.content.ContentValues
import android.content.pm.ProviderInfo
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import java.io.File

/**
 * A stand-in for a documents provider over a directory. Robolectric calls the pre-Android-O query form, which
 * a real DocumentsProvider refuses, so this is a plain provider that answers the same document URIs.
 */
class DirectoryDocumentsProvider : ContentProvider() {
    private val columns =
        arrayOf(
            Document.COLUMN_DOCUMENT_ID,
            Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_SIZE,
            Document.COLUMN_MIME_TYPE,
            Document.COLUMN_FLAGS,
            Document.COLUMN_LAST_MODIFIED,
        )

    override fun onCreate(): Boolean = true

    private fun row(
        cursor: MatrixCursor,
        file: File,
    ) {
        val values =
            mapOf(
                Document.COLUMN_DOCUMENT_ID to file.absolutePath,
                Document.COLUMN_DISPLAY_NAME to file.name,
                Document.COLUMN_SIZE to file.length(),
                Document.COLUMN_MIME_TYPE to if (file.isDirectory) Document.MIME_TYPE_DIR else "application/octet-stream",
                Document.COLUMN_FLAGS to 0,
                Document.COLUMN_LAST_MODIFIED to 0L,
            )
        cursor.addRow(cursor.columnNames.map { values[it] })
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor {
        val file = File(DocumentsContract.getDocumentId(uri))
        val cursor = MatrixCursor(projection ?: columns)
        if (uri.pathSegments.last() == "children") {
            file.listFiles().orEmpty().sortedBy { it.name }.forEach { row(cursor, it) }
        } else {
            row(cursor, file)
        }
        return cursor
    }

    override fun openFile(
        uri: Uri,
        mode: String,
    ): ParcelFileDescriptor = ParcelFileDescriptor.open(File(DocumentsContract.getDocumentId(uri)), ParcelFileDescriptor.MODE_READ_ONLY)

    override fun getType(uri: Uri): String? = null

    override fun insert(
        uri: Uri,
        values: ContentValues?,
    ): Uri? = null

    override fun delete(
        uri: Uri,
        selection: String?,
        selectionArgs: Array<out String>?,
    ): Int = 0

    override fun update(
        uri: Uri,
        values: ContentValues?,
        selection: String?,
        selectionArgs: Array<out String>?,
    ): Int = 0
}

@RunWith(RobolectricTestRunner::class)
class SafTreeReaderTest {
    @get:Rule val tmp = TemporaryFolder()

    private val authority = "dev.nuagemagique.pilipos.test.documents"

    @Test
    fun aTreeIsListedAndReadThroughTheDocumentsProvider() =
        runTest {
            val context = ApplicationProvider.getApplicationContext<android.content.Context>()
            val info =
                ProviderInfo().apply {
                    authority = this@SafTreeReaderTest.authority
                    exported = true
                    grantUriPermissions = true
                }
            Robolectric.buildContentProvider(DirectoryDocumentsProvider::class.java).create(info)

            val dir = bundleDir(tmp.root)
            File(dir, "sub").mkdirs()
            File(dir, "sub/a.txt").writeText("hello")
            val treeUri: Uri = DocumentsContract.buildTreeDocumentUri(authority, dir.absolutePath)

            val root = SafTreeReader(context).root(treeUri)
            assertNotNull(root)
            assertTrue(root!!.isDirectory)
            val children = root.children()
            assertEquals(listOf("MANIFEST.json", "MANIFEST.json.minisig", "pos.json", "sub"), children.map { it.name })
            val pos = children.first { it.name == "pos.json" }
            assertTrue(pos.isFile && !pos.isSymlink && !pos.isDirectory)
            assertEquals(14L, pos.length)
            assertEquals("{\"pos_id\":\"X\"}", pos.open()!!.use { it.readBytes().decodeToString() })
            assertFalse(children.first { it.name == "sub" }.isFile)

            // The importer copies the same tree.
            val fx = Fixture()
            fx.treeReader = SafTreeReader(context)
            assertTrue(fx.importer().import(BundleSource.TreeUri(treeUri)) is ImportResult.Success)
            assertEquals("hello", File(fx.paths.bundleDir, "sub/a.txt").readText())
        }
}
