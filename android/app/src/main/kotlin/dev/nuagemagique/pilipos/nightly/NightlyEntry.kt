package dev.nuagemagique.pilipos.nightly

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import javax.inject.Inject

/** Navigation entry of the nightly export screen. */
internal class NightlyEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "nightly"
        override val routes: List<String> = listOf(NightlyRoutes.NIGHTLY)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(NightlyRoutes.NIGHTLY) { NightlyScreen(onBack = navigator::back) }
        }
    }
