package dev.nuagemagique.pilipos.feature.login

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import app.cash.turbine.test
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
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
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SocialResult
import uniffi.pos_ffi.UserRole

/** The actions of the login screen: tiles, PIN panel, end of shift, first PIN, INSZ entry and navigation. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class LoginActionsTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(): EngineFixture = loginFixture(tmp.root)

    private suspend fun TestScope.start(
        f: EngineFixture,
        open: Boolean = true,
    ): LoginRig = startLogin(f, context, open)

    // ── Tiles ─────────────────────────────────────────────────────

    @Test
    fun aTileWithAPinOpensThePinPanel() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.onTile(cashier)

            assertEquals(cashier, r.state.pin?.user)
            assertFalse(r.state.pin!!.endShift)
            assertNull(r.state.firstPin)
            coVerify(exactly = 0) { f.engine.login(any(), any()) }
        }

    @Test
    fun anAdminWithoutAPinChoosesOne() =
        runTest {
            val f = fixture()
            val noPinAdmin = user("11111111111", "Dora", UserRole.ADMIN, hasPin = false)
            stubReads(f.engine, users = listOf(noPinAdmin))
            val r = start(f)

            r.model.onTile(noPinAdmin)

            assertEquals(FirstPinTarget("11111111111", "Dora"), r.state.firstPin)
            assertNull(r.state.pin)
        }

    @Test
    fun anOperatorOnShiftWithoutAPinGetsThePanel() =
        runTest {
            val f = fixture()
            stubReads(f.engine, shifts = listOf(shift(WAITER_INSZ)))
            val r = start(f)

            r.model.onTile(waiterNoPin)

            assertEquals(waiterNoPin, r.state.pin?.user)
            coVerify(exactly = 0) { f.engine.login(any(), any()) }
        }

    @Test
    fun anOperatorOffShiftWithoutAPinLogsInAtOnceWithTheWarning() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login(WAITER_INSZ, null) } returns waiterNoPin
            val r = start(f)

            r.model.effects.test {
                r.model.onTile(waiterNoPin)
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }

            coVerify(exactly = 1) { f.engine.login(WAITER_INSZ, null) }
            val toast = r.lastToast()
            assertEquals(ToastVariant.Warning, toast.variant)
            assertEquals(r.text("pin_not_set_warning"), toast.message)
        }

    @Test
    fun aPinSetupRefusalOnATileAsksForAFirstPin() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login(WAITER_INSZ, null) } throws PosException.Coded("pin_setup_required", emptyMap())
            val r = start(f)

            r.model.onTile(waiterNoPin)

            assertEquals(FirstPinTarget(WAITER_INSZ, "Chen"), r.state.firstPin)
            assertNull(r.state.error)
        }

    @Test
    fun anyOtherRefusalOnATileShowsTheErrorLine() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login(WAITER_INSZ, null) } throws PosException.Coded("pin_locked", mapOf("seconds" to "30"))
            val r = start(f)

            r.model.onTile(waiterNoPin)

            assertEquals(r.text("err_pin_locked", mapOf("seconds" to "30")), r.state.error)
            // A new tap clears the line.
            r.model.onTile(cashier)
            assertNull(r.state.error)
        }

    // ── PIN panel ─────────────────────────────────────────────────

    @Test
    fun theFourthDigitLogsInAndLeavesForTables() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login(CASHIER_INSZ, "1234") } returns cashier
            val r = start(f)
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("123")
                expectNoEvents()
                r.type("4")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }

            coVerify(exactly = 1) { f.engine.login(CASHIER_INSZ, "1234") }
            assertNull(r.state.pin)
        }

    @Test
    fun aPinRefusalShowsTheDescribedErrorAndClearsThePad() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login(CASHIER_INSZ, "0000") } throws PosException.Coded("pin_wrong", emptyMap())
            val r = start(f)
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("0000")
                expectNoEvents()
            }

            val pad = r.state.pin!!.pad
            assertEquals(r.text("err_pin_wrong"), pad.error)
            assertEquals("", pad.digits)
            assertFalse(pad.busy)
            // The panel stays open for another try.
            assertEquals(cashier, r.state.pin?.user)
        }

    @Test
    fun theKeysEditThePad() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.onTile(cashier)

            r.type("12")
            r.model.onPinKey(NumPadKey.Backspace)
            assertEquals("1", r.state.pin!!.pad.digits)
            r.model.onPinKey(NumPadKey.Clear)
            assertEquals("", r.state.pin!!.pad.digits)
            // A key with no panel open is ignored.
            r.model.cancelPin()
            assertNull(r.state.pin)
            r.model.onPinKey(NumPadKey.Digit('1'))
            assertNull(r.state.pin)
        }

    @Test
    fun backClosesThePanel() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.onTile(cashier)
            assertNotNull(r.state.pin)

            r.model.cancelPin()

            assertNull(r.state.pin)
        }

    // ── End of shift ──────────────────────────────────────────────

    @Test
    fun theEndShiftModeTakesThePinToEndTheShiftAndReloadsTheShifts() =
        runTest {
            val f = fixture()
            stubReads(f.engine, shifts = listOf(shift(CASHIER_INSZ)))
            coEvery { f.engine.endShift(CASHIER_INSZ, "1234") } returns SocialResult(true, null)
            val r = start(f)
            assertNotNull(r.state.shiftStart(CASHIER_INSZ))
            r.model.onTile(cashier)
            r.model.toggleEndShift()
            assertTrue(r.state.pin!!.endShift)
            // The shifts are read again after the clock-out.
            coEvery { f.engine.listOpenShifts() } returns emptyList()

            r.model.effects.test {
                r.type("1234")
                expectNoEvents()
            }

            coVerify(exactly = 1) { f.engine.endShift(CASHIER_INSZ, "1234") }
            coVerify(exactly = 0) { f.engine.login(any(), any()) }
            assertNull(r.state.pin)
            assertNull(r.state.shiftStart(CASHIER_INSZ))
            val toast = r.lastToast()
            assertEquals(ToastVariant.Success, toast.variant)
            assertEquals(r.text("shift_ended", mapOf("name" to "Bob")), toast.message)
        }

    @Test
    fun theEndShiftToggleSwitchesBackAndClearsThePad() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.onTile(cashier)
            r.type("12")

            r.model.toggleEndShift()
            assertTrue(r.state.pin!!.endShift)
            assertEquals("", r.state.pin!!.pad.digits)
            r.model.toggleEndShift()
            assertFalse(r.state.pin!!.endShift)
        }

    @Test
    fun aFailedClockOutShowsTheErrorToastWithThePresenceText() =
        runTest {
            val f = fixture()
            coEvery { f.engine.endShift(CASHIER_INSZ, "1234") } returns SocialResult(false, "FDM_ANSWER_PENDING")
            val r = start(f)
            r.model.onTile(cashier)
            r.model.toggleEndShift()

            r.type("1234")

            val toast = r.lastToast()
            assertEquals(ToastVariant.Error, toast.variant)
            assertTrue(toast.message.startsWith(r.text("presence_failed") + " : "))
            assertNull(r.state.pin)
        }

    @Test
    fun endShiftWithoutAPinNeedsNoPin() =
        runTest {
            val f = fixture()
            stubReads(f.engine, shifts = listOf(shift(WAITER_INSZ)))
            coEvery { f.engine.endShift(WAITER_INSZ, null) } returns SocialResult(true, null)
            val r = start(f)
            r.model.onTile(waiterNoPin)

            r.model.endShiftFromPanel()

            coVerify(exactly = 1) { f.engine.endShift(WAITER_INSZ, null) }
            assertNull(r.state.pin)
            assertEquals(ToastVariant.Success, r.lastToast().variant)
        }

    @Test
    fun aRefusedEndShiftWithoutAPinShowsTheErrorInThePanel() =
        runTest {
            val f = fixture()
            stubReads(f.engine, shifts = listOf(shift(WAITER_INSZ)))
            coEvery { f.engine.endShift(WAITER_INSZ, null) } throws PosException.Coded("pin_wrong", emptyMap())
            val r = start(f)
            r.model.onTile(waiterNoPin)

            r.model.endShiftFromPanel()

            assertEquals(r.text("err_pin_wrong"), r.state.pin?.error)
            assertEquals(waiterNoPin, r.state.pin?.user)
        }

    @Test
    fun aRefusedEndShiftWithAPinClearsThePad() =
        runTest {
            val f = fixture()
            coEvery { f.engine.endShift(CASHIER_INSZ, "9999") } throws PosException.Coded("pin_wrong", emptyMap())
            val r = start(f)
            r.model.onTile(cashier)
            r.model.toggleEndShift()

            r.type("9999")

            assertEquals(r.text("err_pin_wrong"), r.state.pin!!.pad.error)
            assertEquals("", r.state.pin!!.pad.digits)
            assertTrue(r.state.pin!!.endShift)
        }

    @Test
    fun theLoginButtonOfThePanelLogsInWithoutAPin() =
        runTest {
            val f = fixture()
            stubReads(f.engine, shifts = listOf(shift(WAITER_INSZ)))
            coEvery { f.engine.login(WAITER_INSZ, null) } returns waiterNoPin
            val r = start(f)
            r.model.onTile(waiterNoPin)

            r.model.effects.test {
                r.model.loginFromPanel()
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }

            coVerify(exactly = 1) { f.engine.login(WAITER_INSZ, null) }
        }

    @Test
    fun thePanelButtonsDoNothingWithoutAPanel() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.loginFromPanel()
            r.model.endShiftFromPanel()
            r.model.toggleEndShift()

            coVerify(exactly = 0) { f.engine.login(any(), any()) }
            coVerify(exactly = 0) { f.engine.endShift(any(), any()) }
            assertNull(r.state.pin)
        }

    // ── First PIN ─────────────────────────────────────────────────

    @Test
    fun theChosenPinOpensTheSessionAndLeavesForTables() =
        runTest {
            val f = fixture()
            val noPinAdmin = user("11111111111", "Dora", UserRole.ADMIN, hasPin = false)
            stubReads(f.engine, users = listOf(noPinAdmin))
            coEvery { f.engine.loginWithNewPin("11111111111", "4321") } returns noPinAdmin
            val r = start(f)
            r.model.onTile(noPinAdmin)

            r.model.effects.test {
                r.model.chooseFirstPin("4321")
                expectNoEvents()
                r.model.onFirstPinDone()
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }

            coVerify(exactly = 1) { f.engine.loginWithNewPin("11111111111", "4321") }
            assertNull(r.state.firstPin)
            assertEquals(ToastVariant.Success, r.lastToast().variant)
            assertEquals(r.text("pin_chosen"), r.lastToast().message)
        }

    @Test
    fun aRefusedFirstPinPropagatesToTheDialogAndKeepsIt() =
        runTest {
            val f = fixture()
            val noPinAdmin = user("11111111111", "Dora", UserRole.ADMIN, hasPin = false)
            stubReads(f.engine, users = listOf(noPinAdmin))
            coEvery { f.engine.loginWithNewPin(any(), any()) } throws PosException.Coded("pin_format", emptyMap())
            val r = start(f)
            r.model.onTile(noPinAdmin)

            val failure = runCatching { r.model.chooseFirstPin("12") }.exceptionOrNull()

            assertNotNull(failure)
            assertEquals(r.text("err_pin_format"), r.model.describeError(failure!!))
            assertNotNull(r.state.firstPin)
            assertTrue(r.toasts.toasts.value.isEmpty())
        }

    @Test
    fun cancellingTheFirstPinClosesTheDialog() =
        runTest {
            val f = fixture()
            val noPinAdmin = user("11111111111", "Dora", UserRole.ADMIN, hasPin = false)
            stubReads(f.engine, users = listOf(noPinAdmin))
            val r = start(f)
            r.model.onTile(noPinAdmin)

            r.model.cancelFirstPin()

            assertNull(r.state.firstPin)
        }

    @Test
    fun aFirstPinWithoutATargetDoesNothing() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.chooseFirstPin("1234")

            coVerify(exactly = 0) { f.engine.loginWithNewPin(any(), any()) }
        }

    // ── INSZ entry ────────────────────────────────────────────────

    @Test
    fun theInszKeysTypeEraseAndStopAtEleven() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.typeInsz("123456789012")
            assertEquals("12345678901", r.state.insz)
            r.model.onInszKey(NumPadKey.Backspace)
            assertEquals("1234567890", r.state.insz)
            r.model.onInszKey(NumPadKey.Clear)
            assertEquals("", r.state.insz)
        }

    @Test
    fun theOtherPersonButtonSwapsTilesAndInszAndResets() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.model.onTile(cashier)
            r.typeInsz("12")
            assertFalse(r.state.numpadMode)

            r.model.toggleInsz()
            assertTrue(r.state.numpadMode)
            assertEquals("", r.state.insz)
            assertNull(r.state.pin)

            r.typeInsz("1")
            r.model.toggleInsz()
            assertFalse(r.state.numpadMode)
            assertEquals("", r.state.insz)
        }

    @Test
    fun aShortInszIsRefusedWithoutACall() =
        runTest {
            val f = fixture()
            val r = start(f)
            r.typeInsz("1234")

            r.model.submitInsz()

            assertEquals(r.text("settings_err_11_digits_required"), r.state.error)
            coVerify(exactly = 0) { f.engine.login(any(), any()) }
        }

    @Test
    fun aFullInszLogsInWithoutAPin() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login("12345678901", null) } returns waiterNoPin
            val r = start(f)
            r.typeInsz("12345678901")

            r.model.effects.test {
                r.model.submitInsz()
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }

            coVerify(exactly = 1) { f.engine.login("12345678901", null) }
        }

    @Test
    fun aPinRequiredRefusalOpensThePanelOfTheKnownUser() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login(CASHIER_INSZ, null) } throws PosException.Coded("pin_required", emptyMap())
            val r = start(f)
            r.model.toggleInsz()
            r.typeInsz(CASHIER_INSZ)

            r.model.submitInsz()

            assertEquals(cashier, r.state.pin?.user)
            assertNull(r.state.error)
        }

    @Test
    fun aPinRequiredRefusalForAnUnknownInszOpensAPanelNamedByTheInsz() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login("12345678901", null) } throws PosException.Coded("pin_required", emptyMap())
            val r = start(f)
            r.typeInsz("12345678901")

            r.model.submitInsz()

            val panel = r.state.pin!!
            assertEquals("12345678901", panel.user.insz)
            assertEquals("12345678901", panel.user.name)
            assertEquals(UserRole.WAITER, panel.user.role)
            assertTrue(panel.user.hasPin)
        }

    @Test
    fun aPinSetupRefusalForAnInszAsksForAFirstPinNamedAfterTheKnownUser() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login(ADMIN_INSZ, null) } throws PosException.Coded("pin_setup_required", emptyMap())
            val r = start(f)
            r.typeInsz(ADMIN_INSZ)

            r.model.submitInsz()

            assertEquals(FirstPinTarget(ADMIN_INSZ, "Alice"), r.state.firstPin)
        }

    @Test
    fun anyOtherInszRefusalShowsTheErrorLine() =
        runTest {
            val f = fixture()
            coEvery { f.engine.login("12345678901", null) } throws PosException.Coded("pin_locked", mapOf("seconds" to "5"))
            val r = start(f)
            r.typeInsz("12345678901")

            r.model.submitInsz()

            assertEquals(r.text("err_pin_locked", mapOf("seconds" to "5")), r.state.error)
        }

    // ── Navigation ────────────────────────────────────────────────

    @Test
    fun nothingNavigatesWhenTheScreenStartsOrWhileTheOperatorBrowses() =
        runTest {
            val f = fixture()
            val r = start(f)

            r.model.effects.test {
                expectNoEvents()
                r.model.onTile(cashier)
                r.model.toggleEndShift()
                r.model.cancelPin()
                r.model.toggleInsz()
                r.typeInsz("12")
                r.model.setLanguage(AppLanguage.EN)
                expectNoEvents()
            }
        }

    @Test
    fun theTablesEffectReplacesTheWholeStackAndNothingElse() {
        val navigator = RecordingNavigator()

        LoginEffect.GoToTables.perform(navigator)

        assertEquals(listOf("replaceAll:${TablesRoutes.TABLES}"), navigator.calls)
    }
}
