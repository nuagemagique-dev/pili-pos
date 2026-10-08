package dev.nuagemagique.pilipos.feature.settings

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.FeatureEntry

/** Contributes the settings entry to the navigation set. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class SettingsModule {
    @Binds
    @IntoSet
    abstract fun bindSettingsEntry(entry: SettingsEntry): FeatureEntry
}
