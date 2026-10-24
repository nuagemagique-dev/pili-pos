package dev.nuagemagique.pilipos.nightly

import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.SettingsEntry

/** Contributes the nightly export entry and its settings row to the navigation sets. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class NightlyModule {
    @Binds
    @IntoSet
    abstract fun bindNightlyEntry(entry: NightlyEntry): FeatureEntry

    @Binds
    abstract fun bindNightlyLanguage(language: SystemNightlyLanguage): NightlyLanguage

    companion object {
        /** Settings row id of the nightly export screen. */
        const val NIGHTLY_SETTINGS_ID = "nightly.export"

        /** Sort key of the row: after the regular settings, before the about row. */
        const val NIGHTLY_SETTINGS_ORDER = 800

        @Provides
        @IntoSet
        fun provideNightlySettings(): SettingsEntry =
            SettingsEntry(
                id = NIGHTLY_SETTINGS_ID,
                titleRes = R.string.nightly_title,
                route = NightlyRoutes.NIGHTLY,
                order = NIGHTLY_SETTINGS_ORDER,
                managerOnly = true,
            )
    }
}
