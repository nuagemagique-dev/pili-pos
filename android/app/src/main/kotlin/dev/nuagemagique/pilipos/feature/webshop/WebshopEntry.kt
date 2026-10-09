package dev.nuagemagique.pilipos.feature.webshop

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import javax.inject.Inject

/** Navigation entry of the webshop feature: the online orders screen (P5b-T11). */
internal class WebshopEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "webshop"
        override val routes: List<String> = listOf(WebshopRoutes.WEBSHOP)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(WebshopRoutes.WEBSHOP) { WebshopRoute(navigator) }
        }
    }
