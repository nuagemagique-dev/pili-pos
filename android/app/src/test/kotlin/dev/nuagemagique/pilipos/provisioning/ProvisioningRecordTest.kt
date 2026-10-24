package dev.nuagemagique.pilipos.provisioning

import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class ProvisioningRecordTest {
    private val paths = ProvisioningPaths(ApplicationProvider.getApplicationContext())
    private val store = ProvisioningRecordStore(paths)
    private val record = ProvisioningRecord("SERIAL", "ab".repeat(32), "pili dev bundle", "2026-10-08T10:00:00Z")

    @Test
    fun aRecordRoundTripsAndIsPublished() {
        store.write(record)
        assertEquals(record, store.state.value)
        assertEquals(record, ProvisioningRecordStore(paths).load())
        assertFalse(java.io.File(paths.provisioningDir, "record.json.tmp").exists())
    }

    @Test
    fun noFileReadsAsAbsent() {
        assertNull(store.load())
        assertNull(store.state.value)
    }

    @Test
    fun aCorruptFileReadsAsAbsent() {
        paths.recordFile.parentFile?.mkdirs()
        for (text in listOf("not json", "{}", "{\"format\":2,\"deviceId\":\"a\"}", "{\"format\":1,\"deviceId\":\"\"}", "")) {
            paths.recordFile.writeText(text)
            assertNull(text, store.load())
        }
    }

    @Test
    fun aFailedLoadClearsThePublishedRecord() {
        store.write(record)
        paths.recordFile.writeText("garbage")
        assertNull(store.load())
        assertNull(store.state.value)
    }

    @Test
    fun aDirectoryInPlaceOfTheFileReadsAsAbsent() {
        paths.recordFile.mkdirs()
        assertNull(store.load())
    }
}
