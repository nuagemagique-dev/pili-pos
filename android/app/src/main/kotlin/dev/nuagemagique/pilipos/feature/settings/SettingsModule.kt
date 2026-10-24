package dev.nuagemagique.pilipos.feature.settings

import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.settings.menu.MenuEditorEntry
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.SettingsEntry as SettingsRow

/** Contributes the settings entry, the menu editor entry and its settings row to the navigation sets. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class SettingsModule {
    @Binds
    @IntoSet
    abstract fun bindSettingsEntry(entry: SettingsEntry): FeatureEntry

    /** The menu editor screen (V11-D17), a second entry of this feature. */
    @Binds
    @IntoSet
    abstract fun bindMenuEditorEntry(entry: MenuEditorEntry): FeatureEntry

    companion object {
        /** Settings row id of the menu editor. */
        const val MENU_SETTINGS_ID = "settings.menu"

        @Provides
        @IntoSet
        fun provideMenuEditorSettings(): SettingsRow =
            SettingsRow(
                id = MENU_SETTINGS_ID,
                titleRes = R.string.menu_editor_title,
                route = SettingsRoutes.MENU,
                order = 100,
                managerOnly = true,
            )
    }
}
