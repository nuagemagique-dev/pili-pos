package dev.nuagemagique.pilipos.feature.reports

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import javax.inject.Inject

/** Navigation entry of the reports feature: the reports screen (P5b-T9). */
internal class ReportsEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "reports"
        override val routes: List<String> = listOf(ReportsRoutes.REPORTS)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(ReportsRoutes.REPORTS) { ReportsRoute(navigator) }
        }
    }
