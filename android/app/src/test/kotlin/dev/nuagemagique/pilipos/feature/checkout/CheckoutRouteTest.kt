package dev.nuagemagique.pilipos.feature.checkout

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
class CheckoutRouteTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    private val navigator = RecordingNavigator()

    @Test
    fun withTheEngineClosedTheScreenShowsTheHeaderAndTheEmptyStateAndDoesNotNavigate() {
        compose.setContent { CheckoutRoute(navigator, orderId = 12L) }
        compose.waitForIdle()

        compose.onNodeWithTag("placeholder_checkout").assertIsDisplayed()
        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("checkout_empty").assertIsDisplayed()
        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    fun aRouteWithoutAnOrderIdStillRendersAndStaysPut() {
        compose.setContent { CheckoutRoute(navigator, orderId = -1L) }
        compose.waitForIdle()

        compose.onNodeWithTag("placeholder_checkout").assertIsDisplayed()
        compose.onNodeWithTag("checkout_empty").assertIsDisplayed()
        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    fun theCheckoutEntryRegistersTheRouteWithTheOrderIdAndKeepsItsConstructor() {
        val entry = CheckoutEntry()

        assertEquals("checkout", entry.area)
        assertEquals(listOf(CheckoutRoutes.CHECKOUT), entry.routes)
        assertEquals("checkout/{orderId}", CheckoutRoutes.CHECKOUT)
        assertEquals("checkout/42", CheckoutRoutes.checkout(42L))
    }

    @Test
    fun theHeaderLockButtonLeavesTheCheckoutScreenForLogin() {
        compose.setContent { CheckoutRoute(navigator, orderId = 12L) }
        compose.waitForIdle()

        compose.onNodeWithTag("header_lock").performClick()
        compose.waitUntil(timeoutMillis = 5_000) { navigator.calls.isNotEmpty() }

        assertEquals(listOf("toLogin"), navigator.calls)
    }
}
