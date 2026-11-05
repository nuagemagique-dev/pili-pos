package dev.nuagemagique.pilipos.ui.feedback

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.ShellOverlay

/** Contributes the add beep to the shell overlays and binds the sound player behind it. */
@Module
@InstallIn(SingletonComponent::class)
abstract class AddBeepModule {
    @Binds
    @IntoSet
    abstract fun addBeepOverlay(overlay: AddBeepOverlay): ShellOverlay

    @Binds
    abstract fun addBeepPlayer(player: AudioTrackBeepPlayer): AddBeepPlayer
}
