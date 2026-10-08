package dev.nuagemagique.pilipos.nav

import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.Multibinds

/**
 * Declares the three sets [NavRegistry] collects, so they exist (possibly empty) whichever features are
 * present. Features contribute with `@Binds @IntoSet` / `@Provides @IntoSet` in their own modules.
 */
@Module
@InstallIn(SingletonComponent::class)
abstract class NavModule {
    @Multibinds
    abstract fun featureEntries(): Set<@JvmSuppressWildcards FeatureEntry>

    @Multibinds
    abstract fun settingsEntries(): Set<@JvmSuppressWildcards SettingsEntry>

    @Multibinds
    abstract fun shellOverlays(): Set<@JvmSuppressWildcards ShellOverlay>
}
