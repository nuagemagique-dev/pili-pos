package dev.nuagemagique.pilipos.feature.settings

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.ui.badge.FakeNfcSupport
import dev.nuagemagique.pilipos.ui.badge.badgeView
import dev.nuagemagique.pilipos.ui.badge.noBadges
import dev.nuagemagique.pilipos.ui.badge.outcome
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
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
import uniffi.pos_ffi.BadgeLogEntry
import uniffi.pos_ffi.BadgeOutcomeKind
import uniffi.pos_ffi.BadgeOverview
import uniffi.pos_ffi.BadgeSettingsInput
import uniffi.pos_ffi.BadgeSettingsView
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class SettingsBadgeModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: SettingsViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private val bobBadge = ActiveBadge(CASHIER_INSZ, "Bob", "2026-10-17 10:00:00", ADMIN_INSZ, "Alice")

    private suspend fun TestScope.start(
        who: User? = admin,
        reader: BadgeSettingsView = badgeView(reader = "usb"),
        overview: BadgeOverview = BadgeOverview(listOf(bobBadge), emptyList()),
        nfc: FakeNfcSupport = FakeNfcSupport(),
    ): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.getBadgeSettings() } returns reader
        coEvery { f.engine.listBadges() } returns overview
        val env = settingsEnv(f, context, nfc)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = SettingsViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun aManagerLoadsTheReaderSettingAndTheBadgesWithTheEmployees() =
        runTest {
            val r = start()

            assertEquals("usb", r.state.badge.settings?.reader)
            assertEquals(listOf(CASHIER_INSZ), r.state.badge.overview?.badges?.map { it.insz })
            assertTrue(r.state.showBadges)
            assertEquals(bobBadge, r.state.badgeOf(CASHIER_INSZ))
            assertNull(r.state.badgeOf(ADMIN_INSZ))
        }

    @Test
    fun withTheReaderOffTheRowsShowNoBadge() =
        runTest {
            val r = start(reader = badgeView(reader = "off"))

            assertFalse(r.state.showBadges)
            assertFalse(r.state.badge.readerOn)
        }

    @Test
    fun aNonManagerNeverAsksForTheBadgeList() =
        runTest {
            val r = start(who = cashier)

            coVerify(exactly = 0) { r.engine.listBadges() }
            assertFalse(r.state.showBadges)
        }

    @Test
    fun theReloadIsAllOrNothing() =
        runTest {
            val r = start()
            val before = r.state
            coEvery { r.engine.listBadges() } throws PosException.Coded("forbidden", emptyMap())
            coEvery { r.engine.listUsers() } returns listOf(admin)

            r.model.loadUsers()

            assertEquals(before.users, r.state.users)
            assertEquals(before.badge.overview, r.state.badge.overview)
        }

    @Test
    fun aDeviceWithoutNfcOffersNoNfcChoice() =
        runTest {
            val r = start(nfc = FakeNfcSupport(hasNfc = false, isEnabled = false))

            assertFalse(r.state.badge.hasNfc)
            assertFalse(r.state.badge.nfcEnabled)
        }

    @Test
    fun aDeviceWithNfcOffersIt() =
        runTest {
            val r = start(nfc = FakeNfcSupport(hasNfc = true, isEnabled = true))

            assertTrue(r.state.badge.hasNfc)
            assertTrue(r.state.badge.nfcEnabled)
        }

    @Test
    fun theRelinkBannerFollowsTheEngine() =
        runTest {
            val r = start(reader = badgeView(reader = "usb", relink = true))
            assertTrue(r.state.badge.settings?.relinkNeeded == true)
        }

    // ── The reader panel ──────────────────────────────────────────

    @Test
    fun thePanelIsFoldedByDefaultAndToggles() =
        runTest {
            val r = start()
            assertFalse(r.state.badge.open)

            r.model.badge.toggle()
            assertTrue(r.state.badge.open)
            r.model.badge.toggle()
            assertFalse(r.state.badge.open)
        }

    @Test
    fun choosingAReaderSendsTheWholeSetting() =
        runTest {
            val r = start(reader = badgeView(reader = "off", loginEnabled = true, gap = 70L, min = 5L, max = 20L))
            coEvery { r.engine.setBadgeSettings(any()) } returns
                badgeView(reader = "usb", loginEnabled = true, gap = 70L, min = 5L, max = 20L)

            r.model.badge.setReader("usb")
            runCurrent()

            coVerify { r.engine.setBadgeSettings(BadgeSettingsInput("usb", true, 70L, 5L, 20L)) }
            assertEquals("usb", r.state.badge.settings?.reader)
            assertNull(r.state.badge.error)
        }

    @Test
    fun theLoginSwitchFlipsLoginEnabled() =
        runTest {
            val r = start(reader = badgeView(reader = "usb", loginEnabled = false))
            coEvery { r.engine.setBadgeSettings(any()) } returns badgeView(reader = "usb", loginEnabled = true)

            r.model.badge.toggleLogin()
            runCurrent()

            coVerify { r.engine.setBadgeSettings(BadgeSettingsInput("usb", true, 50L, 6L, 32L)) }
            assertTrue(r.state.badge.settings?.loginEnabled == true)
        }

    @Test
    fun aRefusedSettingShowsItsTextAndKeepsTheStoredValue() =
        runTest {
            val r = start()
            coEvery { r.engine.setBadgeSettings(any()) } throws PosException.Coded("badge_settings_invalid", emptyMap())

            r.model.badge.setReader("off")
            runCurrent()

            assertEquals(r.env.text("err_badge_settings_invalid"), r.state.badge.error)
            assertEquals("usb", r.state.badge.settings?.reader)
        }

    @Test
    fun theTimingIsTypedOnThePadAndSaved() =
        runTest {
            val r = start(reader = badgeView(reader = "usb", gap = 50L, min = 6L, max = 32L))
            val badge = r.model.badge

            badge.editField(TimingField.Gap)
            badge.onKey(NumPadKey.Clear)
            badge.onKey(NumPadKey.Digit('1'))
            badge.onKey(NumPadKey.Digit('2'))
            badge.onKey(NumPadKey.Digit('0'))
            badge.onKey(NumPadKey.Digit('9'))
            assertEquals("120", r.state.badge.draft?.gap)
            assertTrue(r.state.badge.dirty)
            assertEquals(TimingField.Gap, r.state.badge.editing)

            badge.onKey(NumPadKey.Backspace)
            assertEquals("12", r.state.badge.draft?.gap)

            coEvery { r.engine.setBadgeSettings(any()) } returns badgeView(reader = "usb", gap = 12L)
            badge.saveTiming()
            runCurrent()

            coVerify { r.engine.setBadgeSettings(BadgeSettingsInput("usb", false, 12L, 6L, 32L)) }
            assertFalse(r.state.badge.dirty)
            assertNull(r.state.badge.editing)
            assertEquals("12", r.state.badge.draft?.gap)
        }

    @Test
    fun aRefusedTimingGoesBackToTheStoredNumbers() =
        runTest {
            val r = start(reader = badgeView(reader = "usb", gap = 50L))
            val badge = r.model.badge
            badge.editField(TimingField.MaxLen)
            badge.onKey(NumPadKey.Clear)
            badge.onKey(NumPadKey.Digit('2'))
            coEvery { r.engine.setBadgeSettings(any()) } throws PosException.Coded("badge_settings_invalid", emptyMap())

            badge.saveTiming()
            runCurrent()

            assertEquals("32", r.state.badge.draft?.maxLen)
            assertFalse(r.state.badge.dirty)
            assertNotNull(r.state.badge.error)
        }

    @Test
    fun theWholeTimingStartsFromTheStoredNumbers() =
        runTest {
            val r = start(reader = badgeView(reader = "usb", gap = 80L, min = 7L, max = 40L))
            assertEquals(BadgeTimingDraft("80", "7", "40"), r.state.badge.draft)
            assertFalse(r.state.badge.dirty)
        }

    // ── Enrolment ─────────────────────────────────────────────────

    @Test
    fun linkingAsksForTheBadgeThenForTheManagersPin() =
        runTest {
            val r = start()
            r.model.badge.startEnrol(waiter)

            assertEquals(BadgeEnrolFlow(WAITER_INSZ, "Chen", null), r.state.enrol)
            assertTrue(r.env.badges.store.capturing.value)

            // The badge is presented: the scan is captured, not sent to the engine.
            r.env.badges.usb.scan("CAFE01")
            runCurrent()

            assertEquals("CAFE01", r.state.enrol?.token)
            coVerify(exactly = 0) { r.engine.badgeTap(any(), any()) }

            coEvery { r.engine.linkBadge(WAITER_INSZ, "CAFE01", "1234") } returns BadgeLink(WAITER_INSZ, "Chen", "2026-10-17 10:00:00")
            r.model.badge.confirmEnrolPin("1234")
            r.model.badge.onEnrolDone()
            runCurrent()

            coVerify { r.engine.linkBadge(WAITER_INSZ, "CAFE01", "1234") }
            assertNull(r.state.enrol)
            val toast = r.env.lastToast()
            assertEquals(r.env.text("badge_linked_done", mapOf("name" to "Chen")), toast.message)
            assertEquals(ToastVariant.Success, toast.variant)
        }

    @Test
    fun leavingSettingsWhileTheDialogWaitsCancelsTheCapture() =
        runTest {
            val r = start()
            coEvery { r.engine.badgeTap(any(), any()) } returns outcome(BadgeOutcomeKind.IGNORED)
            r.model.badge.startEnrol(waiter)
            assertTrue(r.env.badges.store.capturing.value)

            // The view model goes away with the screen: onCleared clears the badge model.
            val owner = ViewModelStore()
            ViewModelProvider.create(
                owner,
                object : ViewModelProvider.Factory {
                    @Suppress("UNCHECKED_CAST")
                    override fun <T : ViewModel> create(modelClass: Class<T>): T = r.model as T
                },
            )[SettingsViewModel::class.java]
            owner.clear()

            assertFalse(r.env.badges.store.capturing.value)
            // The next scan is an ordinary scan: not swallowed by a dead capture.
            r.env.badges.usb.scan("0001")
            runCurrent()
            coVerify(exactly = 1) { r.engine.badgeTap("0001", false) }
        }

    @Test
    fun aWrongManagerPinKeepsTheDialogForAnotherTry() =
        runTest {
            val r = start()
            r.model.badge.startEnrol(waiter)
            r.env.badges.usb.scan("CAFE01")
            runCurrent()
            coEvery { r.engine.linkBadge(any(), any(), any()) } throws PosException.Coded("pin_wrong", emptyMap())

            val failure = runCatching { r.model.badge.confirmEnrolPin("0000") }.exceptionOrNull()

            assertEquals("pin_wrong", (failure as PosException.Coded).code)
            assertEquals("CAFE01", r.state.enrol?.token)
            assertTrue(r.env.toasts.toasts.value.isEmpty())
        }

    @Test
    fun aBadgeAlreadyLinkedEndsTheDialogWithItsText() =
        runTest {
            val r = start()
            r.model.badge.startEnrol(waiter)
            r.env.badges.usb.scan("CAFE01")
            runCurrent()
            coEvery { r.engine.linkBadge(any(), any(), any()) } throws PosException.Coded("badge_in_use", emptyMap())

            val failure = runCatching { r.model.badge.confirmEnrolPin("1234") }.exceptionOrNull()
            r.model.badge.onEnrolDone()

            assertTrue(failure is PosException.Coded)
            assertNull(r.state.enrol)
            val toast = r.env.lastToast()
            assertEquals(ToastVariant.Error, toast.variant)
            assertEquals(r.env.text("err_badge_in_use"), toast.message)
            // The success toast was not shown on top.
            assertEquals(1, r.env.toasts.toasts.value.size)
        }

    @Test
    fun noBadgeWithinThirtySecondsEndsTheDialog() =
        runTest {
            val r = start()
            r.model.badge.startEnrol(waiter)
            advanceTimeBy(BADGE_CAPTURE_MS - 1)
            runCurrent()
            assertNotNull(r.state.enrol)

            advanceTimeBy(2)
            runCurrent()

            assertNull(r.state.enrol)
            assertFalse(r.env.badges.store.capturing.value)
            val toast = r.env.lastToast()
            assertEquals(ToastVariant.Warning, toast.variant)
            assertEquals(r.env.text("badge_capture_timeout"), toast.message)
        }

    @Test
    fun cancellingLeavesNothingLinkedAndTheNextScanIsAnOrdinaryScan() =
        runTest {
            val r = start()
            coEvery { r.engine.badgeTap("0001", false) } returns outcome(BadgeOutcomeKind.IGNORED)
            r.model.badge.startEnrol(waiter)

            r.model.badge.cancelEnrol()
            r.env.badges.usb.scan("0001")
            runCurrent()

            assertNull(r.state.enrol)
            coVerify(exactly = 0) { r.engine.linkBadge(any(), any(), any()) }
            coVerify(exactly = 1) { r.engine.badgeTap("0001", false) }
        }

    @Test
    fun theTimeoutDoesNotFireAfterTheBadgeWasCaptured() =
        runTest {
            val r = start()
            r.model.badge.startEnrol(waiter)
            r.env.badges.usb.scan("CAFE01")
            runCurrent()

            advanceTimeBy(BADGE_CAPTURE_MS * 2)
            runCurrent()

            assertEquals("CAFE01", r.state.enrol?.token)
            assertTrue(r.env.toasts.toasts.value.isEmpty())
        }

    // ── Removal ───────────────────────────────────────────────────

    @Test
    fun removingABadgeAsksForAConfirmationFirst() =
        runTest {
            val r = start()
            r.model.badge.onEmployee(cashier)

            assertEquals(BadgeRevokeAsk(CASHIER_INSZ, "Bob"), r.state.revokeBadge)
            coVerify(exactly = 0) { r.engine.revokeBadge(any()) }

            coEvery { r.engine.revokeBadge(CASHIER_INSZ) } returns Unit
            coEvery { r.engine.listBadges() } returns noBadges()
            r.model.badge.confirmRevoke()
            runCurrent()

            coVerify { r.engine.revokeBadge(CASHIER_INSZ) }
            assertNull(r.state.revokeBadge)
            assertEquals(r.env.text("badge_revoked_done", mapOf("name" to "Bob")), r.env.lastToast().message)
            assertNull(r.state.badgeOf(CASHIER_INSZ))
        }

    @Test
    fun dismissingTheConfirmationRemovesNothing() =
        runTest {
            val r = start()
            r.model.badge.onEmployee(cashier)

            r.model.badge.cancelRevoke()

            assertNull(r.state.revokeBadge)
            coVerify(exactly = 0) { r.engine.revokeBadge(any()) }
        }

    @Test
    fun theButtonOfAnEmployeeWithoutABadgeStartsALink() =
        runTest {
            val r = start()

            r.model.badge.onEmployee(waiter)

            assertEquals(WAITER_INSZ, r.state.enrol?.insz)
            assertNull(r.state.revokeBadge)
        }

    @Test
    fun aRefusedRemovalShowsItsText() =
        runTest {
            val r = start()
            r.model.badge.onEmployee(cashier)
            coEvery { r.engine.revokeBadge(any()) } throws PosException.Coded("badge_none", emptyMap())

            r.model.badge.confirmRevoke()
            runCurrent()

            assertEquals(r.env.text("err_badge_none"), r.env.lastToast().message)
        }

    // ── Presence by badge refreshes who is on shift ───────────────

    @Test
    fun aRegisteredScanReloadsTheEmployees() =
        runTest {
            val r = start()
            coEvery { r.engine.badgeTap("0A0B0C", false) } returns
                outcome(BadgeOutcomeKind.CLOCK_IN, CASHIER_INSZ, "Bob", at = "2026-10-17T14:02:00Z")
            coEvery { r.engine.listOpenShifts() } returns listOf(uniffi.pos_ffi.OpenShift(CASHIER_INSZ, "2026-10-17 14:02:00"))

            r.env.badges.usb.scan("0A0B0C")
            runCurrent()

            assertEquals(mapOf(CASHIER_INSZ to "2026-10-17 14:02:00"), r.state.openShifts)
        }

    @Test
    fun theLogRowsComeWithTheOverview() =
        runTest {
            val log = listOf(BadgeLogEntry("link", CASHIER_INSZ, "Bob", ADMIN_INSZ, "Alice", "2026-10-17 10:00:00", null))
            val r = start(overview = BadgeOverview(listOf(bobBadge), log))

            assertEquals(log, r.state.badge.overview?.log)
        }
}
