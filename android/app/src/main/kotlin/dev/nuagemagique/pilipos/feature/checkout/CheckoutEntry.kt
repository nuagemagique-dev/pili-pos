package dev.nuagemagique.pilipos.feature.checkout

import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import javax.inject.Inject

/** Navigation entry of the checkout feature: the checkout screen of the order in the route (P5b-T8). */
internal class CheckoutEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "checkout"
        override val routes: List<String> = listOf(CheckoutRoutes.CHECKOUT)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(
                route = CheckoutRoutes.CHECKOUT,
                arguments = listOf(navArgument(CheckoutRoutes.ARG_ORDER_ID) { type = NavType.LongType }),
            ) { entry ->
                val orderId = entry.arguments?.getLong(CheckoutRoutes.ARG_ORDER_ID) ?: -1L
                CheckoutRoute(navigator, orderId)
            }
        }
    }
