package dev.nuagemagique.pilipos.platform

import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.SettingsEntry

/** Contributes the platform entry and its settings row to the navigation sets. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class PlatformModule {
    @Binds
    @IntoSet
    abstract fun bindPlatformEntry(entry: PlatformEntry): FeatureEntry

    companion object {
        /** Settings row id of the about screen. */
        const val ABOUT_SETTINGS_ID = "platform.about"

        @Provides
        @IntoSet
        fun provideAboutSettings(): SettingsEntry =
            SettingsEntry(
                id = ABOUT_SETTINGS_ID,
                titleRes = R.string.plat_about_title,
                route = PlatformRoutes.ABOUT,
                order = 900,
                managerOnly = false,
            )
    }
}
