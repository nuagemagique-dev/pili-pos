package dev.nuagemagique.pilipos.ui.scaffold

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
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

/** The scaffold in a plain activity over the real graph, the way the foundation tests host a screen: engine closed. */
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class, qualifiers = "w1920dp-h1080dp-mdpi")
class PosScaffoldRouteTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    private val navigator = RecordingNavigator()

    private fun show(area: HeaderArea) {
        compose.setContent {
            PosScaffold(area, navigator, overlay = { Box(Modifier.testTag("screen_overlay")) }) {
                Box(Modifier.fillMaxSize().testTag("screen_content")) { Text("contenu") }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun withTheEngineClosedTheScaffoldRendersTheHeaderGreyLightsAndTheContent() {
        show(HeaderArea.TABLES)

        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("screen_content").assertIsDisplayed()
        compose.onNodeWithTag("status_dot_ticket_none").assertIsDisplayed()
        compose.onNodeWithTag("status_dot_kitchen_none").assertIsDisplayed()
        compose.onNodeWithTag("fdm_veil").assertDoesNotExist()
        compose.onNodeWithTag("header_nav_reports").assertDoesNotExist()
        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    fun everyAreaRendersWithTheEngineClosedAndNeverNavigatesByItself() {
        show(HeaderArea.CHECKOUT)

        compose.onNodeWithTag("app_header").assertIsDisplayed()
        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    fun theTablesButtonOnTheTablesScreenDoesNotNavigate() {
        show(HeaderArea.TABLES)

        compose.onNodeWithTag("header_nav_tables").performClick()
        compose.waitForIdle()

        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    fun theLockButtonGoesBackToLoginEvenWithTheEngineClosed() {
        show(HeaderArea.TABLES)

        compose.onNodeWithTag("header_lock").performClick()
        compose.waitUntil(timeoutMillis = 5_000) { navigator.calls.isNotEmpty() }

        assertEquals(listOf("toLogin"), navigator.calls)
    }
}
