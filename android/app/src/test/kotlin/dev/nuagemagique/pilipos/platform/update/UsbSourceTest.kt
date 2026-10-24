package dev.nuagemagique.pilipos.platform.update

import android.content.Context
import androidx.documentfile.provider.DocumentFile
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.ByteArrayInputStream
import java.io.File
import java.io.IOException
import java.io.InputStream

@RunWith(RobolectricTestRunner::class)
class UsbSourceTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private class MapFolder(
        val files: MutableMap<String, ByteArray> = mutableMapOf(),
        var failOpen: Boolean = false,
    ) : UsbFolder {
        override fun length(name: String): Long? = files[name]?.size?.toLong()

        override fun open(name: String): InputStream? {
            if (failOpen) throw IOException("gone")
            return files[name]?.let { ByteArrayInputStream(it) }
        }
    }

    private fun full() =
        MapFolder(
            mutableMapOf(
                "declared.json" to UpdateFixtures.json,
                "declared.json.minisig" to UpdateFixtures.minisig.toByteArray(),
                UpdateFixtures.ASSET_NAME to UpdateFixtures.ASSET_BYTES,
            ),
        )

    private suspend fun refusal(block: suspend () -> Unit): UpdateException {
        try {
            block()
        } catch (e: UpdateException) {
            return e
        }
        error("no refusal")
    }

    @Test
    fun theThreeFilesAreReadFromTheFolder() =
        runTest {
            val source = UsbSource(full())
            val files = source.fetchDescriptor()
            assertArrayEquals(UpdateFixtures.json, files.jsonBytes)
            assertEquals(UpdateFixtures.minisig, files.minisigText)
            val dest = File(tmp.root, "update/${UpdateFixtures.ASSET_NAME}")
            source.fetchAsset(UpdateFixtures.ASSET_NAME, dest)
            assertArrayEquals(UpdateFixtures.ASSET_BYTES, dest.readBytes())
            assertFalse(File(dest.path + ".part").exists())
            assertEquals("usb", source.label)
        }

    @Test
    fun anExistingDestinationIsReplaced() =
        runTest {
            val dest = File(tmp.root, "a.apk").also { it.writeText("old and longer than the new one, surely") }
            UsbSource(full()).fetchAsset(UpdateFixtures.ASSET_NAME, dest)
            assertArrayEquals(UpdateFixtures.ASSET_BYTES, dest.readBytes())
        }

    @Test
    fun everyMissingFileIsUsbMissingFile() =
        runTest {
            listOf("declared.json", "declared.json.minisig").forEach { name ->
                val folder = full().also { it.files.remove(name) }
                assertEquals(UpdateRefusalCode.USB_MISSING_FILE, refusal { UsbSource(folder).fetchDescriptor() }.code)
            }
            val folder = full().also { it.files.remove(UpdateFixtures.ASSET_NAME) }
            val dest = File(tmp.root, "a.apk")
            val error = refusal { UsbSource(folder).fetchAsset(UpdateFixtures.ASSET_NAME, dest) }
            assertEquals(UpdateRefusalCode.USB_MISSING_FILE, error.code)
            assertFalse(dest.exists())
        }

    @Test
    fun aFileThatCannotBeOpenedIsUsbMissingFile() =
        runTest {
            val folder = full().also { it.failOpen = true }
            assertEquals(UpdateRefusalCode.USB_MISSING_FILE, refusal { UsbSource(folder).fetchDescriptor() }.code)
        }

    @Test
    fun theSizeLimitsApply() =
        runTest {
            val big = full().also { it.files["declared.json"] = ByteArray(DeclaredParser.MAX_BYTES + 1) }
            assertEquals(UpdateRefusalCode.TOO_LARGE, refusal { UsbSource(big).fetchDescriptor() }.code)
            val dest = File(tmp.root, "a.apk")
            val error = refusal { UsbSource(full()).fetchAsset(UpdateFixtures.ASSET_NAME, dest, maxBytes = 5) }
            assertEquals(UpdateRefusalCode.TOO_LARGE, error.code)
            assertFalse(dest.exists())
        }

    @Test
    fun aFileLongerThanItsReportedSizeIsCutWhileCopying() =
        runTest {
            val lying =
                object : UsbFolder {
                    override fun length(name: String): Long? = 3L

                    override fun open(name: String): InputStream? = ByteArrayInputStream(ByteArray(5_000))
                }
            val dest = File(tmp.root, "a.apk")
            val error = refusal { UsbSource(lying).fetchAsset("pili-pos-android-1.2.0.apk", dest, maxBytes = 1_000) }
            assertEquals(UpdateRefusalCode.TOO_LARGE, error.code)
            assertFalse(dest.exists())
            assertFalse(File(dest.path + ".part").exists())
        }

    @Test
    fun aSafFolderIsReadThroughTheContentResolver() =
        runTest {
            val dir = tmp.newFolder("stick")
            File(dir, "declared.json").writeBytes(UpdateFixtures.json)
            File(dir, "declared.json.minisig").writeText(UpdateFixtures.minisig)
            File(dir, UpdateFixtures.ASSET_NAME).writeBytes(UpdateFixtures.ASSET_BYTES)
            val context = ApplicationProvider.getApplicationContext<Context>()
            val folder = DocumentUsbFolder(context.contentResolver, DocumentFile.fromFile(dir))
            val source = UsbSource(folder)
            assertArrayEquals(UpdateFixtures.json, source.fetchDescriptor().jsonBytes)
            val dest = File(tmp.root, "out.apk")
            source.fetchAsset(UpdateFixtures.ASSET_NAME, dest)
            assertArrayEquals(UpdateFixtures.ASSET_BYTES, dest.readBytes())
            assertEquals(UpdateFixtures.ASSET_BYTES.size.toLong(), folder.length(UpdateFixtures.ASSET_NAME))
            assertTrue(folder.length("nothing.apk") == null)
            assertTrue(folder.open("nothing.apk") == null)
        }
}
