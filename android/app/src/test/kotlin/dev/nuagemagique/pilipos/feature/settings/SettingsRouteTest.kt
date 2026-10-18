package dev.nuagemagique.pilipos.feature.settings

import android.content.Context
import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.test.core.app.ApplicationProvider
import dagger.Lazy
import dagger.hilt.android.EntryPointAccessors
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.ui.badge.badgeView
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.every
import io.mockk.mockkStatic
import io.mockk.unmockkStatic
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.ActiveBadge
import uniffi.pos_ffi.BadgeOverview
import uniffi.pos_ffi.BadgeSettingsInput
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

    @get:Rule
    val tmp = TemporaryFolder()

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
    fun theBadgeLambdasReachTheRealBadgeModel() =
        runTest {
            val f = settingsFixture(tmp.root)
            val bob = ActiveBadge(CASHIER_INSZ, "Bob", "2026-10-17 10:00:00", ADMIN_INSZ, "Alice")
            coEvery { f.engine.getBadgeSettings() } returns badgeView(reader = "usb")
            coEvery { f.engine.listBadges() } returns BadgeOverview(listOf(bob), emptyList())
            coEvery { f.engine.setBadgeSettings(any()) } returns badgeView(reader = "off")
            coEvery { f.engine.revokeBadge(CASHIER_INSZ) } returns Unit
            val env = settingsEnv(f, ApplicationProvider.getApplicationContext())
            openEngine(env)
            with(env) { loginAs(admin) }
            // The route reads its entry point from the application graph: hand it the one over the fixture.
            mockkStatic(EntryPointAccessors::class)
            try {
                every { EntryPointAccessors.fromApplication(any<Context>(), any<Class<*>>()) } answers {
                    if (secondArg<Class<*>>() == SettingsFeatureEntryPoint::class.java) env.entry else callOriginal()
                }
                compose.setContent { SettingsRoute(navigator, lazyRegistry) }
                compose.waitForIdle()

                // The employee row of Bob (who holds a badge) then the confirmation: the revoke lambdas.
                compose.onNodeWithTag("worker_${CASHIER_INSZ}_badge_button").performClick()
                coVerify(exactly = 0) { f.engine.revokeBadge(any()) }
                compose.onNodeWithTag("confirm_ok").performClick()
                runCurrent()
                coVerify(exactly = 1) { f.engine.revokeBadge(CASHIER_INSZ) }

                // The panel header, then the reader choice: the "off" lambda, not another one.
                compose.onNodeWithTag("badge_toggle").performClick()
                compose.onNodeWithTag("badge_reader_off").performClick()
                runCurrent()
                coVerify(exactly = 1) { f.engine.setBadgeSettings(BadgeSettingsInput("off", false, 50L, 6L, 32L)) }
            } finally {
                // A manager's route also polls the Peppol panel every 30 s: stop it, or runTest's final drain
                // loops on virtual time until the heap is exhausted.
                runCatching { ViewModelProvider(compose.activity)[PeppolViewModel::class.java].stopPolling() }
                unmockkStatic(EntryPointAccessors::class)
            }
        }

    @Test
    fun theSettingsEntryRegistersTheRouteAndKeepsItsConstructor() {
        val entry = SettingsEntry(lazyRegistry)

        assertEquals("settings", entry.area)
        assertEquals(listOf(SettingsRoutes.SETTINGS), entry.routes)
        assertTrue(registry.settingsEntries.isNotEmpty())
    }
}
