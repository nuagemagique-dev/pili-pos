package dev.nuagemagique.pilipos.ui.badge

import android.app.Activity
import android.content.Context
import android.content.pm.PackageManager
import android.nfc.NfcAdapter
import androidx.test.core.app.ApplicationProvider
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.shadows.ShadowNfcAdapter

/** Lane 5: the device adapter behind [NfcSupport] (the flags themselves are not exposed by the shadow). */
@RunWith(RobolectricTestRunner::class)
class AndroidNfcSupportTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private val support = AndroidNfcSupport(context)

    @After
    fun tearDown() {
        ShadowNfcAdapter.reset()
    }

    private fun feature(on: Boolean) = shadowOf(context.packageManager).setSystemFeature(PackageManager.FEATURE_NFC, on)

    @Test
    fun hasNfcFollowsThePackageManagerFeature() {
        feature(false)
        assertFalse(support.hasNfc)
        feature(true)
        assertTrue(support.hasNfc)
    }

    @Test
    fun withoutAnAdapterNothingIsEnabledAndNothingThrows() {
        feature(false)
        ShadowNfcAdapter.setNfcHardwareExists(false)
        assertNull(NfcAdapter.getDefaultAdapter(context))
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()

        assertFalse(support.isEnabled)
        support.enableReaderMode(activity) { error("no tag expected") }
        support.disableReaderMode(activity)
    }

    @Test
    fun isEnabledFollowsTheAdapter() {
        feature(true)
        ShadowNfcAdapter.setNfcHardwareExists(true)
        val adapter = NfcAdapter.getDefaultAdapter(context)
        assertNotNull(adapter)

        shadowOf(adapter).setEnabled(true)
        assertTrue(support.isEnabled)
        shadowOf(adapter).setEnabled(false)
        assertFalse(support.isEnabled)
    }

    @Test
    fun readerModeStartsOnTheActivityDeliversTheTagIdAndEnds() {
        feature(true)
        ShadowNfcAdapter.setNfcHardwareExists(true)
        val adapter = NfcAdapter.getDefaultAdapter(context)
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val got = mutableListOf<ByteArray>()

        support.enableReaderMode(activity) { got += it }

        assertTrue(shadowOf(adapter).isInReaderMode)
        val tag = ShadowNfcAdapter.createMockTag()
        shadowOf(adapter).dispatchTagDiscovered(tag)
        assertEquals(1, got.size)
        assertArrayEquals(tag.id, got.single())

        support.disableReaderMode(activity)
        assertFalse(shadowOf(adapter).isInReaderMode)
    }
}
