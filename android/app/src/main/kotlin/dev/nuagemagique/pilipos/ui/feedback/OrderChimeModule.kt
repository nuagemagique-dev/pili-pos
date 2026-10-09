package dev.nuagemagique.pilipos.ui.feedback

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.ShellOverlay

/** Contributes the order chime to the shell overlays and binds the sound player behind it. */
@Module
@InstallIn(SingletonComponent::class)
abstract class OrderChimeModule {
    @Binds
    @IntoSet
    abstract fun orderChimeOverlay(overlay: OrderChimeOverlay): ShellOverlay

    @Binds
    abstract fun orderChimePlayer(player: AudioTrackChimePlayer): OrderChimePlayer
}
