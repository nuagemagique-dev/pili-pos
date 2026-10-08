package dev.nuagemagique.pilipos.platform

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.PlaceholderScreen
import javax.inject.Inject

/** Route constants of the platform lane (P6). */
object PlatformRoutes {
    /** The about screen (version, device identity). */
    const val ABOUT = "platform/about"
}

/** Navigation entry of the platform lane (placeholder until P6). */
internal class PlatformEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "platform"
        override val routes: List<String> = listOf(PlatformRoutes.ABOUT)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(PlatformRoutes.ABOUT) { PlaceholderScreen(area, PlatformRoutes.ABOUT) }
        }
    }
