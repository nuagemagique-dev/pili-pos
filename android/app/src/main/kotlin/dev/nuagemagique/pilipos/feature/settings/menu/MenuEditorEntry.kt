package dev.nuagemagique.pilipos.feature.settings.menu

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.feature.settings.SettingsRoutes
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import javax.inject.Inject

/** Navigation entry of the menu editor (V11-D17): the screen `settings/menu`, opened from a settings row. */
internal class MenuEditorEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "settings-menu"
        override val routes: List<String> = listOf(SettingsRoutes.MENU)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(SettingsRoutes.MENU) {
                MenuEditorRoute(navigator)
            }
        }
    }
