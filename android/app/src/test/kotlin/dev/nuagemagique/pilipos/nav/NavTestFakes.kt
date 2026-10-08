package dev.nuagemagique.pilipos.nav

import androidx.compose.runtime.Composable
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable

/** Test entry registering [routes] with a placeholder each. */
class FakeEntry(
    override val area: String,
    override val routes: List<String>,
) : FeatureEntry {
    constructor(area: String, vararg routes: String) : this(area, routes.toList())

    override fun register(
        builder: NavGraphBuilder,
        navigator: AppNavigator,
    ) {
        routes.forEach { route -> builder.composable(route) { PlaceholderScreen(area, route) } }
    }
}

/** Test overlay with a fixed order. */
class FakeOverlay(
    override val order: Int,
) : ShellOverlay {
    @Composable
    override fun Content() = Unit
}

/** Registry over the given contributions, empty by default. */
fun registryOf(
    entries: Set<FeatureEntry>,
    settings: Set<SettingsEntry> = emptySet(),
    overlays: Set<ShellOverlay> = emptySet(),
): NavRegistry = NavRegistry(entries, settings, overlays)
