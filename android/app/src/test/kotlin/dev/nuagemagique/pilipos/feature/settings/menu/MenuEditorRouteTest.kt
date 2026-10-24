package dev.nuagemagique.pilipos.feature.settings.menu

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ApplicationProvider
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.feature.settings.SettingsEnv
import dev.nuagemagique.pilipos.feature.settings.admin
import dev.nuagemagique.pilipos.feature.settings.cashier
import dev.nuagemagique.pilipos.feature.settings.openEngine
import dev.nuagemagique.pilipos.feature.settings.settingsEnv
import dev.nuagemagique.pilipos.feature.settings.settingsFixture
import dev.nuagemagique.pilipos.feature.tables.RecordingNavigator
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
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

/** The `settings/menu` route over a signed-in operator and a mocked engine. */
@OptIn(ExperimentalCoroutinesApi::class)
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class, qualifiers = "w1920dp-h1080dp-mdpi")
class MenuEditorRouteTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()
    private val tmp = TemporaryFolder()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(tmp).around(compose)

    private val context = ApplicationProvider.getApplicationContext<android.content.Context>()
    private val navigator = RecordingNavigator()

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
        coEvery { f.engine.menuAdminGet() } returns adminMenu()
        coEvery { f.engine.listMenuChanges(any()) } returns listOf(menuChange(2L), menuChange(1L, action = "create"))
        val env = settingsEnv(f, context)
        openEngine(env)
        with(env) { loginAs(who) }
        return env
    }

    @Test
    fun aManagerLoadsTheMenuOnceAndSeesItsCategories() =
        runTest {
            val env = signedIn(admin)
            compose.setContent { MenuEditorRoute(navigator, env.entry) }
            settle()

            coVerify(exactly = 1) { env.f.engine.menuAdminGet() }
            compose.onNodeWithTag("menu_ed_category_1").assertIsDisplayed()
            compose.onNodeWithTag("menu_ed_category_2").assertIsDisplayed()
        }

    @Test
    fun theHistoryButtonOpensAndClosesTheHistory() =
        runTest {
            val env = signedIn(admin)
            compose.setContent { MenuEditorRoute(navigator, env.entry) }
            settle()

            compose.onNodeWithTag("menu_ed_history").performClick()
            settle()
            coVerify(exactly = 1) { env.f.engine.listMenuChanges(any()) }
            compose.onNodeWithTag("menu_ed_history_row_2").assertIsDisplayed()

            // The second tap is the toggle's other branch: the history closes, nothing is read again.
            compose.onNodeWithTag("menu_ed_history").performClick()
            settle()
            compose.onNodeWithTag("menu_ed_history_row_2").assertDoesNotExist()
            compose.onNodeWithTag("menu_ed_hint").assertIsDisplayed()
            coVerify(exactly = 1) { env.f.engine.listMenuChanges(any()) }
        }

    @Test
    fun retourGoesBackThroughTheNavigator() =
        runTest {
            val env = signedIn(admin)
            compose.setContent { MenuEditorRoute(navigator, env.entry) }
            settle()

            compose.onNodeWithTag("menu_ed_back").performClick()

            assertEquals(listOf("back"), navigator.calls)
        }

    @Test
    fun aCashierNeverLoadsTheMenu() =
        runTest {
            val env = signedIn(cashier)
            compose.setContent { MenuEditorRoute(navigator, env.entry) }
            settle()

            coVerify(exactly = 0) { env.f.engine.menuAdminGet() }
            compose.onNodeWithTag("menu_ed_category_1").assertDoesNotExist()
        }

    private companion object {
        const val SETTLE_ROUNDS = 4
    }
}
