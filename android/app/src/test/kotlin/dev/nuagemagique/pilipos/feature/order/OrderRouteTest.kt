package dev.nuagemagique.pilipos.feature.order

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The route in a plain activity over the real graph, the way the foundation tests host it: engine closed. */
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class, qualifiers = "w1920dp-h1080dp-mdpi")
class OrderRouteTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    private val navigator = RecordingNavigator()

    @Test
    fun withTheEngineClosedTheScreenShowsTheHeaderAndTheEmptyStateAndDoesNotNavigate() {
        compose.setContent { OrderRoute(navigator, 7L) }
        compose.waitForIdle()

        compose.onNodeWithTag("placeholder_order").assertIsDisplayed()
        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("order_empty").assertIsDisplayed()
        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    fun theOrderEntryRegistersTheRouteAndKeepsItsConstructor() {
        val entry = OrderEntry()

        assertEquals("order", entry.area)
        assertEquals(listOf(OrderRoutes.ORDER), entry.routes)
        assertEquals("order/{orderId}", OrderRoutes.ORDER)
        assertEquals("order/42", OrderRoutes.order(42))
    }

    @Test
    fun theHeaderLockButtonLeavesTheOrderScreenForLogin() {
        compose.setContent { OrderRoute(navigator, 7L) }
        compose.waitForIdle()

        compose.onNodeWithTag("header_lock").performClick()
        compose.waitUntil(timeoutMillis = 5_000) { navigator.calls.isNotEmpty() }

        assertEquals(listOf("toLogin"), navigator.calls)
    }
}
