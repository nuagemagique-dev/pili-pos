package dev.nuagemagique.pilipos.nav

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.feature.checkout.CheckoutRoutes
import dev.nuagemagique.pilipos.feature.login.LoginRoutes
import dev.nuagemagique.pilipos.feature.order.OrderRoutes
import dev.nuagemagique.pilipos.feature.reports.ReportsRoutes
import dev.nuagemagique.pilipos.feature.settings.SettingsRoutes
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.feature.webshop.WebshopRoutes
import dev.nuagemagique.pilipos.platform.PlatformRoutes
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
class FeatureEntriesTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

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
    fun theSetHoldsSevenFeaturesAndThePlatform() {
        hiltRule.inject()
        assertEquals(
            setOf("login", "tables", "order", "checkout", "reports", "settings", "webshop", "platform"),
            entries.map { it.area }.toSet(),
        )
        assertEquals(8, entries.size)
    }

    @Test
    fun allRoutesAreUnique() {
        hiltRule.inject()
        val routes = entries.flatMap { it.routes }
        assertEquals(routes.size, routes.toSet().size)
        assertEquals(
            setOf(
                LoginRoutes.LOGIN,
                TablesRoutes.TABLES,
                OrderRoutes.ORDER,
                CheckoutRoutes.CHECKOUT,
                ReportsRoutes.REPORTS,
                SettingsRoutes.SETTINGS,
                WebshopRoutes.WEBSHOP,
                PlatformRoutes.ABOUT,
            ),
            routes.toSet(),
        )
    }

    @Test
    fun loginIsTheStartRoute() {
        assertEquals(NavContract.START_ROUTE, LoginRoutes.LOGIN)
    }

    @Test
    fun orderRoutesCarryTheId() {
        assertEquals("order/42", OrderRoutes.order(42))
        assertEquals("checkout/7", CheckoutRoutes.checkout(7))
    }

    @Test
    fun theAboutSettingsRowIsPresentAndTheRegistryConstructs() {
        hiltRule.inject()
        val about = settings.single { it.id == "platform.about" }
        assertEquals(PlatformRoutes.ABOUT, about.route)
        assertEquals(registry.settingsEntries, registry.settingsEntries.sortedWith(compareBy({ it.order }, { it.id })))
        assertTrue(registry.entries.size == 8)
    }

    @Test
    fun everyPlaceholderIsReachableAndSettingsListsTheRows() {
        hiltRule.inject()
        compose.setContent { AppNavHost(registry) { navigator = it } }
        compose.waitForIdle()
        compose.onNodeWithTag("placeholder_login").assertIsDisplayed()
        val targets =
            mapOf(
                TablesRoutes.TABLES to "placeholder_tables",
                OrderRoutes.order(5) to "placeholder_order",
                CheckoutRoutes.checkout(5) to "placeholder_checkout",
                ReportsRoutes.REPORTS to "placeholder_reports",
                WebshopRoutes.WEBSHOP to "placeholder_webshop",
                PlatformRoutes.ABOUT to "placeholder_platform",
                SettingsRoutes.SETTINGS to "placeholder_settings",
            )
        targets.forEach { (route, tag) ->
            compose.runOnUiThread { navigator.navigate(route) }
            compose.onNodeWithTag(tag).assertIsDisplayed()
        }
        compose.onNodeWithTag("settings_row_platform.about").assertIsDisplayed()
    }
}
