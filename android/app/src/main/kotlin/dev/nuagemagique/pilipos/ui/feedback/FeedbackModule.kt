package dev.nuagemagique.pilipos.ui.feedback

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.ShellOverlay

/** Contributes the toast stack to the shell overlays. */
@Module
@InstallIn(SingletonComponent::class)
abstract class FeedbackModule {
    @Binds
    @IntoSet
    abstract fun toastOverlay(overlay: ToastOverlay): ShellOverlay
}
