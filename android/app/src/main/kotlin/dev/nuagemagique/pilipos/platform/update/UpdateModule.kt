package dev.nuagemagique.pilipos.platform.update

import android.content.Context
import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.di.AppLifecycleHook
import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.platform.about.IdentityProvider
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.export.VolumeProbe
import kotlinx.coroutines.CoroutineScope
import java.io.File
import javax.inject.Singleton

/** Binds the update hook and the seams of the updater. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class UpdateModule {
    @Binds
    @IntoSet
    abstract fun bindCompletionHook(hook: UpdateCompletionHook): AppLifecycleHook

    @Binds
    abstract fun bindPackageSessions(sessions: AndroidPackageSessions): PackageSessions

    @Binds
    abstract fun bindEngineRestarter(restarter: ServiceEngineRestarter): EngineRestarter

    @Binds
    abstract fun bindUpdateSources(sources: AppUpdateSources): UpdateSources

    companion object {
        @Provides
        @Singleton
        fun provideMarkerStore(
            @ApplicationContext context: Context,
        ): UpdateMarkerStore = UpdateMarkerStore(File(context.filesDir, UpdateVerifier.UPDATE_DIR))
    }
}

/** What the update receivers need from the Hilt graph; a receiver cannot be constructor-injected. */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface UpdateEntryPoint {
    fun platformAudit(): PlatformAudit

    fun statusHandler(): InstallStatusHandler

    @ApplicationScope
    fun applicationScope(): CoroutineScope
}

/** Gives [UpdateScreen] its collaborators from the Hilt graph, whatever activity hosts the screen. */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface UpdateScreenEntryPoint {
    fun preconditions(): UpdatePreconditions

    fun installer(): UpdateInstaller

    fun verifier(): UpdateVerifier

    fun identity(): IdentityProvider

    fun sources(): UpdateSources

    fun volumeProbe(): VolumeProbe
}
