package dev.nuagemagique.pilipos.feature.tables

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.PlaceholderScreen
import javax.inject.Inject

/** Navigation entry of the tables feature (placeholder until P5). */
internal class TablesEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "tables"
        override val routes: List<String> = listOf(TablesRoutes.TABLES)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(TablesRoutes.TABLES) { PlaceholderScreen(area, TablesRoutes.TABLES) }
        }
    }
