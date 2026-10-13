package dev.nuagemagique.pilipos.feature.login

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.AppLanguage
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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.UserRole

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class LoginViewModelTest {
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

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun loadsTheActiveUsersAndTheOpenShifts() =
        runTest {
            val f = fixture()
            val inactive = user("00000000000", "Gone", UserRole.WAITER, active = false)
            stubReads(f.engine, users = listOf(admin, inactive, cashier), shifts = listOf(shift(CASHIER_INSZ)))

            val r = start(f)

            assertEquals(listOf(admin, cashier), r.state.users)
            assertEquals("2026-10-09 09:05:00", r.state.shiftStart(CASHIER_INSZ))
            assertNull(r.state.shiftStart(ADMIN_INSZ))
            assertFalse(r.state.numpadMode)
        }

    @Test
    fun anEmptyUserListGivesTheNumpadMode() =
        runTest {
            val f = fixture()
            stubReads(f.engine, users = emptyList())

            val r = start(f)

            assertTrue(r.state.users.isEmpty())
            assertTrue(r.state.numpadMode)
        }

    @Test
    fun aFailingUserListGivesTheNumpadMode() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listUsers() } throws PosException.Coded("boom", emptyMap())

            val r = start(f)

            assertTrue(r.state.numpadMode)
        }

    @Test
    fun aFailingShiftListGivesNoShift() =
        runTest {
            val f = fixture()
            coEvery { f.engine.listOpenShifts() } throws PosException.Coded("boom", emptyMap())

            val r = start(f)

            assertTrue(r.state.shifts.isEmpty())
            assertEquals(3, r.state.users.size)
        }

    @Test
    fun theFooterTheVersionAndTheLanguageComeFromTheSettings() =
        runTest {
            val f = fixture()
            coEvery { f.engine.getSettings() } returns settings(training = true)

            val r = start(f)

            assertEquals("Chez Pili · T1", r.state.footer)
            assertEquals("pili-pos-android/1.2.3", r.state.version)
            assertTrue(r.state.training)
            assertEquals(AppLanguage.FR, r.state.language)

            r.model.setLanguage(AppLanguage.ZH)
            assertEquals(AppLanguage.ZH, r.state.language)
            assertEquals(AppLanguage.ZH, r.settings.language.value)
        }

    @Test
    fun theEngineNotReadyGivesTheNumpadModeAndNoChecks() =
        runTest {
            val f = fixture()

            val r = start(f, open = false)

            assertTrue(r.state.numpadMode)
            assertTrue(r.state.users.isEmpty())
            assertEquals("", r.state.version)
            assertEquals("", r.state.footer)
            assertEquals(
                listOf(CheckId.Fdm, CheckId.Ticket, CheckId.Kitchen, CheckId.Mail),
                r.state.checks.map { it.id },
            )
            assertTrue(r.state.checks.all { it.state == CheckState.None })
            assertTrue(r.state.checks.all { it.textKey == "check_not_configured" })
            // The numpad still works: nothing throws.
            r.typeInsz("1234")
            assertEquals("1234", r.state.insz)
        }

    // ── Start-up checks ───────────────────────────────────────────

    @Test
    fun theChecksReportEachDevice() =
        runTest {
            val f = fixture()
            coEvery { f.engine.checkKitchenPrinter() } returns false
            coEvery { f.engine.checkMailStatus() } returns mailStatus(configured = true, reachable = true)

            val r = start(f)

            val byId = r.state.checks.associateBy { it.id }
            assertEquals(StartupCheck(CheckId.Fdm, CheckState.Ok, "check_connected"), byId[CheckId.Fdm])
            assertEquals(StartupCheck(CheckId.Ticket, CheckState.Ok, "check_ready"), byId[CheckId.Ticket])
            assertEquals(StartupCheck(CheckId.Kitchen, CheckState.Err, "check_unreachable"), byId[CheckId.Kitchen])
            assertEquals(StartupCheck(CheckId.Mail, CheckState.Ok, "check_reachable"), byId[CheckId.Mail])
            assertNull(byId[CheckId.Hubrise])
        }

    @Test
    fun theFdmSeverityGivesTheState() =
        runTest {
            listOf(
                FdmSeverity.BLOCKING to (CheckState.Err to "check_offline"),
                FdmSeverity.WARNING to (CheckState.Warn to "check_warning"),
                FdmSeverity.INFO to (CheckState.Ok to "check_connected"),
            ).forEach { (severity, expected) ->
                val f = fixture()
                coEvery { f.engine.fdmCheckNow() } returns fdmHealth(severity)
                val r = start(f)
                assertEquals(StartupCheck(CheckId.Fdm, expected.first, expected.second), r.state.checks.first())
            }
        }

    @Test
    fun anUnansweredFdmCheckIsNotConfigured() =
        runTest {
            val f = fixture()
            coEvery { f.engine.fdmCheckNow() } throws PosException.Coded("boom", emptyMap())

            val r = start(f)

            assertEquals(StartupCheck(CheckId.Fdm, CheckState.None, "check_not_configured"), r.state.checks.first())
        }

    @Test
    fun aPrinterThatIsNotConfiguredOrUnreachable() =
        runTest {
            val f = fixture()
            coEvery { f.engine.checkPrinter() } returns false
            coEvery { f.engine.checkKitchenPrinter() } returns null

            val r = start(f)

            val byId = r.state.checks.associateBy { it.id }
            assertEquals(StartupCheck(CheckId.Ticket, CheckState.Err, "check_unreachable"), byId[CheckId.Ticket])
            assertEquals(StartupCheck(CheckId.Kitchen, CheckState.None, "check_not_configured"), byId[CheckId.Kitchen])
        }

    @Test
    fun theTerminalHasALineOnlyWhenOneIsConfigured() =
        runTest {
            val f = fixture()
            val none = start(f)
            assertNull(none.state.checks.firstOrNull { it.id == CheckId.Terminal })

            val g = fixture()
            coEvery { g.engine.checkPaymentTerminal() } returns terminalStatus()
            val ok = start(g)
            val line = ok.state.checks.single { it.id == CheckId.Terminal }
            assertEquals(StartupCheck(CheckId.Terminal, CheckState.Ok, "check_connected"), line)
            // After the printers, before the mail line.
            assertEquals(
                listOf(CheckId.Fdm, CheckId.Ticket, CheckId.Kitchen, CheckId.Terminal, CheckId.Mail),
                ok.state.checks.map { it.id },
            )
        }

    @Test
    fun theTerminalStatesAndItsReason() =
        runTest {
            val busy = fixture()
            coEvery { busy.engine.checkPaymentTerminal() } returns terminalStatus(busy = true)
            assertEquals(
                StartupCheck(CheckId.Terminal, CheckState.Warn, "check_payment_busy"),
                start(busy).state.checks.single { it.id == CheckId.Terminal },
            )

            val down = fixture()
            coEvery { down.engine.checkPaymentTerminal() } returns terminalStatus(connected = false, detail = "connection refused")
            val r = start(down)
            assertEquals(
                StartupCheck(CheckId.Terminal, CheckState.Err, "check_unreachable", "connection refused"),
                r.state.checks.single { it.id == CheckId.Terminal },
            )
            // Informative: the users are still there to log in.
            assertTrue(r.state.users.isNotEmpty() || r.state.numpadMode)

            val failed = fixture()
            coEvery { failed.engine.checkPaymentTerminal() } throws PosException.Coded("boom", emptyMap())
            assertNull(start(failed).state.checks.firstOrNull { it.id == CheckId.Terminal })
        }

    @Test
    fun theMailStates() =
        runTest {
            val f = fixture()
            coEvery { f.engine.checkMailStatus() } returns mailStatus(configured = true, reachable = false)
            val unreachable = start(f).state.checks.single { it.id == CheckId.Mail }
            assertEquals(StartupCheck(CheckId.Mail, CheckState.Warn, "check_unreachable"), unreachable)

            val g = fixture()
            coEvery { g.engine.checkMailStatus() } throws PosException.Coded("boom", emptyMap())
            val failed = start(g).state.checks.single { it.id == CheckId.Mail }
            assertEquals(StartupCheck(CheckId.Mail, CheckState.None, "check_not_configured"), failed)
        }

    @Test
    fun hubriseIsCheckedOnlyWhenTheIntegrationIsOn() =
        runTest {
            val off = fixture()
            val offRig = start(off)
            assertNull(offRig.state.checks.firstOrNull { it.id == CheckId.Hubrise })
            coVerify(exactly = 0) { off.engine.hubriseGetConnectionStatus() }

            val on = fixture()
            stubReads(on.engine, hubrise = true)
            val onRig = start(on)
            assertEquals(
                StartupCheck(CheckId.Hubrise, CheckState.Ok, "check_connected"),
                onRig.state.checks.single { it.id == CheckId.Hubrise },
            )
            assertEquals(
                listOf(CheckId.Fdm, CheckId.Ticket, CheckId.Kitchen, CheckId.Hubrise, CheckId.Mail),
                onRig.state.checks.map { it.id },
            )
        }

    @Test
    fun hubriseNotConnectedIsAWarningAndNoAnswerIsNone() =
        runTest {
            val f = fixture()
            stubReads(f.engine, hubrise = true)
            coEvery { f.engine.hubriseGetConnectionStatus() } returns hubriseInfo(false)
            val warn = start(f).state.checks.single { it.id == CheckId.Hubrise }
            assertEquals(StartupCheck(CheckId.Hubrise, CheckState.Warn, "check_not_connected"), warn)

            val g = fixture()
            stubReads(g.engine, hubrise = true)
            coEvery { g.engine.hubriseGetConnectionStatus() } throws PosException.Coded("boom", emptyMap())
            val none = start(g).state.checks.single { it.id == CheckId.Hubrise }
            assertEquals(StartupCheck(CheckId.Hubrise, CheckState.None, "check_not_connected"), none)
        }
}
