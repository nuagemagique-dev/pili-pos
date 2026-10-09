package dev.nuagemagique.pilipos.platform.export

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** Records the persisted grants instead of asking a content provider. */
class FakeGrants : UriGrants {
    val held = mutableSetOf<Uri>()
    val log = mutableListOf<String>()
    var releaseFails = false

    override fun take(uri: Uri) {
        log += "take $uri"
        held += uri
    }

    override fun release(uri: Uri) {
        log += "release $uri"
        if (releaseFails) throw SecurityException("no grant")
        held -= uri
    }

    override fun isHeld(uri: Uri): Boolean = uri in held
}

fun treeUri(
    uuid: String,
    path: String,
): Uri = DocumentsContract.buildTreeDocumentUri("com.android.externalstorage.documents", "$uuid:$path")

@RunWith(RobolectricTestRunner::class)
class ExportFolderStoreTest {
    private val sd = "BFD1-0A11"
    private val usb = "1234-ABCD"
    private lateinit var grants: FakeGrants
    private lateinit var store: ExportFolderStore

    @Before
    fun setUp() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        context.getSharedPreferences(ExportFolderStore.PREFS_NAME, Context.MODE_PRIVATE).edit().clear().commit()
        grants = FakeGrants()
        store = ExportFolderStore(context.getSharedPreferences(ExportFolderStore.PREFS_NAME, Context.MODE_PRIVATE), grants)
    }

    @Test
    fun theRootOfAVolumeIsRefusedAndNothingIsStored() {
        val result = store.take(sd, treeUri(sd, ""))
        assertEquals(TakeResult.Refused(ExportProblem.ROOT), result)
        assertNull(store.folder(sd))
        assertTrue(grants.log.isEmpty())
        assertEquals(R.string.plat_export_err_root, ExportProblem.ROOT.textRes)
    }

    @Test
    fun aTreeOfAnotherVolumeIsRefused() {
        assertEquals(TakeResult.Refused(ExportProblem.WRONG_VOLUME), store.take(sd, treeUri("primary", "Download")))
        assertEquals(TakeResult.Refused(ExportProblem.WRONG_VOLUME), store.take(sd, treeUri(usb, "PILI-EXPORT")))
        assertTrue(grants.log.isEmpty())
    }

    @Test
    fun aUriThatIsNotATreeIsRefused() {
        val notATree = Uri.parse("content://com.android.externalstorage.documents/document/BFD1-0A11%3Ax")
        assertEquals(TakeResult.Refused(ExportProblem.WRONG_VOLUME), store.take(sd, notATree))
    }

    @Test
    fun aFolderIsStoredWithItsPersistedGrant() {
        val tree = treeUri(sd, "PILI-EXPORT")
        assertEquals(TakeResult.Taken(tree), store.take(sd, tree))
        assertEquals(tree, store.folder(sd))
        assertEquals(listOf("take $tree"), grants.log)
        assertEquals("PILI-EXPORT", ExportFolderStore.displayName(tree))
    }

    @Test
    fun theUuidMatchIgnoresCase() {
        val tree = treeUri(sd.lowercase(), "PILI-EXPORT")
        assertTrue(store.take(sd, tree) is TakeResult.Taken)
    }

    @Test
    fun aFolderSurvivesANewStoreOnTheSamePreferences() {
        val tree = treeUri(sd, "PILI-EXPORT")
        store.take(sd, tree)
        val context = ApplicationProvider.getApplicationContext<Context>()
        val again = ExportFolderStore(context.getSharedPreferences(ExportFolderStore.PREFS_NAME, Context.MODE_PRIVATE), grants)
        assertEquals(tree, again.folder(sd))
    }

    @Test
    fun aFolderWhoseGrantIsGoneIsNotReturned() {
        val tree = treeUri(sd, "PILI-EXPORT")
        store.take(sd, tree)
        grants.held.clear()
        assertNull(store.folder(sd))
    }

    @Test
    fun eachVolumeKeepsItsOwnFolder() {
        val a = treeUri(sd, "A")
        val b = treeUri(usb, "B")
        store.take(sd, a)
        store.take(usb, b)
        assertEquals(a, store.folder(sd))
        assertEquals(b, store.folder(usb))
        store.change(sd)
        assertNull(store.folder(sd))
        assertEquals(b, store.folder(usb))
    }

    @Test
    fun choosingAnotherFolderReleasesTheOldGrant() {
        val first = treeUri(sd, "A")
        val second = treeUri(sd, "B")
        store.take(sd, first)
        store.take(sd, second)
        assertEquals(second, store.folder(sd))
        assertEquals(listOf("take $first", "take $second", "release $first"), grants.log)
        assertFalse(first in grants.held)
    }

    @Test
    fun choosingTheSameFolderAgainReleasesNothing() {
        val tree = treeUri(sd, "A")
        store.take(sd, tree)
        store.take(sd, tree)
        assertEquals(listOf("take $tree", "take $tree"), grants.log)
        assertTrue(tree in grants.held)
    }

    @Test
    fun changeFolderReleasesTheGrantAndForgetsTheFolder() {
        val tree = treeUri(sd, "A")
        store.take(sd, tree)
        store.change(sd)
        assertNull(store.folder(sd))
        assertEquals("release $tree", grants.log.last())
        store.change(sd)
        assertEquals(2, grants.log.size)
    }

    @Test
    fun aGrantThatCannotBeReleasedDoesNotBreakTheChange() {
        val tree = treeUri(sd, "A")
        store.take(sd, tree)
        grants.releaseFails = true
        store.change(sd)
        assertNull(store.folder(sd))
    }

    @Test
    fun displayNameOfAnUnparsableUriIsTheUriItself() {
        assertEquals("content://x/y", ExportFolderStore.displayName(Uri.parse("content://x/y")))
    }
}
