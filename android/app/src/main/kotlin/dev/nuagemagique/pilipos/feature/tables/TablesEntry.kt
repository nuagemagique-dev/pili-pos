package dev.nuagemagique.pilipos.feature.tables

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import javax.inject.Inject

/** Navigation entry of the tables feature: the tables screen (P5b-T6). */
internal class TablesEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "tables"
        override val routes: List<String> = listOf(TablesRoutes.TABLES)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(TablesRoutes.TABLES) { TablesRoute(navigator) }
        }
    }
