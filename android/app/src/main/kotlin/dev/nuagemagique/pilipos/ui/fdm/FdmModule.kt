package dev.nuagemagique.pilipos.ui.fdm

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.ShellOverlay

/** Contributes the FDM banner to the shell overlays. */
@Module
@InstallIn(SingletonComponent::class)
abstract class FdmModule {
    @Binds
    @IntoSet
    abstract fun fdmBannerOverlay(overlay: FdmBannerOverlay): ShellOverlay
}
