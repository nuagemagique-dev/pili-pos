package dev.nuagemagique.pilipos.feature.tables

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
class TablesRouteTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    private val navigator = RecordingNavigator()

    @Test
    fun withTheEngineClosedTheScreenShowsTheHeaderAndTheSignedOutStateAndDoesNotNavigate() {
        compose.setContent { TablesRoute(navigator) }
        compose.waitForIdle()

        compose.onNodeWithTag("placeholder_tables").assertIsDisplayed()
        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("tables_empty").assertIsDisplayed()
        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    fun theTablesEntryRegistersTheRouteAndKeepsItsConstructor() {
        val entry = TablesEntry()

        assertEquals("tables", entry.area)
        assertEquals(listOf(TablesRoutes.TABLES), entry.routes)
    }

    @Test
    fun theHeaderLockButtonLeavesTheTablesScreenForLogin() {
        compose.setContent { TablesRoute(navigator) }
        compose.waitForIdle()

        compose.onNodeWithTag("header_lock").performClick()
        compose.waitUntil(timeoutMillis = 5_000) { navigator.calls.isNotEmpty() }

        assertEquals(listOf("toLogin"), navigator.calls)
    }
}
