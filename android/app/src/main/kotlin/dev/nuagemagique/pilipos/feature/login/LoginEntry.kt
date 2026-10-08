package dev.nuagemagique.pilipos.feature.login

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.PlaceholderScreen
import javax.inject.Inject

/** Navigation entry of the login feature (placeholder until P5). */
internal class LoginEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "login"
        override val routes: List<String> = listOf(LoginRoutes.LOGIN)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(LoginRoutes.LOGIN) { PlaceholderScreen(area, LoginRoutes.LOGIN) }
        }
    }
