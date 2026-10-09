package dev.nuagemagique.pilipos.platform.about

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.nav.AppNavHost
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FakeEntry
import dev.nuagemagique.pilipos.nav.registryOf
import dev.nuagemagique.pilipos.platform.PlatformEntry
import dev.nuagemagique.pilipos.platform.PlatformRoutes
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The route `platform/about` renders [AboutScreen], with the provider of the Hilt graph. */
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class AboutRouteTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    @Test
    fun theAboutRouteRendersTheAboutScreen() {
        hiltRule.inject()
        lateinit var navigator: AppNavigator
        val registry = registryOf(setOf(FakeEntry("login", "login"), PlatformEntry()))
        compose.setContent { AppNavHost(registry) { navigator = it } }
        compose.waitForIdle()
        compose.runOnUiThread { navigator.navigate(PlatformRoutes.ABOUT) }
        compose.onNodeWithTag("about_root").assertIsDisplayed()
    }
}
