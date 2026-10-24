package dev.nuagemagique.pilipos.platform.update

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@RunWith(RobolectricTestRunner::class)
class UpdateMarkerStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val marker = UpdateMarker(101, 102, "1.2.0", "a".repeat(40), 1_700_000_000_000)

    @Test
    fun missingMarkerReadsAsMissing() {
        assertEquals(MarkerRead.Missing, UpdateMarkerStore(File(tmp.root, "none")).read())
    }

    @Test
    fun writtenMarkerReadsBackAndLeavesNoTemporaryFile() {
        val dir = File(tmp.root, "update")
        val store = UpdateMarkerStore(dir)
        store.write(marker)
        assertEquals(MarkerRead.Found(marker), store.read())
        assertEquals(listOf("pending.json"), dir.list()!!.toList())
        assertTrue(store.exists())
    }

    @Test
    fun aSecondWriteReplacesTheFirst() {
        val store = UpdateMarkerStore(tmp.newFolder("u"))
        store.write(marker)
        store.write(marker.copy(toVersionCode = 103))
        assertEquals(103L, (store.read() as MarkerRead.Found).marker.toVersionCode)
    }

    @Test
    fun garbageIsCorruptAndAFieldMissingIsCorrupt() {
        val dir = tmp.newFolder("c")
        val store = UpdateMarkerStore(dir)
        File(dir, "pending.json").writeText("not json")
        assertTrue(store.read() is MarkerRead.Corrupt)
        File(dir, "pending.json").writeText("""{"fromVersionCode":1}""")
        assertTrue(store.read() is MarkerRead.Corrupt)
    }

    @Test
    fun deleteRemovesTheMarkerAndIsQuietWhenThereIsNone() {
        val dir = tmp.newFolder("d")
        val store = UpdateMarkerStore(dir)
        store.delete()
        store.write(marker)
        File(dir, "pending.json.tmp").writeText("x")
        store.delete()
        assertFalse(store.exists())
        assertEquals(emptyList<String>(), dir.list()!!.toList())
    }
}
