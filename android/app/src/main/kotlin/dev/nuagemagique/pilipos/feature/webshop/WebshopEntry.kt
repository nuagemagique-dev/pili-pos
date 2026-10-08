package dev.nuagemagique.pilipos.feature.webshop

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.PlaceholderScreen
import javax.inject.Inject

/** Navigation entry of the webshop feature (placeholder until P5). */
internal class WebshopEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "webshop"
        override val routes: List<String> = listOf(WebshopRoutes.WEBSHOP)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(WebshopRoutes.WEBSHOP) { PlaceholderScreen(area, WebshopRoutes.WEBSHOP) }
        }
    }
