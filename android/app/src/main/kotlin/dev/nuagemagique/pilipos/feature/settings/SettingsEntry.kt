package dev.nuagemagique.pilipos.feature.settings

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dagger.Lazy
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.NavRegistry
import javax.inject.Inject

/**
 * Navigation entry of the settings feature: the settings screen (P5b-T10).
 *
 * The registry is injected lazily: it is built from the set this entry belongs to, and the screen lists its
 * settings rows (UI-PD308).
 */
internal class SettingsEntry
    @Inject
    constructor(
        private val registry: Lazy<NavRegistry>,
    ) : FeatureEntry {
        override val area: String = "settings"
        override val routes: List<String> = listOf(SettingsRoutes.SETTINGS)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(SettingsRoutes.SETTINGS) {
                SettingsRoute(navigator, registry)
            }
        }
    }
