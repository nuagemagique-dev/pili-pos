package dev.nuagemagique.pilipos.provisioning

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class DeviceIdReaderTest {
    @Test
    fun theSerialIsTrimmedAndAvailable() {
        assertEquals(DeviceIdResult.Available("EMULATOR37X2X12X0"), DeviceIdReader { "  EMULATOR37X2X12X0\n" }.read())
    }

    @Test
    fun aSecurityExceptionIsUnavailable() {
        val result = DeviceIdReader { throw SecurityException("needs READ_PHONE_STATE") }.read()
        assertTrue(result is DeviceIdResult.Unavailable)
        assertTrue((result as DeviceIdResult.Unavailable).detail.contains("READ_PHONE_STATE"))
    }

    @Test
    fun aBlankOrMissingSerialIsUnavailable() {
        assertTrue(DeviceIdReader { "   " }.read() is DeviceIdResult.Unavailable)
        assertTrue(DeviceIdReader { "" }.read() is DeviceIdResult.Unavailable)
        assertTrue(DeviceIdReader { null }.read() is DeviceIdResult.Unavailable)
    }

    @Test
    fun theUnknownPlaceholderIsUnavailable() {
        assertTrue(DeviceIdReader { "unknown" }.read() is DeviceIdResult.Unavailable)
        assertTrue(DeviceIdReader { "UNKNOWN" }.read() is DeviceIdResult.Unavailable)
    }

    @Test
    fun aSerialOfMoreThan600CharactersIsUnavailable() {
        assertTrue(DeviceIdReader { "x".repeat(601) }.read() is DeviceIdResult.Unavailable)
        assertEquals(DeviceIdResult.Available("x".repeat(600)), DeviceIdReader { "x".repeat(600) }.read())
    }

    @Test
    fun theProductionReaderUsesTheBuildSerial() {
        org.robolectric.shadows.ShadowBuild.setSerial("SUNMI123")
        assertEquals(DeviceIdResult.Available("SUNMI123"), DeviceIdReader().read())
    }
}
