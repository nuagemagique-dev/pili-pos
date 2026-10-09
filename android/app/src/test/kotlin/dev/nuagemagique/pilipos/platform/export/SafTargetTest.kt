package dev.nuagemagique.pilipos.platform.export

import android.Manifest
import android.content.Context
import android.content.pm.ProviderInfo
import android.provider.DocumentsContract
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import java.io.File
import java.io.IOException

@RunWith(RobolectricTestRunner::class)
class SafTargetTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private lateinit var served: File
    private lateinit var context: Context

    @Before
    fun setUp() {
        context = ApplicationProvider.getApplicationContext()
        served = tmp.newFolder("served")
        TestDocumentsProvider.root = served
        TestDocumentsProvider.created.clear()
        val info =
            ProviderInfo().apply {
                authority = TestDocumentsProvider.AUTHORITY
                exported = true
                grantUriPermissions = true
                readPermission = Manifest.permission.MANAGE_DOCUMENTS
                writePermission = Manifest.permission.MANAGE_DOCUMENTS
            }
        Robolectric.buildContentProvider(TestDocumentsProvider::class.java).create(info)
    }

    private fun target() =
        SafTarget(
            context.contentResolver,
            DocumentsContract.buildTreeDocumentUri(TestDocumentsProvider.AUTHORITY, TestDocumentsProvider.ROOT_ID),
            "BFD1-0A11",
        )

    @Test
    fun directoriesAndFilesAreCreatedWrittenSyncedAndReadBack() {
        val target = target()
        val top = target.createDirectory(target.root, "pili-export-x")
        val sub = target.createDirectory(top, "logs")
        val out = target.createFile(sub, "a.log", EXPORT_MIME)
        out.write("hello".toByteArray())
        out.write(byteArrayOf(33), 0, 1)
        out.write(33)
        out.sync()
        out.close()
        assertEquals("hello!!", File(served, "pili-export-x/logs/a.log").readText())
        assertEquals("hello!!", target.openForRead(out.file).use { String(it.readBytes()) })
        assertEquals("BFD1-0A11", target.volumeUuid)
        assertEquals(listOf("pili-export-x", "logs", "a.log"), TestDocumentsProvider.created)
    }

    @Test
    fun aMissingParentIsAnIoException() {
        val target = target()
        val gone = target.createDirectory(target.root, "gone")
        served.resolve("gone").deleteRecursively()
        val failure = runCatching { target.createFile(gone, "x", EXPORT_MIME) }.exceptionOrNull()
        assertTrue(failure.toString(), failure is IOException)
    }

    @Test
    fun anExportThroughTheSafTargetVerifiesEveryFile() =
        kotlinx.coroutines.test.runTest {
            val fx = ExportFixture(this, tmp.newFolder()).open()
            fx.volumes.volumes = listOf(RemovableVolume("BFD1-0A11", "Virtual SD card"))
            val outcome = fx.exporter.run(target(), "PILI-EXPORT")
            assertTrue(outcome.toString(), outcome is ExportOutcome.Done)
            val dir = File(served, EXPORT_FOLDER)
            val lines = File(dir, "MANIFEST.sha256").readLines()
            assertTrue(lines.isNotEmpty())
            lines.forEach { line ->
                val (sha, path) = line.split("  ", limit = 2)
                assertEquals(path, sha, sha256Of(File(dir, path)))
            }
        }
}
