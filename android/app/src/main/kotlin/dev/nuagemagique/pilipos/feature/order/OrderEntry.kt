package dev.nuagemagique.pilipos.feature.order

import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import javax.inject.Inject

/** Navigation entry of the order feature: the order screen (P5b-T7, the cart column is filled by P5b-T12). */
internal class OrderEntry
    @Inject
    constructor() : FeatureEntry {
        override val area: String = "order"
        override val routes: List<String> = listOf(OrderRoutes.ORDER)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(
                route = OrderRoutes.ORDER,
                arguments = listOf(navArgument(OrderRoutes.ARG_ORDER_ID) { type = NavType.LongType }),
            ) { entry ->
                val orderId = entry.arguments?.getLong(OrderRoutes.ARG_ORDER_ID) ?: -1L
                OrderRoute(navigator, orderId)
            }
        }
    }
