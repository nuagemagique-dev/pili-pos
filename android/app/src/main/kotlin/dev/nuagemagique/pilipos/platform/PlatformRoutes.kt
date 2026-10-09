package dev.nuagemagique.pilipos.platform

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.platform.about.AboutScreen
import dev.nuagemagique.pilipos.platform.export.ExportScreen
import dev.nuagemagique.pilipos.platform.update.UpdateScreen
import javax.inject.Inject

/** Route constants of the platform lane (P6). */
object PlatformRoutes {
    /** The about screen (version, device identity). */
    const val ABOUT = "platform/about"

    /** The SPF export screen (manager only). */
    const val EXPORT = "platform/export"

    /** The updater screen (manager only). */
    const val UPDATE = "platform/update"
}

private const val LEGACY_PLACEHOLDER_TAG = "placeholder_platform"

/** Navigation entry of the platform lane: the About, the SPF export and the update screens. */
internal class PlatformEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "platform"
        override val routes: List<String> = listOf(PlatformRoutes.ABOUT, PlatformRoutes.EXPORT, PlatformRoutes.UPDATE)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(PlatformRoutes.ABOUT) {
                // The outer tag is the one the nav-level FeatureEntriesTest still looks for ("placeholder_platform");
                // it goes away with that test's update at integration.
                Box(Modifier.testTag(LEGACY_PLACEHOLDER_TAG)) { AboutScreen() }
            }
            builder.composable(PlatformRoutes.EXPORT) { ExportScreen() }
            builder.composable(PlatformRoutes.UPDATE) { UpdateScreen() }
        }
    }
