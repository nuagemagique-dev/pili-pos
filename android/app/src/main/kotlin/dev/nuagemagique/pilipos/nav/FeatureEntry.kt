package dev.nuagemagique.pilipos.nav

import androidx.navigation.NavGraphBuilder

/**
 * One feature's contribution to the navigation graph.
 *
 * A feature binds its entry into the Hilt set with `@Binds @IntoSet` in its own module, so adding a screen
 * never edits a shared file. [NavRegistry] validates the whole set at construction.
 */
interface FeatureEntry {
    /** Short lowercase name of the feature, used in validation messages and to sort entries. */
    val area: String

    /** Every route pattern [register] adds to the graph (for example `order/{orderId}`), used for validation. */
    val routes: List<String>

    /** Adds the feature's destinations to [builder]; [navigator] moves between screens. */
    fun register(
        builder: NavGraphBuilder,
        navigator: AppNavigator,
    )
}
