package dev.nuagemagique.pilipos.feature.settings

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import app.cash.turbine.test
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
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
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.ApplyOutcome
import uniffi.pos_ffi.AuthorizeLink
import uniffi.pos_ffi.Channel
import uniffi.pos_ffi.Issue
import uniffi.pos_ffi.IssueCode
import uniffi.pos_ffi.LockReason
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SessionLocked
import uniffi.pos_ffi.User

private const val LINK = "https://manager.hubrise.com/oauth2/v1/authorize?client_id=x"

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class HubriseViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: HubriseViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(
        who: User? = admin,
        enabled: Boolean = true,
        arrange: (SettingsEnv) -> Unit = {},
    ): Rig {
        val f = settingsFixture(tmp.root, hubrise = enabled)
        stubHubriseReads(f.engine)
        val env = settingsEnv(f, context)
        arrange(env)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = HubriseViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    private fun failure(code: String = "boom") = PosException.Coded(code, emptyMap())

    // ── Active or not ─────────────────────────────────────────────

    @Test
    fun aDisabledSwitchMakesThePanelInertAndCallsNothing() =
        runTest {
            val r = start(enabled = false)

            assertFalse(r.state.active)
            assertNull(r.state.status)
            coVerify(exactly = 0) { r.engine.hubriseGetConnectionStatus() }
            coVerify(exactly = 0) { r.engine.hubriseStatus() }

            r.model.connect()
            r.model.submitCode("abc")
            r.model.requestDisconnect()
            r.model.confirmDisconnect()
            r.model.syncCatalog()
            r.model.adoptCatalog()
            r.model.saveChannel(channel())
            r.model.setNewChannel("Uber")
            r.model.addChannel()
            runCurrent()

            assertFalse(r.state.confirmDisconnect)
            coVerify(exactly = 0) { r.engine.hubriseStartOauthFlow() }
            coVerify(exactly = 0) { r.engine.hubriseCompleteOauth(any()) }
            coVerify(exactly = 0) { r.engine.hubriseDisconnect() }
            coVerify(exactly = 0) { r.engine.hubriseSyncCatalog() }
            coVerify(exactly = 0) { r.engine.hubriseAdoptCatalog(any()) }
            coVerify(exactly = 0) { r.engine.hubriseSetChannel(any()) }
            r.model.load()
            coVerify(exactly = 0) { r.engine.hubriseStatus() }
        }

    @Test
    fun switchingOnLoadsThePanelAndSwitchingOffForgetsIt() =
        runTest {
            val r = start(enabled = false)
            coEvery { r.engine.setHubriseEnabled(true) } returns integrationsOf(true)

            r.env.integrations.setHubriseEnabled(true)
            runCurrent()

            assertTrue(r.state.active)
            assertTrue(r.state.connected)
            assertEquals(listOf(channel()), r.state.channels)

            coEvery { r.engine.setHubriseEnabled(false) } returns integrationsOf(false)
            r.env.integrations.setHubriseEnabled(false)
            runCurrent()

            assertFalse(r.state.active)
            assertNull(r.state.status)
            assertNull(r.state.health)
            assertTrue(r.state.channels.isEmpty())
        }

    @Test
    fun anEnabledSwitchLoadsTheStatusTheHealthAndTheChannels() =
        runTest {
            val r = start()

            assertTrue(r.state.active)
            assertTrue(r.state.isManager)
            assertEquals(connection(true), r.state.status)
            assertEquals(health(), r.state.health)
            assertEquals(listOf(channel()), r.state.channels)
            assertNull(r.state.error)
        }

    @Test
    fun aNonManagerGetsNeitherTheAdoptionNorTheChannels() =
        runTest {
            val r = start(who = cashier) { env -> coEvery { env.f.engine.hubriseStatus() } returns health(awaiting = true) }

            assertFalse(r.state.isManager)
            assertTrue(r.state.adoptions.isEmpty())
            assertTrue(r.state.channels.isEmpty())
            assertFalse(r.state.awaitingAdoption)
            coVerify(exactly = 0) { r.engine.hubriseAdoptionPreview() }
            coVerify(exactly = 0) { r.engine.hubriseListChannels() }
        }

    @Test
    fun aDisconnectedAccountHasNoChannelsToList() =
        runTest {
            val r =
                start { env ->
                    coEvery { env.f.engine.hubriseGetConnectionStatus() } returns connection(false)
                    coEvery { env.f.engine.hubriseStatus() } returns health(connected = false)
                }

            assertFalse(r.state.connected)
            coVerify(exactly = 0) { r.engine.hubriseListChannels() }
        }

    @Test
    fun aFailingStatusShowsTheErrorAndAFailingHealthStopsThere() =
        runTest {
            val r =
                start { env ->
                    coEvery { env.f.engine.hubriseGetConnectionStatus() } throws failure()
                    coEvery { env.f.engine.hubriseStatus() } throws failure("other")
                }

            assertNotNull(r.state.error)
            assertNull(r.state.status)
            assertNull(r.state.health)
            coVerify(exactly = 0) { r.engine.hubriseListChannels() }
        }

    // ── Connect ───────────────────────────────────────────────────

    @Test
    fun connectKeepsTheLinkForTheQrCode() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseStartOauthFlow() } returns AuthorizeLink(url = LINK, qrSvg = "<svg/>")

            r.model.connect()
            runCurrent()

            assertEquals(LINK, r.state.authorizeUrl)
            assertFalse(r.state.loading)
            assertNull(r.state.error)
        }

    @Test
    fun aFailedConnectShowsTheErrorAndNoLink() =
        runTest {
            val r = start()
            val refusal = failure("hubrise_unavailable")
            coEvery { r.engine.hubriseStartOauthFlow() } throws refusal

            r.model.connect()
            runCurrent()

            assertNull(r.state.authorizeUrl)
            assertEquals(r.env.describe(refusal), r.state.error)
            assertFalse(r.state.loading)
        }

    @Test
    fun theTypedCodeIsTrimmedAndConnectsTheAccount() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseStartOauthFlow() } returns AuthorizeLink(LINK, "")
            coEvery { r.engine.hubriseCompleteOauth("code-123") } returns connection(true)
            r.model.connect()
            runCurrent()

            r.model.submitCode("  code-123 \n")
            runCurrent()

            coVerify(exactly = 1) { r.engine.hubriseCompleteOauth("code-123") }
            assertNull(r.state.authorizeUrl)
            assertTrue(r.state.connected)
            assertFalse(r.state.loading)
            // The connection fetched the catalog: the health is read again.
            coVerify(exactly = 2) { r.engine.hubriseStatus() }
        }

    @Test
    fun aBlankCodeIsIgnored() =
        runTest {
            val r = start()

            r.model.submitCode("   ")
            runCurrent()

            coVerify(exactly = 0) { r.engine.hubriseCompleteOauth(any()) }
        }

    @Test
    fun aRefusedCodeKeepsTheDialogOpenWithTheError() =
        runTest {
            val r = start()
            val refusal = failure("oauth_denied")
            coEvery { r.engine.hubriseStartOauthFlow() } returns AuthorizeLink(LINK, "")
            coEvery { r.engine.hubriseCompleteOauth("bad") } throws refusal
            r.model.connect()
            runCurrent()

            r.model.submitCode("bad")
            runCurrent()

            assertEquals(LINK, r.state.authorizeUrl)
            assertEquals(r.env.describe(refusal), r.state.error)
            assertFalse(r.state.loading)
        }

    @Test
    fun cancellingTheConnectDialogDropsTheLinkAndTheError() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseStartOauthFlow() } returns AuthorizeLink(LINK, "")
            coEvery { r.engine.hubriseCompleteOauth("bad") } throws failure()
            r.model.connect()
            runCurrent()
            r.model.submitCode("bad")
            runCurrent()

            r.model.cancelConnect()

            assertNull(r.state.authorizeUrl)
            assertNull(r.state.error)
        }

    // ── Disconnect ────────────────────────────────────────────────

    @Test
    fun disconnectAsksForAConfirmationFirst() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseDisconnect() } returns Unit

            r.model.confirmDisconnect()
            runCurrent()
            coVerify(exactly = 0) { r.engine.hubriseDisconnect() }

            r.model.requestDisconnect()
            assertTrue(r.state.confirmDisconnect)
            r.model.cancelDisconnect()
            assertFalse(r.state.confirmDisconnect)
            coVerify(exactly = 0) { r.engine.hubriseDisconnect() }
        }

    @Test
    fun aConfirmedDisconnectCallsTheEngineAndReadsTheStatusAgain() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseDisconnect() } returns Unit
            coEvery { r.engine.hubriseGetConnectionStatus() } returns connection(false)

            r.model.requestDisconnect()
            r.model.confirmDisconnect()
            runCurrent()

            coVerify(exactly = 1) { r.engine.hubriseDisconnect() }
            assertFalse(r.state.connected)
            assertFalse(r.state.confirmDisconnect)
            assertFalse(r.state.loading)
        }

    @Test
    fun aRefusedDisconnectShowsTheError() =
        runTest {
            val r = start()
            val refusal = failure("orders_open")
            coEvery { r.engine.hubriseDisconnect() } throws refusal

            r.model.requestDisconnect()
            r.model.confirmDisconnect()
            runCurrent()

            assertEquals(r.env.describe(refusal), r.state.error)
            assertTrue(r.state.connected)
            assertFalse(r.state.loading)
        }

    // ── Catalog ───────────────────────────────────────────────────

    @Test
    fun syncingTheCatalogShowsASuccessToastAndReadsTheHealthAgain() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseSyncCatalog() } returns applied

            r.model.syncCatalog()
            runCurrent()

            coVerify(exactly = 1) { r.engine.hubriseSyncCatalog() }
            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
            assertEquals(r.env.text("hubrise_catalog_sync_ok"), r.env.lastToast().message)
            assertFalse(r.state.catalogBusy)
            coVerify(exactly = 2) { r.engine.hubriseStatus() }
        }

    @Test
    fun aSyncWithNoAnswerCountsAsDone() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseSyncCatalog() } returns null

            r.model.syncCatalog()
            runCurrent()

            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
        }

    @Test
    fun aRefusedCatalogIsAnErrorToastNotAnException() =
        runTest {
            val r = start()
            coEvery { r.engine.hubriseSyncCatalog() } returns ApplyOutcome.Rejected(listOf(Issue(IssueCode.EMPTY_KEY, "")))

            r.model.syncCatalog()
            runCurrent()

            assertEquals(ToastVariant.Error, r.env.lastToast().variant)
            assertEquals(r.env.text("hubrise_catalog_rejected"), r.env.lastToast().message)
            assertNull(r.state.error)
        }

    @Test
    fun aFailedSyncShowsTheError() =
        runTest {
            val r = start()
            val refusal = failure("offline")
            coEvery { r.engine.hubriseSyncCatalog() } throws refusal

            r.model.syncCatalog()
            runCurrent()

            assertEquals(r.env.describe(refusal), r.state.error)
            assertFalse(r.state.catalogBusy)
        }

    @Test
    fun theAdoptionPreviewIsTickedByDefaultAndToggles() =
        runTest {
            val r =
                start { env ->
                    coEvery { env.f.engine.hubriseStatus() } returns health(awaiting = true)
                    coEvery { env.f.engine.hubriseAdoptionPreview() } returns listOf(adoption(1L), adoption(2L), adoption(3L))
                }

            assertTrue(r.state.awaitingAdoption)
            assertEquals(setOf(1L, 2L, 3L), r.state.adopted)

            r.model.toggleAdoption(2L)
            assertEquals(listOf(1L, 3L), r.state.adoptedList.map { it.menuItemId })

            r.model.toggleAdoption(2L)
            assertEquals(setOf(1L, 2L, 3L), r.state.adopted)
        }

    @Test
    fun adoptingSendsOnlyTheTickedProductsAndClearsThePreview() =
        runTest {
            val one = adoption(1L)
            val two = adoption(2L)
            val r =
                start { env ->
                    coEvery { env.f.engine.hubriseStatus() } returns health(awaiting = true)
                    coEvery { env.f.engine.hubriseAdoptionPreview() } returns listOf(one, two)
                }
            coEvery { r.engine.hubriseAdoptCatalog(listOf(one)) } returns applied
            coEvery { r.engine.hubriseStatus() } returns health(awaiting = false)
            r.model.toggleAdoption(2L)

            r.model.adoptCatalog()
            runCurrent()

            coVerify(exactly = 1) { r.engine.hubriseAdoptCatalog(listOf(one)) }
            assertTrue(r.state.adoptions.isEmpty())
            assertTrue(r.state.adopted.isEmpty())
            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
            assertFalse(r.state.awaitingAdoption)
            assertFalse(r.state.catalogBusy)
        }

    @Test
    fun aRefusedAdoptionKeepsThePreviewAndShowsTheError() =
        runTest {
            val one = adoption(1L)
            val r =
                start { env ->
                    coEvery { env.f.engine.hubriseStatus() } returns health(awaiting = true)
                    coEvery { env.f.engine.hubriseAdoptionPreview() } returns listOf(one)
                }
            val refusal = failure("adopt_failed")
            coEvery { r.engine.hubriseAdoptCatalog(any()) } throws refusal

            r.model.adoptCatalog()
            runCurrent()

            assertEquals(r.env.describe(refusal), r.state.error)
            assertEquals(listOf(one), r.state.adoptions)
        }

    @Test
    fun aFailingPreviewShowsTheError() =
        runTest {
            val r =
                start { env ->
                    coEvery { env.f.engine.hubriseStatus() } returns health(awaiting = true)
                    coEvery { env.f.engine.hubriseAdoptionPreview() } throws failure("preview")
                }

            assertNotNull(r.state.error)
            assertTrue(r.state.adoptions.isEmpty())
        }

    @Test
    fun theCatalogRefusedFlagComparesTheTwoDates() {
        fun refused(
            applied: String?,
            rejected: String?,
        ) = HubriseUiState(health = health(catalog = syncState(appliedAt = applied, rejectedAt = rejected))).catalogRefused

        assertFalse(HubriseUiState().catalogRefused)
        assertFalse(HubriseUiState(health = health()).catalogRefused)
        assertFalse(refused(applied = "2026-10-09 10:00:00", rejected = null))
        assertTrue(refused(applied = null, rejected = "2026-10-09 10:00:00"))
        assertTrue(refused(applied = "2026-10-09 09:00:00", rejected = "2026-10-09 10:00:00"))
        assertTrue(refused(applied = "2026-10-09 10:00:00", rejected = "2026-10-09 10:00:00"))
        assertFalse(refused(applied = "2026-10-09 11:00:00", rejected = "2026-10-09 10:00:00"))
    }

    @Test
    fun theLastSyncIsTheHealthTimeThenTheAppliedTimeThenNever() {
        val none = HubriseUiState(health = health())
        val applied = HubriseUiState(health = health(catalog = syncState(appliedAt = "2026-10-08 10:00:00")))
        val synced =
            HubriseUiState(
                health = health(lastCatalogSyncAt = "2026-10-09 10:00:00", catalog = syncState(appliedAt = "2026-10-08 10:00:00")),
            )

        assertNull(HubriseUiState().catalogSyncedAt)
        assertNull(none.catalogSyncedAt)
        assertEquals("2026-10-08 10:00:00", applied.catalogSyncedAt)
        assertEquals("2026-10-09 10:00:00", synced.catalogSyncedAt)
    }

    // ── Channels ──────────────────────────────────────────────────

    @Test
    fun savingAChannelToastsTheReleasedOrdersSendsAnEffectAndReloads() =
        runTest {
            val r = start()
            val web = channel("Shop", kind = ChannelKind.WEBSHOP, payment = ChannelPayment.ONLINE)
            coEvery { r.engine.hubriseSetChannel(web) } returns 3L

            r.model.effects.test {
                r.model.saveChannel(web)
                runCurrent()

                assertEquals(HubriseEffect.ChannelSaved(3L), awaitItem())
            }

            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
            assertEquals(r.env.text("hubrise_channel_saved", mapOf("count" to 3L)), r.env.lastToast().message)
            coVerify(exactly = 2) { r.engine.hubriseListChannels() }
        }

    @Test
    fun aRefusedChannelShowsTheErrorAndReloadsTheList() =
        runTest {
            val r = start()
            val refusal = failure("bad_channel")
            coEvery { r.engine.hubriseSetChannel(any()) } throws refusal

            r.model.saveChannel(channel("X"))
            runCurrent()

            assertEquals(r.env.describe(refusal), r.state.error)
            coVerify(exactly = 2) { r.engine.hubriseListChannels() }
        }

    @Test
    fun aNewChannelIsAPlatformPaidByAppWithoutTheServiceCharge() =
        runTest {
            val r = start()
            val expected = Channel("Uber Eats", "platform", "APP", "exclude")
            coEvery { r.engine.hubriseSetChannel(expected) } returns 0L

            r.model.setNewChannel("  Uber Eats ")
            assertEquals("  Uber Eats ", r.state.newChannel)
            r.model.addChannel()
            runCurrent()

            coVerify(exactly = 1) { r.engine.hubriseSetChannel(expected) }
            assertEquals("", r.state.newChannel)
        }

    @Test
    fun aBlankNewChannelIsIgnored() =
        runTest {
            val r = start()

            r.model.setNewChannel("   ")
            r.model.addChannel()
            runCurrent()

            coVerify(exactly = 0) { r.engine.hubriseSetChannel(any()) }
        }

    @Test
    fun changingAChannelSettingSavesTheChangedChannel() =
        runTest {
            val r = start()
            val base = channel("Deliveroo")
            coEvery { r.engine.hubriseSetChannel(any()) } returns 0L

            r.model.setChannelKind(base, ChannelKind.WEBSHOP)
            r.model.setChannelPayment(base, ChannelPayment.ONLINE)
            r.model.setChannelChargePolicy(base, ChargePolicy.INCLUDE)
            runCurrent()

            coVerify { r.engine.hubriseSetChannel(Channel("Deliveroo", "webshop", "ONLINE", "exclude")) }
            coVerify { r.engine.hubriseSetChannel(Channel("Deliveroo", "platform", "ONLINE", "exclude")) }
            coVerify { r.engine.hubriseSetChannel(Channel("Deliveroo", "platform", "APP", "include")) }
        }

    @Test
    fun theKindDecidesThePaymentType() {
        val platform = channel(kind = ChannelKind.WEBSHOP, payment = ChannelPayment.ONLINE).withKind(ChannelKind.PLATFORM)
        val webshop = channel().withKind(ChannelKind.WEBSHOP)

        assertEquals(ChannelPayment.APP, platform.paymentType)
        assertEquals(ChannelPayment.ONLINE, webshop.paymentType)
    }

    // ── Events ────────────────────────────────────────────────────

    @Test
    fun theCatalogEventsReloadTheHealth() =
        runTest {
            val r = start()
            val event = callbackEvent()

            r.env.feed.send(PosEvent.CatalogAwaitingAdoption(event))
            runCurrent()
            r.env.feed.send(PosEvent.CatalogRejected(event))
            runCurrent()
            r.env.feed.send(PosEvent.CatalogUpdated(event))
            runCurrent()

            coVerify(exactly = 4) { r.engine.hubriseStatus() }
            coVerify(exactly = 1) { r.engine.hubriseGetConnectionStatus() }
        }

    @Test
    fun aResyncReloadsTheStatusAndTheHealth() =
        runTest {
            val r = start()

            r.env.feed.send(PosEvent.Resync(missed = 2L))
            runCurrent()

            coVerify(exactly = 2) { r.engine.hubriseGetConnectionStatus() }
            coVerify(exactly = 2) { r.engine.hubriseStatus() }
        }

    @Test
    fun anOtherEventChangesNothing() =
        runTest {
            val r = start()

            r.env.feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, null)))
            runCurrent()

            coVerify(exactly = 1) { r.engine.hubriseStatus() }
        }

    @Test
    fun theEventsAreIgnoredWhileTheSwitchIsOff() =
        runTest {
            val r = start(enabled = false)

            r.env.feed.send(PosEvent.CatalogUpdated(callbackEvent()))
            r.env.feed.send(PosEvent.Resync(missed = 1L))
            runCurrent()

            coVerify(exactly = 0) { r.engine.hubriseStatus() }
            coVerify(exactly = 0) { r.engine.hubriseGetConnectionStatus() }
        }
}
