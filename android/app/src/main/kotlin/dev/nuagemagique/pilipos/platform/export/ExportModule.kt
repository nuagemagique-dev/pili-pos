package dev.nuagemagique.pilipos.platform.export

import android.content.Context
import android.net.Uri
import dagger.Binds
import dagger.Module
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Inject

/** Opens the [ExportTarget] of a chosen folder: a seam so that the view model tests write to a temp directory. */
fun interface ExportTargets {
    /** The target for the persisted [tree] of the volume [uuid]. */
    fun open(
        tree: Uri,
        uuid: String,
    ): ExportTarget
}

/** The production [ExportTargets]: a SAF tree through the content resolver. */
class SafTargets
    @Inject
    constructor(
        @ApplicationContext private val context: Context,
    ) : ExportTargets {
        override fun open(
            tree: Uri,
            uuid: String,
        ): ExportTarget = SafTarget(context.contentResolver, tree, uuid)
    }

/** Binds the volume probe and the target factory. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class ExportModule {
    @Binds
    abstract fun bindVolumeProbe(probe: StorageVolumeProbe): VolumeProbe

    @Binds
    abstract fun bindExportTargets(targets: SafTargets): ExportTargets
}

/** Gives [ExportScreen] its collaborators from the Hilt graph, whatever activity hosts the screen. */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface ExportEntryPoint {
    fun exporter(): Exporter

    fun volumeProbe(): VolumeProbe

    fun folderStore(): ExportFolderStore

    fun targets(): ExportTargets
}
