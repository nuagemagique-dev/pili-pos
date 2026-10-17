package dev.nuagemagique.pilipos.ui.badge

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.EventFeed
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
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
import uniffi.pos_ffi.ActiveBadge
import uniffi.pos_ffi.BadgeLink
import uniffi.pos_ffi.BadgeOutcomeKind
import uniffi.pos_ffi.BadgeOverview
import uniffi.pos_ffi.BadgeSettingsInput
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole

private const val CASHIER = "90020200234"

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class BadgeStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(reader: BadgeSettingsFixture = BadgeSettingsFixture()): EngineFixture {
        val f = EngineFixture(this, tmp.root)
        Dispatchers.setMain(f.dispatcher)
        coEvery { f.engine.getBadgeSettings() } returns reader.view
        coEvery { f.engine.listBadges() } returns noBadges()
        coEvery { f.engine.getSettings() } returns uniffiSettings()
        return f
    }

    private class BadgeSettingsFixture(
        val view: uniffi.pos_ffi.BadgeSettingsView = badgeView(),
    )

    private fun uniffiSettings() =
        uniffi.pos_ffi.PosSettings(
            trainingMode = false,
            printerAddress = null,
            fdmUrl = "http://fdm",
            posId = "POS1",
            terminalId = "T1",
            restaurantName = "Chez Pili",
            language = "fr",
            configProblems = emptyList(),
            posSwVersion = "pili-pos-android/test",
        )

    /** The store over the stubbed engine, opened so that it reads the setting. */
    private suspend fun TestScope.start(
        reader: uniffi.pos_ffi.BadgeSettingsView = badgeView(),
        support: FakeNfcSupport = FakeNfcSupport(),
        open: Boolean = true,
        arrange: (EngineFixture) -> Unit = {},
    ): Pair<BadgeRig, EngineFixture> {
        val f = fixture(BadgeSettingsFixture(reader))
        arrange(f)
        val rig = badgeRig(f, context, support)
        if (open) {
            f.holder.open()
            runCurrent()
        }
        return rig to f
    }

    private val cashier = User(insz = CASHIER, name = "Bob", role = UserRole.CASHIER, active = true, hasPin = true)

    // ── The source follows the setting ────────────────────────────

    @Test
    fun readerOffStartsNoSource() =
        runTest {
            val (r, _) = start(badgeView(reader = "off"))

            assertFalse(r.usb.calls.contains("start"))
            assertFalse(r.nfc.calls.contains("start"))
            assertFalse(r.store.readerOn)
        }

    @Test
    fun beforeTheEngineAnswersNoSourceIsStarted() =
        runTest {
            val (r, _) = start(open = false)

            assertTrue(r.usb.calls.isEmpty())
            assertTrue(r.nfc.calls.isEmpty())
            assertNull(r.store.settings.value)
        }

    @Test
    fun usbStartsTheWedgeWithTheConfiguredTiming() =
        runTest {
            val (r, _) = start(badgeView(reader = "usb", gap = 80L, min = 5L, max = 20L))

            assertEquals("start", r.usb.calls.last())
            assertEquals(WedgeConfig(80L, 5, 20), r.usb.lastConfig)
            assertFalse(r.nfc.calls.contains("start"))
            assertTrue(r.store.readerOn)
        }

    @Test
    fun nfcStartsTheNfcSourceWhenTheDeviceHasNfc() =
        runTest {
            val (r, _) = start(badgeView(reader = "nfc"), FakeNfcSupport(hasNfc = true))

            assertEquals("start", r.nfc.calls.last())
            assertFalse(r.usb.calls.contains("start"))
        }

    @Test
    fun nfcOnADeviceWithoutNfcReadsNothing() =
        runTest {
            val (r, _) = start(badgeView(reader = "nfc"), FakeNfcSupport(hasNfc = false))

            assertFalse(r.nfc.calls.contains("start"))
            assertFalse(r.usb.calls.contains("start"))
            assertFalse(r.store.hasNfc)
        }

    @Test
    fun changingTheReaderSwitchesTheSource() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.setBadgeSettings(any()) } returns badgeView(reader = "nfc")

            r.store.save(BadgeSettingsInput("nfc", false, 50L, 6L, 32L))

            assertEquals("stop", r.usb.calls.last())
            assertEquals("start", r.nfc.calls.last())
            assertEquals("nfc", r.store.settings.value?.reader)
        }

    @Test
    fun turningTheReaderOffStopsBoth() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.setBadgeSettings(any()) } returns badgeView(reader = "off")

            r.store.save(BadgeSettingsInput("off", false, 50L, 6L, 32L))

            assertEquals("stop", r.usb.calls.last())
            assertEquals("stop", r.nfc.calls.last())
        }

    @Test
    fun loginIsOnOnlyWithAReaderAndTheSwitch() =
        runTest {
            val (r, _) = start(badgeView(reader = "usb", loginEnabled = true))
            assertTrue(r.store.loginOn)
        }

    // ── What a scan does ──────────────────────────────────────────

    @Test
    fun aScanFromTheSourceGoesToTheEngine() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("04A2", false) } returns outcome(BadgeOutcomeKind.IGNORED)

            r.usb.scan("04A2")
            runCurrent()

            coVerify(exactly = 1) { f.engine.badgeTap("04A2", false) }
            assertTrue(r.toasts.toasts.value.isEmpty())
            assertTrue(r.cue.played.isEmpty())
        }

    @Test
    fun theNfcSourceScansToo() =
        runTest {
            val (r, f) = start(badgeView(reader = "nfc"))
            coEvery { f.engine.badgeTap("0A0B0C0D", false) } returns outcome(BadgeOutcomeKind.IGNORED)

            r.nfc.scan("0A0B0C0D")
            runCurrent()

            coVerify(exactly = 1) { f.engine.badgeTap("0A0B0C0D", false) }
        }

    @Test
    fun clockInOutcomeShowsTheToast() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("112233", false) } returns
                outcome(BadgeOutcomeKind.CLOCK_IN, CASHIER, "Bob", at = "2026-10-17T14:02:00Z")

            r.store.onScan("112233")

            val toast = r.toasts.toasts.value.single()
            assertEquals(r.catalog.text("badge_clock_in", AppLanguage.FR, mapOf("name" to "Bob", "time" to "16:02")), toast.message)
            assertTrue(toast.message.contains("Bob") && toast.message.contains("16:02"))
            assertEquals(ToastVariant.Success, toast.variant)
            assertEquals(listOf("ok"), r.cue.played)
            assertEquals(1, r.store.presenceVersion.value)
        }

    @Test
    fun clockOutOutcomeShowsTheDepartureToast() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("112233", false) } returns
                outcome(BadgeOutcomeKind.CLOCK_OUT, CASHIER, "Bob", at = "2026-10-17T22:30:00Z")

            r.store.onScan("112233")

            val toast = r.toasts.toasts.value.single()
            assertEquals(r.catalog.text("badge_clock_out", AppLanguage.FR, mapOf("name" to "Bob", "time" to "00:30")), toast.message)
            assertEquals(listOf("ok"), r.cue.played)
        }

    @Test
    fun presenceFailedShowsTheRefusalToast() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("112233", false) } returns
                outcome(BadgeOutcomeKind.PRESENCE_FAILED, CASHIER, "Bob", error = "FDM unreachable")

            r.store.onScan("112233")

            val toast = r.toasts.toasts.value.single()
            assertEquals(ToastVariant.Error, toast.variant)
            assertTrue(toast.message.contains("Bob"))
            assertTrue(toast.message.startsWith("Pointage refusé"))
            // The FDM's own text, as the FDM message table words it, reaches the toast (L5-D8).
            val fdmText = FdmTexts(r.catalog).describe("FDM unreachable", AppLanguage.FR)!!.french
            assertTrue(fdmText.isNotBlank())
            assertTrue(toast.message, toast.message.contains(fdmText))
            assertEquals(listOf("refused"), r.cue.played)
            assertEquals(0, r.store.presenceVersion.value)
        }

    @Test
    fun presenceFailedWithAKnownCodeShowsItsTextNotTheCode() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("112233", false) } returns
                outcome(BadgeOutcomeKind.PRESENCE_FAILED, CASHIER, "Bob", error = "previous_day_open")

            r.store.onScan("112233")

            val toast = r.toasts.toasts.value.single()
            assertEquals(ToastVariant.Error, toast.variant)
            assertTrue(toast.message, toast.message.contains(r.catalog.text("err_previous_day_open", AppLanguage.FR)))
            assertFalse(toast.message, toast.message.contains("previous_day_open"))
            assertEquals(listOf("refused"), r.cue.played)
            assertEquals(0, r.store.presenceVersion.value)
        }

    // ── The setting follows the engine ────────────────────────────

    @Test
    fun aResyncReadsTheSettingAgain() =
        runTest {
            lateinit var feed: EventFeed
            val (r, f) = start(badgeView(reader = "off")) { feed = EventFeed(it) }
            assertFalse(r.usb.calls.contains("start"))
            coEvery { f.engine.getBadgeSettings() } returns badgeView(reader = "usb")

            feed.send(PosEvent.Resync(missed = 1L))
            runCurrent()

            assertEquals("usb", r.store.settings.value?.reader)
            assertEquals("start", r.usb.calls.last())
        }

    @Test
    fun aFailedReadKeepsThePreviousSettingAndTheReaderRunning() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            assertEquals("start", r.usb.calls.last())
            val callsBefore = r.usb.calls.toList()
            coEvery { f.engine.getBadgeSettings() } throws PosException.Coded("engine_unavailable", emptyMap())

            r.store.load()

            assertEquals("usb", r.store.settings.value?.reader)
            assertEquals(callsBefore, r.usb.calls.toList())
            assertFalse(r.usb.calls.contains("stop"))
        }

    @Test
    fun loggedInOpensTheSessionAndAnnouncesIt() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb", loginEnabled = true))
            coEvery { f.engine.badgeTap("AABBCC", false) } returns outcome(BadgeOutcomeKind.LOGGED_IN, CASHIER, "Bob", user = cashier)
            val arrived = mutableListOf<User>()
            backgroundScope.launch { r.store.loggedIn.toList(arrived) }
            runCurrent()

            r.store.onScan("AABBCC")
            runCurrent()

            assertEquals(cashier, r.session.currentUser.value)
            assertTrue(r.session.canTakePayment.value)
            assertFalse(r.session.isManager.value)
            assertEquals(listOf(cashier), arrived)
            assertEquals(listOf("ok"), r.cue.played)
        }

    @Test
    fun pinRequiredAsksForThatManagersPin() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb", loginEnabled = true))
            coEvery { f.engine.badgeTap("AABBCC", false) } returns outcome(BadgeOutcomeKind.PIN_REQUIRED, "85010100123", "Alice")
            val requests = mutableListOf<PinRequest>()
            backgroundScope.launch { r.store.pinRequests.toList(requests) }
            runCurrent()

            r.store.onScan("AABBCC")
            runCurrent()

            assertEquals(listOf(PinRequest("85010100123", "Alice")), requests)
            assertNull(r.session.currentUser.value)
            assertTrue(r.cue.played.isEmpty())
        }

    @Test
    fun aRefusalShowsItsTextAndTheLowCue() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("ZZ", false) } throws PosException.Coded("badge_unknown", emptyMap())

            r.store.onScan("ZZ")

            val toast = r.toasts.toasts.value.single()
            assertEquals(ToastVariant.Error, toast.variant)
            assertEquals(r.catalog.text("err_badge_unknown", AppLanguage.FR), toast.message)
            assertEquals(listOf("refused"), r.cue.played)
        }

    @Test
    fun everyRefusalCodeHasItsOwnText() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            val codes = listOf("badge_reader_off", "badge_token_invalid", "badge_key_unavailable", "badge_unknown", "badge_revoked")
            for (code in codes) {
                coEvery { f.engine.badgeTap("ZZ", false) } throws PosException.Coded(code, emptyMap())
                r.store.onScan("ZZ")
            }
            assertEquals(codes.map { r.catalog.text("err_$it", AppLanguage.FR) }, r.toasts.toasts.value.map { it.message })
        }

    @Test
    fun endShiftModeIsSentWithTheScan() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("112233", true) } returns
                outcome(BadgeOutcomeKind.CLOCK_OUT, CASHIER, "Bob", at = "2026-10-17T14:02:00Z")
            r.store.setEndShiftMode(true)

            r.store.onScan("112233")

            coVerify { f.engine.badgeTap("112233", true) }
            assertTrue(r.store.endShiftMode.value)
            r.store.setEndShiftMode(false)
            assertFalse(r.store.endShiftMode.value)
        }

    @Test
    fun theEngineAwayGivesAnErrorToast() =
        runTest {
            val (r, _) = start(badgeView(reader = "usb"), open = false)

            r.store.onScan("112233")

            assertEquals(ToastVariant.Error, r.toasts.toasts.value.single().variant)
            assertEquals(listOf("refused"), r.cue.played)
        }

    // ── Enrolment capture ─────────────────────────────────────────

    @Test
    fun captureRoutingTakesTheNextScanInsteadOfTheEngine() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            val capture = r.store.startCapture()
            assertTrue(r.store.capturing.value)

            r.usb.scan("CAFE01")
            runCurrent()

            assertEquals("CAFE01", capture.token.await())
            assertFalse(r.store.capturing.value)
            coVerify(exactly = 0) { f.engine.badgeTap(any(), any()) }
            assertTrue(r.toasts.toasts.value.isEmpty())
        }

    @Test
    fun afterTheCaptureTheNextScanIsAnOrdinaryScan() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("0002", false) } returns outcome(BadgeOutcomeKind.IGNORED)
            val capture = r.store.startCapture()
            r.store.onScan("0001")
            assertEquals("0001", capture.token.await())

            r.store.onScan("0002")

            coVerify(exactly = 1) { f.engine.badgeTap("0002", false) }
        }

    @Test
    fun aCancelledCaptureLetsTheScanGoToTheEngine() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.badgeTap("0003", false) } returns outcome(BadgeOutcomeKind.IGNORED)
            r.store.startCapture().cancel()
            assertFalse(r.store.capturing.value)

            r.store.onScan("0003")

            coVerify(exactly = 1) { f.engine.badgeTap("0003", false) }
        }

    // ── Settings, link, revoke ────────────────────────────────────

    @Test
    fun savingKeepsTheEnginesAnswerAndReadsTheListAgain() =
        runTest {
            val (r, f) = start(badgeView(reader = "off"))
            coEvery { f.engine.setBadgeSettings(any()) } returns badgeView(reader = "usb", loginEnabled = true)
            coEvery { f.engine.listBadges() } returns
                BadgeOverview(listOf(ActiveBadge(CASHIER, "Bob", "2026-10-17 10:00:00", "85010100123", "Alice")), emptyList())

            r.store.save(BadgeSettingsInput("usb", true, 50L, 6L, 32L))

            assertEquals("usb", r.store.settings.value?.reader)
            assertEquals(1, r.store.overview.value?.badges?.size)
        }

    @Test
    fun aRefusedSavePropagatesAndKeepsTheSetting() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.setBadgeSettings(any()) } throws PosException.Coded("badge_settings_invalid", emptyMap())

            val failure = runCatching { r.store.save(BadgeSettingsInput("usb", false, 5L, 6L, 32L)) }.exceptionOrNull()

            assertTrue(failure is PosException.Coded)
            assertEquals("usb", r.store.settings.value?.reader)
        }

    @Test
    fun linkSendsTheCapturedTokenWithTheManagersPin() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.linkBadge(CASHIER, "CAFE01", "1234") } returns BadgeLink(CASHIER, "Bob", "2026-10-17 10:00:00")

            val link = r.store.link(CASHIER, "CAFE01", "1234")

            assertEquals("Bob", link.name)
            coVerify { f.engine.linkBadge(CASHIER, "CAFE01", "1234") }
        }

    @Test
    fun aRefusedLinkPropagates() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.linkBadge(any(), any(), any()) } throws PosException.Coded("badge_in_use", emptyMap())

            val failure = runCatching { r.store.link(CASHIER, "CAFE01", "1234") }.exceptionOrNull()

            assertEquals("badge_in_use", (failure as PosException.Coded).code)
        }

    @Test
    fun revokeCallsTheEngineAndReadsTheListAgain() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.revokeBadge(CASHIER) } returns Unit

            r.store.revoke(CASHIER)

            coVerify { f.engine.revokeBadge(CASHIER) }
            assertNotNull(r.store.overview.value)
        }

    @Test
    fun anOverviewForAnotherRoleIsNull() =
        runTest {
            val (r, f) = start(badgeView(reader = "usb"))
            coEvery { f.engine.listBadges() } throws PosException.Coded("forbidden", emptyMap())

            r.store.loadOverview()

            assertNull(r.store.overview.value)
        }
}
