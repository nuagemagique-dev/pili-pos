package dev.nuagemagique.pilipos.nav

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class AppNavHostTest {
    @get:Rule
    val compose = createComposeRule()

    private lateinit var navigator: AppNavigator
    private lateinit var controller: NavHostController

    private val orderEntry =
        object : FeatureEntry {
            override val area = "order"
            override val routes = listOf("order/{orderId}")

            override fun register(
                builder: NavGraphBuilder,
                navigator: AppNavigator,
            ) {
                builder.composable("order/{orderId}", listOf(navArgument("orderId") { type = NavType.LongType })) {
                    PlaceholderScreen("order", "order/" + it.arguments?.getLong("orderId"))
                }
            }
        }

    private fun show() {
        val registry =
            registryOf(setOf(FakeEntry("login", "login"), FakeEntry("tables", "tables"), orderEntry))
        compose.setContent {
            controller = rememberNavController()
            AppNavHost(registry, controller) { navigator = it }
        }
        compose.waitForIdle()
    }

    private fun loginEntries(): Int = controller.currentBackStack.value.count { it.destination.route == "login" }

    @Test
    fun startsOnLogin() {
        show()
        compose.onNodeWithTag("placeholder_login").assertIsDisplayed()
        compose.onNodeWithText("login").assertIsDisplayed()
    }

    @Test
    fun navigatesToTablesAndBack() {
        show()
        compose.runOnUiThread { navigator.navigate("tables") }
        compose.onNodeWithTag("placeholder_tables").assertIsDisplayed()
        compose.runOnUiThread { navigator.back() }
        compose.onNodeWithTag("placeholder_login").assertIsDisplayed()
    }

    @Test
    fun navigatesToOrderWithTheArgument() {
        show()
        compose.runOnUiThread { navigator.navigate("order/42") }
        compose.onNodeWithTag("placeholder_order").assertIsDisplayed()
        compose.onNodeWithText("order/42").assertIsDisplayed()
    }

    @Test
    fun toLoginLeavesASingleLoginEntry() {
        show()
        compose.runOnUiThread { navigator.navigate("tables") }
        compose.runOnUiThread { navigator.navigate("order/7") }
        compose.runOnUiThread { navigator.toLogin() }
        compose.onNodeWithTag("placeholder_login").assertIsDisplayed()
        assertEquals(1, loginEntries())
        assertEquals(0, controller.currentBackStack.value.count { it.destination.route == "tables" })
    }

    @Test
    fun replaceAllKeepsOnlyTheNewScreen() {
        show()
        compose.runOnUiThread { navigator.navigate("tables") }
        compose.runOnUiThread { navigator.replaceAll("order/9") }
        compose.onNodeWithTag("placeholder_order").assertIsDisplayed()
        assertEquals(0, loginEntries())
        assertEquals(0, controller.currentBackStack.value.count { it.destination.route == "tables" })
    }

    @Test
    fun theNavigatorIsCreatedOncePerController() {
        show()
        val first = navigator
        compose.runOnUiThread { navigator.navigate("tables") }
        compose.waitForIdle()
        assertEquals(first, navigator)
    }
}
