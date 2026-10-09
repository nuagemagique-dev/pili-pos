package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.Channel

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class HubriseSectionTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        HubriseCallbacks(
            onConnect = { calls += "connect" },
            onDisconnect = { calls += "disconnect" },
            onSync = { calls += "sync" },
            onToggleAdoption = { calls += "toggle:$it" },
            onAdopt = { calls += "adopt" },
            onChannelKind = { c, kind -> calls += "kind:${c.channel}:$kind" },
            onChannelCharge = { c, policy -> calls += "charge:${c.channel}:$policy" },
            onNewChannel = { calls += "new:$it" },
            onAddChannel = { calls += "addChannel" },
        )

    private fun text(
        key: String,
        vars: Map<String, Any> = emptyMap(),
    ): String = catalog.text(key, AppLanguage.FR, vars)

    private fun show(
        state: HubriseUiState,
        pending: Int = 0,
    ) {
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { HubriseSection(state, pending, callbacks) } }
        }
        compose.waitForIdle()
    }

    // ── Not connected ─────────────────────────────────────────────

    @Test
    fun notConnectedItShowsTheHelpAndTheConnectButtonOnly() {
        show(HubriseUiState(active = true, isManager = true, status = connection(false)))

        compose.onNodeWithTag("hubrise_status").assertTextEquals(text("hubrise_disconnected"))
        compose.onNodeWithTag("hubrise_pending").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_disconnect").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_channels").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_connect").assertTextContains(text("hubrise_open_authorize_url")).performClick()

        assertEquals(listOf("connect"), calls)
    }

    @Test
    fun theConnectButtonIsOffWhileAStepRuns() {
        show(HubriseUiState(active = true, status = connection(false), loading = true))

        compose.onNodeWithTag("hubrise_connect").assertIsNotEnabled()
    }

    @Test
    fun aRefusalShowsInThePanelWhenNoDialogIsOpen() {
        show(HubriseUiState(active = true, status = connection(false), error = "HubRise refuse"))

        compose.onNodeWithTag("hubrise_error").assertTextEquals("HubRise refuse")
    }

    @Test
    fun theRefusalIsLeftToTheDialogWhileItIsOpen() {
        show(HubriseUiState(active = true, status = connection(false), error = "HubRise refuse", authorizeUrl = "https://x"))

        compose.onNodeWithTag("hubrise_error").assertDoesNotExist()
    }

    // ── Connected ─────────────────────────────────────────────────

    @Test
    fun connectedItShowsTheAccountTheLocationAndThePendingCount() {
        show(connectedState(), pending = 3)

        compose.onNodeWithTag("hubrise_status").assertTextEquals(text("hubrise_connected"))
        compose.onNodeWithTag("hubrise_pending").assertTextEquals(text("settings_hubrise_pending", mapOf("count" to 3)))
        compose.onNodeWithText("Account").assertIsDisplayed()
        compose.onNodeWithText("Location").assertIsDisplayed()
    }

    @Test
    fun theDisconnectButtonAsksTheViewModelForAConfirmation() {
        show(connectedState())

        compose.onNodeWithTag("hubrise_disconnect").performClick()

        assertEquals(listOf("disconnect"), calls)
    }

    @Test
    fun theCatalogShowsWhenItWasSyncedInBrusselsTimeAndTheSyncButtonCallsBack() {
        show(connectedState())

        // 09:05 UTC is 11:05 in Brussels in October.
        compose.onNodeWithTag("hubrise_synced").assertTextEquals(text("hubrise_catalog_synced", mapOf("time" to "09/10 11:05")))
        compose.onNodeWithTag("hubrise_sync").performClick()

        assertEquals(listOf("sync"), calls)
    }

    @Test
    fun aNeverSyncedCatalogSaysSo() {
        show(connectedState().copy(health = health(lastCatalogSyncAt = null, catalog = null)))

        compose.onNodeWithTag("hubrise_synced").assertTextEquals(text("hubrise_catalog_never"))
    }

    @Test
    fun theSyncButtonIsOffWhileTheCatalogIsBusy() {
        show(connectedState().copy(catalogBusy = true))

        compose.onNodeWithTag("hubrise_sync").assertIsNotEnabled()
    }

    @Test
    fun aRefusedCatalogListsItsIssuesWithTheirKeys() {
        show(connectedState(refused = true))

        compose.onNodeWithTag("hubrise_refused").assertIsDisplayed()
        compose.onNodeWithText(text("hubrise_catalog_rejected")).assertIsDisplayed()
        compose.onNodeWithText(text("catalog_issue_empty_key")).assertIsDisplayed()
        compose.onNodeWithText(text("catalog_issue_missing_name") + " · sushi-42").assertIsDisplayed()
    }

    @Test
    fun anAppliedCatalogShowsNoRefusal() {
        show(connectedState())

        compose.onNodeWithTag("hubrise_refused").assertDoesNotExist()
    }

    @Test
    fun theManagedMenuBannerShowsWhenTheCatalogManagesTheMenu() {
        show(connectedState())

        compose.onNodeWithTag("hubrise_managed").assertTextEquals(text("hubrise_menu_managed_banner"))
    }

    @Test
    fun theHealthBannersShowOnlyWhatIsWrong() {
        val bad =
            health(catalog = syncState(appliedAt = "2026-10-09 09:05:00")).let {
                it.copy(
                    health = it.health.copy(authErrorAt = "2026-10-09 09:00:00"),
                    parkedEvents = 2L,
                    outbox = it.outbox.copy(pending = 4L, parked = 1L),
                    outboxError = "timeout",
                )
            }
        show(connectedState().copy(health = bad))

        compose.onNodeWithTag("hubrise_auth_error").assertTextEquals(text("hubrise_health_auth"))
        compose.onNodeWithTag("hubrise_parked").assertTextEquals(text("hubrise_health_parked", mapOf("count" to 3L)))
        compose.onNodeWithTag("hubrise_outbox").assertTextEquals(text("hubrise_health_outbox", mapOf("count" to 4L)))
    }

    @Test
    fun aHealthyConnectionShowsNoFaultBanner() {
        show(connectedState())

        compose.onNodeWithTag("hubrise_auth_error").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_parked").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_outbox").assertDoesNotExist()
    }

    // ── Adoption (manager) ────────────────────────────────────────

    @Test
    fun aManagerPicksTheProductsThatKeepTheirIdAndAdopts() {
        show(connectedState(adoptions = listOf(adoption(1L), adoption(2L))))

        compose.onNodeWithTag("hubrise_adoptions").assertIsDisplayed()
        compose.onNodeWithTag("adoption_1").assertTextContains("Item 1", substring = true)
        compose.onNodeWithTag("adoption_1").assertTextContains("1,00 €", substring = true)
        compose.onNodeWithTag("adoption_2").performClick()
        compose.onNodeWithTag("hubrise_adopt").performClick()

        assertEquals(listOf("toggle:2", "adopt"), calls)
    }

    @Test
    fun aTickedAdoptionIsMarkedAndAnUntickedOneIsNot() {
        show(connectedState(adoptions = listOf(adoption(1L), adoption(2L))).let { it.copy(adopted = setOf(1L)) })

        // Only the ticked box holds the check mark.
        compose.onNodeWithTag("adoption_1_check", useUnmergedTree = true).assertExists()
        compose.onNodeWithTag("adoption_2_check", useUnmergedTree = true).assertDoesNotExist()
    }

    @Test
    fun withoutMatchesTheAdoptionSaysSoAndStillOffersTheAction() {
        val state = connectedState().copy(health = health(awaiting = true, catalog = syncState()), adoptions = emptyList())
        show(state)

        compose.onNodeWithText(text("hubrise_adoption_none")).assertIsDisplayed()
        compose.onNodeWithTag("hubrise_adopt").assertIsEnabled()
    }

    @Test
    fun aNonManagerSeesNeitherTheAdoptionNorTheChannels() {
        show(connectedState(manager = false, adoptions = listOf(adoption(1L))))

        compose.onNodeWithTag("hubrise_adoptions").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_adopt").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_channels").assertDoesNotExist()
        compose.onNodeWithTag("hubrise_sync").assertIsDisplayed()
    }

    // ── Channels (manager) ────────────────────────────────────────

    @Test
    fun eachChannelMarksItsKindAndItsChargePolicy() {
        show(connectedState())

        compose.onNodeWithTag("channel_Deliveroo_kind_platform").assertIsSelected()
        compose.onNodeWithTag("channel_Deliveroo_charge_exclude").assertIsSelected()
        compose.onNodeWithTag("channel_Webshop_kind_webshop").assertIsSelected()
        compose.onNodeWithTag("channel_Webshop_charge_include").assertIsSelected()
    }

    @Test
    fun aKindOrChargeTapSavesTheChannelWithTheNewSetting() {
        show(connectedState())

        compose.onNodeWithTag("channel_Deliveroo_kind_webshop").performClick()
        compose.onNodeWithTag("channel_Webshop_charge_exclude").performClick()

        assertEquals(listOf("kind:Deliveroo:webshop", "charge:Webshop:exclude"), calls)
    }

    @Test
    fun theNewChannelFieldForwardsWhatIsTyped() {
        show(connectedState().copy(newChannel = ""))

        compose.onNodeWithTag("hubrise_new_channel").performTextInput("Uber")

        assertEquals(listOf("new:Uber"), calls)
    }

    @Test
    fun theAddChannelButtonCallsBackForANonBlankName() {
        show(connectedState().copy(newChannel = "Uber"))

        compose.onNodeWithTag("hubrise_add_channel").assertIsEnabled().performClick()

        assertEquals(listOf("addChannel"), calls)
    }

    @Test
    fun theAddChannelButtonIsOffForABlankName() {
        show(connectedState().copy(newChannel = "  "))

        compose.onNodeWithTag("hubrise_add_channel").assertIsNotEnabled()
    }

    @Test
    fun channelKindsMapToThePaymentTypeTheViewModelSaves() {
        val platform = Channel("X", ChannelKind.PLATFORM, ChannelPayment.APP, ChargePolicy.EXCLUDE)

        assertEquals(ChannelPayment.ONLINE, platform.withKind(ChannelKind.WEBSHOP).paymentType)
    }
}
