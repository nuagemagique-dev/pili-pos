package dev.nuagemagique.pilipos.ui.badge

import android.app.Activity
import io.mockk.mockk
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class NfcBadgeSourceTest {
    private val support = FakeNfcSupport()
    private val source = NfcBadgeSource(support)
    private val activity: Activity = mockk(relaxed = true)

    @Test
    fun tagIdBytesBecomeUppercaseHex() {
        assertEquals("04A2243B1C5D80", tagIdToHex(byteArrayOf(0x04, 0xA2.toByte(), 0x24, 0x3B, 0x1C, 0x5D, 0x80.toByte())))
        assertEquals("00FF0A", tagIdToHex(byteArrayOf(0x00, 0xFF.toByte(), 0x0A)))
        assertEquals("", tagIdToHex(byteArrayOf()))
    }

    @Test
    fun theReaderModeStartsOnlyWithAnActivityAndTheSetting() {
        source.attach(activity)
        assertTrue(support.calls.isEmpty())
        source.start()
        assertEquals(listOf("enable"), support.calls)
        assertTrue(source.active)
    }

    @Test
    fun theSettingFirstThenTheActivityAlsoStarts() {
        source.start()
        assertTrue(support.calls.isEmpty())
        source.attach(activity)
        assertEquals(listOf("enable"), support.calls)
    }

    @Test
    fun noNfcOnTheDeviceMeansNoReaderMode() {
        support.hasNfc = false
        source.start()
        source.attach(activity)
        assertTrue(support.calls.isEmpty())
        assertFalse(source.active)
    }

    @Test
    fun anAdapterTurnedOffMeansNoReaderModeUntilTheNextResume() {
        support.isEnabled = false
        source.start()
        source.attach(activity)
        assertTrue(support.calls.isEmpty())
        support.isEnabled = true
        source.attach(activity)
        assertEquals(listOf("enable"), support.calls)
    }

    @Test
    fun pausingTheActivityEndsTheReaderMode() {
        source.start()
        source.attach(activity)
        source.detach(activity)
        assertEquals(listOf("enable", "disable"), support.calls)
        assertFalse(source.active)
    }

    @Test
    fun aDifferentActivityDoesNotDetachTheCurrentOne() {
        source.start()
        source.attach(activity)
        source.detach(mockk(relaxed = true))
        assertEquals(listOf("enable"), support.calls)
    }

    @Test
    fun stoppingTheSourceEndsTheReaderModeAndTheSettingChangeIsFollowed() {
        source.start()
        source.attach(activity)
        source.stop()
        assertEquals(listOf("enable", "disable"), support.calls)
        source.start()
        assertEquals(listOf("enable", "disable", "enable"), support.calls)
    }

    @Test
    fun startingTwiceDoesNotEnableTwice() {
        source.start()
        source.attach(activity)
        source.start()
        assertEquals(listOf("enable"), support.calls)
    }

    @Test
    fun aTagIsHandedOverAsHex() =
        runTest(UnconfinedTestDispatcher()) {
            val scans = mutableListOf<String>()
            val job = launch { source.scans.toList(scans) }
            source.start()
            source.attach(activity)
            support.onTag?.invoke(byteArrayOf(0x04, 0x5A, 0xFF.toByte(), 0x10))
            job.cancel()
            assertEquals(listOf("045AFF10"), scans)
        }
}
