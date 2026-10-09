package dev.nuagemagique.pilipos.platform.export

import android.content.Context
import android.content.Intent
import android.os.Environment
import android.os.storage.StorageManager
import android.os.storage.StorageVolume
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject

/** A mounted removable volume that is not the primary storage: a USB stick or an SD card. */
data class RemovableVolume(
    /** The volume uuid, the key of the persisted folder. */
    val uuid: String,
    /** The label the system shows, for example "Virtual SD card". */
    val description: String,
)

/** Finds the removable volumes and builds the folder picker intent for one (Q17). */
interface VolumeProbe {
    /** The removable, non-primary volumes that are mounted now (a missing uuid is skipped). */
    fun mounted(): List<RemovableVolume>

    /**
     * `StorageVolume.createOpenDocumentTreeIntent()` of the volume [uuid], or null when it is no longer
     * mounted. The caller starts it from the activity, never with `FLAG_ACTIVITY_NEW_TASK`.
     */
    fun pickerIntent(uuid: String): Intent?
}

/**
 * The production [VolumeProbe] over [StorageManager.getStorageVolumes]: a volume counts when it is removable,
 * not the primary storage, mounted and has a uuid. [source] is a seam so that tests list volumes of their own.
 */
class StorageVolumeProbe internal constructor(
    private val context: Context,
    private val source: () -> List<StorageVolume>,
) : VolumeProbe {
    @Inject
    constructor(
        @ApplicationContext context: Context,
    ) : this(context, { context.getSystemService(StorageManager::class.java).storageVolumes })

    private fun candidates(): List<StorageVolume> =
        source().filter { it.isRemovable && !it.isPrimary && it.state == Environment.MEDIA_MOUNTED && it.uuid != null }

    override fun mounted(): List<RemovableVolume> =
        candidates().mapNotNull { v -> v.uuid?.let { RemovableVolume(it, v.getDescription(context).orEmpty()) } }

    override fun pickerIntent(uuid: String): Intent? =
        candidates().firstOrNull { it.uuid.equals(uuid, ignoreCase = true) }?.createOpenDocumentTreeIntent()
}
