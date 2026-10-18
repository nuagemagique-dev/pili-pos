package dev.nuagemagique.pilipos.platform.about

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class IdentityReportTest {
    @Test
    fun toJsonCarriesTheKeysInAStableOrder() {
        val json = sampleReport().toJson()
        val keys = json.keys().asSequence().toList()
        assertEquals(
            listOf(
                "applicationId",
                "versionName",
                "versionCode",
                "posSwVersion",
                "engineRef",
                "apkSha1",
                "apkSha256",
                "soSha1",
                "soProblem",
                "signerSha256",
                "deviceId",
                "posId",
                "terminalId",
                "androidRelease",
                "sdkInt",
                "fingerprint",
                "manufacturer",
                "model",
                "freeInternalBytes",
                "backupsCount",
                "backupsBytes",
                "generatedAt",
            ),
            keys,
        )
        assertEquals("a9993e364706816aba3e25717850c26c9cd0d89d", json.getString("apkSha1"))
        assertEquals("3f786850e387550fdab836ed7e6dc881de23001b", json.getString("soSha1"))
        assertEquals("SN-0001", json.getString("deviceId"))
        assertEquals("POS-01", json.getString("posId"))
        assertEquals("pili-pos-android/1.0.0", json.getString("posSwVersion"))
        assertEquals("pili-pos 1.0.0, linux-v1.0.0-rc3 (pending)", json.getString("engineRef"))
        assertEquals(12_000_000_000L, json.getLong("freeInternalBytes"))
        assertEquals(820_000_000L, json.getLong("backupsBytes"))
        assertEquals(1, json.getJSONArray("signerSha256").length())
    }

    @Test
    fun aMissingValueIsAJsonNullNotAnAbsentKey() {
        val json = sampleReport(posId = null, soSha1 = null).toJson()
        assertTrue(json.has("posId"))
        assertTrue(json.isNull("posId"))
        assertTrue(json.isNull("terminalId"))
        assertTrue(json.has("soSha1"))
        assertTrue(json.isNull("soSha1"))
        assertEquals("lib/arm64-v8a/libpos_ffi.so is not in the APK", json.getString("soProblem"))
    }

    @Test
    fun theJsonRoundTripsThroughItsText() {
        val json = sampleReport().toJson()
        assertEquals(json.toString(), JSONObject(json.toString()).toString())
    }

    @Test
    fun lowFreeSpaceIsBelowTwoGibibytes() {
        assertTrue(sampleReport(freeInternalBytes = IdentityReport.LOW_FREE_BYTES - 1).lowFreeSpace)
        assertFalse(sampleReport(freeInternalBytes = IdentityReport.LOW_FREE_BYTES).lowFreeSpace)
    }

    @Test
    fun backupsOverHalfOfTheFreeSpaceWarn() {
        assertTrue(sampleReport(freeInternalBytes = 1000, backupsBytes = 501).backupsOverHalfOfFree)
        assertFalse(sampleReport(freeInternalBytes = 1000, backupsBytes = 500).backupsOverHalfOfFree)
        assertFalse(sampleReport(freeInternalBytes = 0, backupsBytes = 0).backupsOverHalfOfFree)
        assertTrue(sampleReport(freeInternalBytes = 0, backupsBytes = 1).backupsOverHalfOfFree)
    }
}
