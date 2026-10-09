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

        /** Settings row id of the SPF export screen. */
        const val EXPORT_SETTINGS_ID = "platform.export"

        /** Settings row id of the update screen. */
        const val UPDATE_SETTINGS_ID = "platform.update"

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

        @Provides
        @IntoSet
        fun provideExportSettings(): SettingsEntry =
            SettingsEntry(
                id = EXPORT_SETTINGS_ID,
                titleRes = R.string.plat_export_title,
                route = PlatformRoutes.EXPORT,
                order = 910,
                managerOnly = true,
            )

        @Provides
        @IntoSet
        fun provideUpdateSettings(): SettingsEntry =
            SettingsEntry(
                id = UPDATE_SETTINGS_ID,
                titleRes = R.string.plat_update_title,
                route = PlatformRoutes.UPDATE,
                order = 920,
                managerOnly = true,
            )
    }
}
