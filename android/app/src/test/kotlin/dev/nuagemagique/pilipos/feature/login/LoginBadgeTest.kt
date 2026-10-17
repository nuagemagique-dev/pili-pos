package dev.nuagemagique.pilipos.feature.login

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import app.cash.turbine.test
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.ui.badge.badgeView
import dev.nuagemagique.pilipos.ui.badge.outcome
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
import uniffi.pos_ffi.BadgeOutcomeKind
import uniffi.pos_ffi.BadgeSettingsView
import uniffi.pos_ffi.UserRole

/** The badge reader on the login screen (lane 5): the hints, a manager's PIN request, a badge login, the end of shift. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class LoginBadgeTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private suspend fun TestScope.start(
        reader: BadgeSettingsView =
            badgeView(
                reader = "usb",
                loginEnabled = true,
            ),
    ): Pair<LoginRig, EngineFixture> {
        val f = loginFixture(tmp.root)
        coEvery { f.engine.getBadgeSettings() } returns reader
        return startLogin(f, context) to f
    }

    // ── Hints ─────────────────────────────────────────────────────

    @Test
    fun theHintsFollowTheReaderAndTheLoginSwitch() =
        runTest {
            val (on, _) = start(badgeView(reader = "usb", loginEnabled = true))
            assertTrue(on.state.badgeReaderOn)
            assertTrue(on.state.badgeLoginOn)
        }

    @Test
    fun aReaderWithoutBadgeLoginOnlyHintsTheEndOfShift() =
        runTest {
            val (r, _) = start(badgeView(reader = "usb", loginEnabled = false))
            assertTrue(r.state.badgeReaderOn)
            assertFalse(r.state.badgeLoginOn)
        }

    @Test
    fun withTheReaderOffThereIsNoHint() =
        runTest {
            val (r, _) = start(badgeView(reader = "off", loginEnabled = true))
            assertFalse(r.state.badgeReaderOn)
            assertFalse(r.state.badgeLoginOn)
        }

    // ── A badge login ─────────────────────────────────────────────

    @Test
    fun badgeLoginNavigatesLikeAPinLogin() =
        runTest {
            val (r, f) = start()
            coEvery { f.engine.badgeTap("AABBCC", false) } returns outcome(BadgeOutcomeKind.LOGGED_IN, CASHIER_INSZ, "Bob", user = cashier)

            r.model.effects.test {
                r.badges.usb.scan("AABBCC")
                runCurrent()

                assertEquals(LoginEffect.GoToTables, awaitItem())
                cancelAndIgnoreRemainingEvents()
            }
            assertEquals(cashier, r.badges.session.currentUser.value)
            assertNull(r.state.pin)
            assertEquals(listOf("ok"), r.badges.cue.played)
        }

    @Test
    fun aRefusedBadgeStaysOnTheLoginScreenWithAToast() =
        runTest {
            val (r, f) = start()
            coEvery { f.engine.badgeTap("ZZ", false) } throws uniffi.pos_ffi.PosException.Coded("badge_unknown", emptyMap())

            r.model.effects.test {
                r.badges.usb.scan("ZZ")
                runCurrent()
                expectNoEvents()
                cancelAndIgnoreRemainingEvents()
            }
            assertNull(r.badges.session.currentUser.value)
            assertEquals(r.text("err_badge_unknown"), r.lastToast().message)
        }

    // ── A manager's badge ─────────────────────────────────────────

    @Test
    fun pinRequiredOpensThePinPanel() =
        runTest {
            val (r, f) = start()
            coEvery { f.engine.badgeTap("ADMIN1", false) } returns outcome(BadgeOutcomeKind.PIN_REQUIRED, ADMIN_INSZ, "Alice")

            r.model.effects.test {
                r.badges.usb.scan("ADMIN1")
                runCurrent()
                expectNoEvents()
                cancelAndIgnoreRemainingEvents()
            }

            assertEquals(admin, r.state.pin?.user)
            assertFalse(r.state.pin!!.endShift)
            assertNull(r.badges.session.currentUser.value)
            coVerify(exactly = 0) { f.engine.login(any(), any()) }
        }

    @Test
    fun pinRequiredLeavesTheInszEntryForTheTiles() =
        runTest {
            val (r, f) = start()
            r.model.toggleInsz()
            assertTrue(r.state.numpadMode)
            coEvery { f.engine.badgeTap("ADMIN1", false) } returns outcome(BadgeOutcomeKind.PIN_REQUIRED, ADMIN_INSZ, "Alice")

            r.badges.usb.scan("ADMIN1")
            runCurrent()

            assertFalse(r.state.numpadMode)
            assertEquals(ADMIN_INSZ, r.state.pin?.user?.insz)
        }

    @Test
    fun pinRequiredForAManagerTheTilesDoNotKnowStillOpensThePad() =
        runTest {
            val (r, f) = start()
            coEvery { f.engine.badgeTap("NEW1", false) } returns outcome(BadgeOutcomeKind.PIN_REQUIRED, "70010100111", "Zed")

            r.badges.usb.scan("NEW1")
            runCurrent()

            val user = r.state.pin?.user
            assertNotNull(user)
            assertEquals("70010100111", user?.insz)
            assertEquals("Zed", user?.name)
            assertEquals(UserRole.ADMIN, user?.role)
            assertTrue(user?.hasPin == true)
        }

    @Test
    fun theManagerThenTypesThePinAndTheNormalLoginRuns() =
        runTest {
            val (r, f) = start()
            coEvery { f.engine.badgeTap("ADMIN1", false) } returns outcome(BadgeOutcomeKind.PIN_REQUIRED, ADMIN_INSZ, "Alice")
            coEvery { f.engine.login(ADMIN_INSZ, "1234") } returns admin
            r.badges.usb.scan("ADMIN1")
            runCurrent()

            r.model.effects.test {
                r.type("1234")
                runCurrent()
                assertEquals(LoginEffect.GoToTables, awaitItem())
                cancelAndIgnoreRemainingEvents()
            }
            coVerify(exactly = 1) { f.engine.login(ADMIN_INSZ, "1234") }
        }

    // ── The end of the shift ──────────────────────────────────────

    @Test
    fun theEndShiftModeIsToldToTheStoreAndSentWithTheScan() =
        runTest {
            val (r, f) = start()
            coEvery { f.engine.badgeTap("0A0B0C", true) } returns
                outcome(BadgeOutcomeKind.CLOCK_OUT, CASHIER_INSZ, "Bob", at = "2026-10-17T14:02:00Z")
            r.model.onTile(cashier)
            assertFalse(r.badges.store.endShiftMode.value)

            r.model.toggleEndShift()
            runCurrent()
            assertTrue(r.badges.store.endShiftMode.value)

            r.badges.usb.scan("0A0B0C")
            runCurrent()
            coVerify { f.engine.badgeTap("0A0B0C", true) }
        }

    @Test
    fun leavingTheEndShiftModeTellsTheStoreToo() =
        runTest {
            val (r, _) = start()
            r.model.onTile(cashier)
            r.model.toggleEndShift()
            runCurrent()
            assertTrue(r.badges.store.endShiftMode.value)

            r.model.toggleEndShift()
            runCurrent()
            assertFalse(r.badges.store.endShiftMode.value)

            r.model.toggleEndShift()
            runCurrent()
            r.model.cancelPin()
            runCurrent()
            assertFalse(r.badges.store.endShiftMode.value)
        }

    @Test
    fun aBadgeClockOutClosesTheEndShiftPadAndReloadsTheShifts() =
        runTest {
            val (r, f) = start()
            coEvery { f.engine.listOpenShifts() } returns listOf(shift(CASHIER_INSZ))
            coEvery { f.engine.badgeTap("0A0B0C", true) } returns
                outcome(BadgeOutcomeKind.CLOCK_OUT, CASHIER_INSZ, "Bob", at = "2026-10-17T14:02:00Z")
            r.model.onTile(cashier)
            r.model.toggleEndShift()
            runCurrent()
            coEvery { f.engine.listOpenShifts() } returns emptyList()

            r.badges.usb.scan("0A0B0C")
            runCurrent()

            assertNull(r.state.pin)
            assertTrue(r.state.shifts.isEmpty())
            assertEquals(r.text("badge_clock_out", mapOf("name" to "Bob", "time" to "16:02")), r.lastToast().message)
        }

    @Test
    fun anArrivalWithoutAnEndShiftPadKeepsThePinPanelButRefreshesTheTiles() =
        runTest {
            val (r, f) = start()
            r.model.onTile(cashier)
            coEvery { f.engine.listOpenShifts() } returns listOf(shift(WAITER_INSZ))
            coEvery { f.engine.badgeTap("0A0B0C", false) } returns
                outcome(BadgeOutcomeKind.CLOCK_IN, WAITER_INSZ, "Chen", at = "2026-10-17T14:02:00Z")

            r.badges.usb.scan("0A0B0C")
            runCurrent()

            assertEquals(cashier, r.state.pin?.user)
            assertEquals(listOf(WAITER_INSZ), r.state.shifts.map { it.userInsz })
        }
}
