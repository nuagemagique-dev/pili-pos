package dev.nuagemagique.pilipos.platform.export

import android.content.Context
import android.content.Intent
import android.os.Environment
import android.os.storage.StorageVolume
import io.mockk.every
import io.mockk.mockk
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test

class VolumeProbeTest {
    private val context = mockk<Context>()

    @Suppress("LongParameterList")
    private fun volume(
        uuid: String?,
        removable: Boolean = true,
        primary: Boolean = false,
        state: String = Environment.MEDIA_MOUNTED,
        label: String? = "Card $uuid",
        intent: Intent = mockk(),
    ): StorageVolume =
        mockk {
            every { this@mockk.uuid } returns uuid
            every { isRemovable } returns removable
            every { isPrimary } returns primary
            every { this@mockk.state } returns state
            every { getDescription(context) } returns label
            every { createOpenDocumentTreeIntent() } returns intent
        }

    @Test
    fun onlyMountedRemovableNonPrimaryVolumesWithAUuidCount() {
        val probe =
            StorageVolumeProbe(
                context,
            ) {
                listOf(
                    volume("BFD1-0A11"),
                    volume("PRIM", removable = false, primary = true),
                    volume("EMUL", removable = false),
                    volume("PRIM2", primary = true),
                    volume("GONE", state = Environment.MEDIA_UNMOUNTED),
                    volume("RO", state = Environment.MEDIA_MOUNTED_READ_ONLY),
                    volume(null),
                    volume("1234-ABCD", label = null),
                )
            }
        assertEquals(listOf(RemovableVolume("BFD1-0A11", "Card BFD1-0A11"), RemovableVolume("1234-ABCD", "")), probe.mounted())
    }

    @Test
    fun noVolumeMeansAnEmptyList() {
        assertEquals(emptyList<RemovableVolume>(), StorageVolumeProbe(context) { emptyList() }.mounted())
    }

    @Test
    fun thePickerIntentComesFromTheVolumeItself() {
        val intent = mockk<Intent>()
        val probe = StorageVolumeProbe(context) { listOf(volume("BFD1-0A11", intent = intent), volume("1234-ABCD")) }
        assertSame(intent, probe.pickerIntent("bfd1-0a11"))
        assertNull(probe.pickerIntent("NOPE"))
    }
}
