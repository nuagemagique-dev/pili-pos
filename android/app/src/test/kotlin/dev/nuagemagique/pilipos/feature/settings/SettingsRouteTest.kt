package dev.nuagemagique.pilipos.feature.settings

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dagger.Lazy
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.nav.NavRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import javax.inject.Inject

/** The route in a plain activity over the real graph, the way the foundation tests host it: engine closed, no user. */
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class, qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsRouteTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    @Inject
    lateinit var registry: NavRegistry

    private val navigator = RecordingNavigator()

    private val lazyRegistry: Lazy<NavRegistry> = Lazy { registry }

    @Before
    fun inject() {
        hiltRule.inject()
    }

    @Test
    fun withTheEngineClosedTheScreenShowsTheHeaderAndTheRowsAndDoesNotNavigate() {
        compose.setContent { SettingsRoute(navigator, lazyRegistry) }
        compose.waitForIdle()

        compose.onNodeWithTag("placeholder_settings").assertIsDisplayed()
        compose.onNodeWithTag("app_header").assertIsDisplayed()
        compose.onNodeWithTag("settings_rows").assertIsDisplayed()
        registry.settingsEntries.filter { !it.managerOnly }.forEach {
            compose.onNodeWithTag("settings_row_" + it.id).assertIsDisplayed()
        }
        compose.onNodeWithTag("settings_workers").assertDoesNotExist()
        assertEquals(emptyList<String>(), navigator.calls)
    }

    @Test
    @Config(qualifiers = "w320dp-h470dp-mdpi")
    fun onTheFoundationTestsDefaultWindowTheOpenRowsAreStillDisplayed() {
        compose.setContent { SettingsRoute(navigator, lazyRegistry) }
        compose.waitForIdle()

        compose.onNodeWithTag("placeholder_settings").assertIsDisplayed()
        registry.settingsEntries.filter { !it.managerOnly }.forEach {
            compose.onNodeWithTag("settings_row_" + it.id).assertIsDisplayed()
        }
    }

    @Test
    fun aRowTapOpensItsRouteThroughTheNavigator() {
        val row = registry.settingsEntries.first { !it.managerOnly }
        compose.setContent { SettingsRoute(navigator, lazyRegistry) }
        compose.waitForIdle()

        compose.onNodeWithTag("settings_row_" + row.id).performClick()

        assertEquals(listOf("navigate:" + row.route), navigator.calls)
    }

    @Test
    fun theSettingsEntryRegistersTheRouteAndKeepsItsConstructor() {
        val entry = SettingsEntry(lazyRegistry)

        assertEquals("settings", entry.area)
        assertEquals(listOf(SettingsRoutes.SETTINGS), entry.routes)
        assertTrue(registry.settingsEntries.isNotEmpty())
    }
}
