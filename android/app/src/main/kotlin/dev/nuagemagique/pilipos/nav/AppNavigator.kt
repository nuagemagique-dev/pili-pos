package dev.nuagemagique.pilipos.nav

import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavHostController

/** What a screen may ask of the navigation, without seeing the controller. */
interface AppNavigator {
    /** Opens [route] on top of the current screen. */
    fun navigate(route: String)

    /** Goes back one screen. */
    fun back()

    /** Returns to the login screen and drops everything above it. */
    fun toLogin()

    /** Opens [route] as the only screen of the back stack. */
    fun replaceAll(route: String)
}

/** [AppNavigator] backed by a Compose [NavHostController]. */
class NavControllerNavigator(
    private val navController: NavHostController,
) : AppNavigator {
    override fun navigate(route: String) {
        navController.navigate(route)
    }

    override fun back() {
        navController.popBackStack()
    }

    override fun toLogin() {
        relaunchStart(NavContract.START_ROUTE)
    }

    override fun replaceAll(route: String) {
        relaunchStart(route)
    }

    /** Pops to the start destination (itself included) and opens [route] single-top: one entry remains. */
    private fun relaunchStart(route: String) {
        navController.navigate(route) {
            popUpTo(navController.graph.findStartDestination().id) { inclusive = true }
            launchSingleTop = true
        }
    }
}
