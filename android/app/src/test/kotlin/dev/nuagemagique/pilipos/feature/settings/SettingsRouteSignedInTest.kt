package dev.nuagemagique.pilipos.feature.settings

import androidx.activity.ComponentActivity
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.lifecycle.ViewModelProvider
import androidx.test.core.app.ApplicationProvider
import dagger.Lazy
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.tables.RecordingNavigator
import dev.nuagemagique.pilipos.nav.FakeEntry
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.nav.registryOf
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.User

/**
 * The settings route over a signed-in operator and a mocked engine: the Peppol panel is read every 30 seconds for a
 * manager only and stops when the screen is left, and the "Carte" row is a manager's row that opens `settings/menu`.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class, qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsRouteSignedInTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()
    private val tmp = TemporaryFolder()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(tmp).around(compose)

    private val context = ApplicationProvider.getApplicationContext<android.content.Context>()
    private val navigator = RecordingNavigator()

    private val registry: NavRegistry =
        registryOf(
            entries =
                setOf(
                    FakeEntry("login", "login"),
                    FakeEntry("settings", "settings"),
                    FakeEntry("misc", "settings-open", SettingsRoutes.MENU),
                ),
            settings =
                setOf(
                    rowOpen.copy(route = "settings-open", titleRes = R.string.plat_about_title),
                    SettingsModule.provideMenuEditorSettings(),
                ),
        )

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.settle() {
        repeat(SETTLE_ROUNDS) {
            runCurrent()
            compose.waitForIdle()
        }
    }

    private suspend fun TestScope.signedIn(who: User): SettingsEnv {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.listPeppolDeliveries(any()) } returns peppolPanel()
        val env = settingsEnv(f, context)
        openEngine(env)
        with(env) { loginAs(who) }
        return env
    }

    /**
     * Runs [body] and stops the Peppol polling however it ends: `runTest` drains the scheduler when the body returns
     * or throws, and a poll still running would loop on virtual time until the heap is exhausted, hiding the failure.
     */
    private fun pollingTest(body: suspend TestScope.() -> Unit) =
        runTest {
            try {
                body()
            } finally {
                runCatching { ViewModelProvider(compose.activity)[PeppolViewModel::class.java].stopPolling() }
            }
        }

    @Test
    fun aManagerSeesThePeppolPanelAndItIsReadAgainEveryThirtySeconds() =
        pollingTest {
            val env = signedIn(admin)
            compose.setContent { SettingsRoute(navigator, Lazy { registry }, env.entry) }
            settle()

            coVerify(exactly = 1) { env.f.engine.listPeppolDeliveries(100L) }
            // The third column scrolls: the badge reader panel sits above Peppol.
            compose.onNodeWithTag("peppol_section").performScrollTo().assertIsDisplayed()
            advanceTimeBy(PEPPOL_POLL_MS + 1L)
            settle()
            coVerify(exactly = 2) { env.f.engine.listPeppolDeliveries(100L) }
        }

    @Test
    fun aCashierNeverTriggersThePeppolReads() =
        runTest {
            val env = signedIn(cashier)
            compose.setContent { SettingsRoute(navigator, Lazy { registry }, env.entry) }
            settle()
            advanceTimeBy(PEPPOL_POLL_MS * 3L)
            settle()

            coVerify(exactly = 0) { env.f.engine.listPeppolDeliveries(any()) }
            compose.onNodeWithTag("peppol_section").assertDoesNotExist()
        }

    @Test
    fun leavingTheScreenStopsThePolling() =
        runTest {
            val env = signedIn(admin)
            var shown by mutableStateOf(true)
            compose.setContent { if (shown) SettingsRoute(navigator, Lazy { registry }, env.entry) }
            settle()
            coVerify(exactly = 1) { env.f.engine.listPeppolDeliveries(any()) }

            // The route is popped: the view models stay in the activity here, but nothing keeps reading.
            shown = false
            settle()
            advanceTimeBy(PEPPOL_POLL_MS * 3L)
            runCurrent()
            coVerify(exactly = 1) { env.f.engine.listPeppolDeliveries(any()) }
        }

    @Test
    fun aManagerSeesTheCarteRowAndATapOpensTheMenuEditorRoute() =
        pollingTest {
            val env = signedIn(admin)
            compose.setContent { SettingsRoute(navigator, Lazy { registry }, env.entry) }
            settle()

            compose.onNodeWithTag("settings_row_settings.menu").assertIsDisplayed()
            compose.onNodeWithTag("settings_row_settings.menu").performClick()

            assertEquals(listOf("navigate:" + SettingsRoutes.MENU), navigator.calls)
        }

    @Test
    fun aCashierDoesNotSeeTheCarteRow() =
        runTest {
            val env = signedIn(cashier)
            compose.setContent { SettingsRoute(navigator, Lazy { registry }, env.entry) }
            settle()

            compose.onNodeWithTag("settings_row_a.open").assertIsDisplayed()
            compose.onNodeWithTag("settings_row_settings.menu").assertDoesNotExist()
        }

    private companion object {
        const val SETTLE_ROUNDS = 4
    }
}
