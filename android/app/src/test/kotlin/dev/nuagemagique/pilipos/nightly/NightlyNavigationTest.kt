package dev.nuagemagique.pilipos.nightly

import androidx.activity.compose.setContent
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createEmptyComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.test.core.app.ActivityScenario
import dagger.hilt.android.testing.BindValue
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import dev.nuagemagique.pilipos.nav.AppNavHost
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FakeEntry
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.nav.SettingsEntry
import dev.nuagemagique.pilipos.nav.registryOf
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.mockk
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import javax.inject.Inject

@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class NightlyNavigationTest {
    private val hiltRule = HiltAndroidRule(this)

    // MainActivity is the Hilt entry point that hiltViewModel() needs; the test replaces its content.
    @BindValue
    @JvmField
    val forwarder: UserActivityForwarder = mockk(relaxed = true)

    private val compose = createEmptyComposeRule()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    @Inject
    lateinit var entries: Set<@JvmSuppressWildcards FeatureEntry>

    @Inject
    lateinit var settings: Set<@JvmSuppressWildcards SettingsEntry>

    @Inject
    lateinit var registry: NavRegistry

    private lateinit var navigator: AppNavigator

    @Test
    fun theNightlyRouteIsRegisteredInTheGraph() {
        hiltRule.inject()
        val entry = entries.single { it.area == "nightly" }
        assertEquals(listOf(NightlyRoutes.NIGHTLY), entry.routes)
        assertEquals("nightly", NightlyRoutes.NIGHTLY)
        assertTrue(registry.entries.any { it.area == "nightly" })
    }

    @Test
    fun theNightlySettingsRowIsRegisteredForManagersOnly() {
        hiltRule.inject()
        val row = settings.single { it.id == "nightly.export" }
        assertEquals(NightlyRoutes.NIGHTLY, row.route)
        assertEquals(R.string.nightly_title, row.titleRes)
        assertEquals(800, row.order)
        assertTrue(row.managerOnly)
        assertTrue(registry.settingsEntries.any { it.id == "nightly.export" })
    }

    @Test
    fun theRowAndTheEntryFitTheRegistryValidation() {
        val rows = setOf(NightlyModule.provideNightlySettings())
        val built = registryOf(setOf(FakeEntry("login", "login"), NightlyEntry()), rows)
        assertEquals(listOf("nightly.export"), built.settingsEntries.map { it.id })
    }

    @Test
    fun openingTheRouteShowsTheScreenAndBackLeavesIt() {
        hiltRule.inject()
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            scenario.onActivity { activity ->
                activity.setContent { PiliTheme { AppNavHost(registry) { navigator = it } } }
            }
            compose.waitForIdle()
            compose.runOnUiThread { navigator.navigate(NightlyRoutes.NIGHTLY) }
            compose.onNodeWithTag("nightly_screen").assertIsDisplayed()
            compose.onNodeWithTag("nightly_back").assertIsDisplayed()
            compose.runOnUiThread { navigator.back() }
            compose.waitForIdle()
            compose.onNodeWithTag("nightly_screen").assertDoesNotExist()
        }
    }
}
