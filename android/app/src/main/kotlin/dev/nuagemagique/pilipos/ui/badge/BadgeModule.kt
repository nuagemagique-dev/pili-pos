package dev.nuagemagique.pilipos.ui.badge

import androidx.compose.runtime.Composable
import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.ShellOverlay
import javax.inject.Inject
import javax.inject.Singleton

/** Stacking order of the badge overlay: below every visible overlay (it draws nothing). */
const val BADGE_OVERLAY_ORDER: Int = 11

/**
 * Draws nothing; its job is to make the shell build the [BadgeStore] at start-up, so the reader is armed from the
 * login screen on, before anyone opens Settings.
 */
class BadgeOverlay
    @Inject
    constructor(
        @Suppress("unused") private val store: BadgeStore,
    ) : ShellOverlay {
        override val order: Int = BADGE_OVERLAY_ORDER

        @Composable
        override fun Content() = Unit
    }

/** Binds the badge reader pieces: the sources, the device NFC, the cue and the overlay that starts the store. */
@Module
@InstallIn(SingletonComponent::class)
abstract class BadgeModule {
    @Binds
    @IntoSet
    abstract fun badgeOverlay(overlay: BadgeOverlay): ShellOverlay

    @Binds
    abstract fun badgeCue(cue: AudioTrackBadgeCue): BadgeCue

    @Binds
    abstract fun nfcSupport(support: AndroidNfcSupport): NfcSupport

    companion object {
        @Provides
        @Singleton
        fun badgeSources(
            wedge: KeyWedgeSource,
            nfc: NfcBadgeSource,
        ): BadgeSources = BadgeSources(wedge, nfc)
    }
}
