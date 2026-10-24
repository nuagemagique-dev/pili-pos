package dev.nuagemagique.pilipos.nav

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.rememberNavController

/**
 * The navigation host: every entry of [registry] adds its destinations, the graph starts on
 * [NavContract.START_ROUTE].
 *
 * @param onNavigator receives the [AppNavigator] of [navController] (created once per controller), so the
 *   shell can navigate from outside the graph
 */
@Composable
fun AppNavHost(
    registry: NavRegistry,
    navController: NavHostController = rememberNavController(),
    onNavigator: (AppNavigator) -> Unit = {},
) {
    val navigator = remember(navController) { NavControllerNavigator(navController) }
    val currentOnNavigator by rememberUpdatedState(onNavigator)
    LaunchedEffect(navigator) { currentOnNavigator(navigator) }
    NavHost(navController = navController, startDestination = NavContract.START_ROUTE) {
        registry.entries.forEach { it.register(this, navigator) }
    }
}
