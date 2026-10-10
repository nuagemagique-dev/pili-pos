package dev.nuagemagique.pilipos.feature.settings

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
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
import uniffi.pos_ffi.CreateUserInput
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.TablePlan
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class SettingsViewModelTest {
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

    /** The view model over the stubbed engine; [who] is logged in first unless null. */
    private suspend fun TestScope.start(
        who: User? = admin,
        open: Boolean = true,
        hubrise: Boolean = false,
        arrange: (SettingsEnv) -> Unit = {},
    ): Rig {
        val f = settingsFixture(tmp.root, hubrise)
        val env = settingsEnv(f, context)
        arrange(env)
        if (open) openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = SettingsViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    // ── Rows (UI-PD308) ───────────────────────────────────────────

    @Test
    fun aManagerOnlyRowIsHiddenFromANonManagerAndShownToAManager() =
        runTest {
            val r = start(who = cashier)

            val forWorker = r.model.rows(isManager = false)
            val forManager = r.model.rows(isManager = true)

            assertEquals(listOf(rowOpen), forWorker)
            assertFalse(forWorker.any { it.managerOnly })
            assertEquals(listOf(rowOpen, rowAbout, rowExport), forManager)
            assertTrue(forManager.any { it.id == "platform.about" })
        }

    @Test
    fun theStateRowsFollowTheRoleOfTheOperator() =
        runTest {
            val asCashier = start(who = cashier)
            assertEquals(listOf("a.open"), asCashier.state.rows.map { it.id })
            assertFalse(asCashier.state.isManager)
        }

    @Test
    fun theStateRowsOfAManagerKeepTheRegistryOrder() =
        runTest {
            val r = start(who = admin)

            assertEquals(listOf("a.open", "platform.about", "platform.export"), r.state.rows.map { it.id })
            assertTrue(r.state.isManager)
            assertEquals(ADMIN_INSZ, r.state.currentInsz)
        }

    @Test
    fun withNoOperatorTheManagerRowsStayHidden() =
        runTest {
            val r = start(who = null)

            assertEquals(listOf("a.open"), r.state.rows.map { it.id })
            assertNull(r.state.currentInsz)
        }

    @Test
    fun visibleRowsKeepsTheOrderItIsGiven() {
        val rows = listOf(rowExport, rowOpen, rowAbout)

        assertEquals(listOf(rowExport, rowOpen, rowAbout), visibleRows(rows, isManager = true))
        assertEquals(listOf(rowOpen), visibleRows(rows, isManager = false))
    }

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun loadsTheEmployeesTheMailStatusAndTheDeliveries() =
        runTest {
            val r = start()

            assertEquals(listOf(admin, cashier, waiter), r.state.users)
            assertEquals(mail(), r.state.mailStatus)
            assertEquals(listOf(1L, 2L), r.state.deliveries.map { it.id })
            assertFalse(r.state.deliveriesLoading)
            coVerify(exactly = 1) { r.engine.listTicketDeliveries(50L) }
        }

    @Test
    fun theInfoComesFromTheEngineSettings() =
        runTest {
            val r = start()

            assertEquals("POS1", r.state.info?.posId)
            assertEquals("Chez Pili", r.state.info?.restaurantName)
        }

    @Test
    fun failingReadsGiveEmptyListsAndNoMailStatus() =
        runTest {
            val r =
                start(who = null, open = false) { env ->
                    coEvery { env.f.engine.listUsers() } throws PosException.Coded("boom", emptyMap())
                    coEvery { env.f.engine.checkMailStatus() } throws PosException.Coded("boom", emptyMap())
                    coEvery { env.f.engine.listTicketDeliveries(50L) } throws PosException.Coded("boom", emptyMap())
                }
            openEngine(r.env)

            assertTrue(r.state.users.isEmpty())
            assertNull(r.state.mailStatus)
            assertTrue(r.state.deliveries.isEmpty())
            assertFalse(r.state.deliveriesLoading)
        }

    @Test
    fun aFailedUserReloadKeepsThePreviousList() =
        runTest {
            val r = start()
            coEvery { r.engine.listUsers() } throws PosException.Coded("boom", emptyMap())

            r.model.loadUsers()

            assertEquals(3, r.state.users.size)
        }

    @Test
    fun nothingLoadsBeforeTheEngineIsReadyThenItDoes() =
        runTest {
            val r = start(who = null, open = false)
            assertTrue(r.state.users.isEmpty())
            coVerify(exactly = 0) { r.engine.listUsers() }

            openEngine(r.env)

            assertEquals(3, r.state.users.size)
        }

    @Test
    fun reloadDeliveriesAsksTheEngineAgain() =
        runTest {
            val r = start()
            coEvery { r.engine.listTicketDeliveries(50L) } returns listOf(delivery(9L))

            r.model.reloadDeliveries()
            runCurrent()

            assertEquals(listOf(9L), r.state.deliveries.map { it.id })
        }

    // ── Training, language, product label ────────────────────────

    @Test
    fun theTrainingSwitchGoesThroughTheEngine() =
        runTest {
            val r = start()
            coEvery { r.engine.setTrainingMode(true) } returns true

            r.model.toggleTraining()
            runCurrent()

            coVerify(exactly = 1) { r.engine.setTrainingMode(true) }
            assertTrue(r.state.training)

            coEvery { r.engine.setTrainingMode(false) } returns false
            r.model.toggleTraining()
            runCurrent()

            assertFalse(r.state.training)
        }

    @Test
    fun theTablePlanIsLoadedThenSavedWithASuccessToast() =
        runTest {
            val r = start()
            assertEquals(TablePlanState(plan = "1-20", count = 20L, input = "1-20"), r.state.tablePlan)
            coEvery { r.engine.setTablePlan("20-25,1-12") } returns TablePlan("1-12, 20-25", 18L, emptyList())

            r.model.setTablePlanInput("20-25,1-12")
            r.model.saveTablePlan()
            runCurrent()

            assertEquals(TablePlanState(plan = "1-12, 20-25", count = 18L, input = "1-12, 20-25"), r.state.tablePlan)
            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
            assertEquals(r.env.text("settings_tables_saved"), r.env.lastToast().message)
        }

    @Test
    fun aRefusedTablePlanKeepsTheFieldAndShowsWhy() =
        runTest {
            val r = start()
            val refusal = PosException.Coded("table_plan_invalid", emptyMap())
            coEvery { r.engine.setTablePlan("A1") } throws refusal

            r.model.setTablePlanInput("A1")
            r.model.saveTablePlan()
            runCurrent()

            assertEquals("A1", r.state.tablePlan.input)
            assertEquals("1-20", r.state.tablePlan.plan)
            assertEquals(r.env.describe(refusal), r.state.tablePlan.error)
        }

    @Test
    fun aRefusedTrainingSwitchIsShownAsAnErrorToast() =
        runTest {
            val r = start()
            val refusal = PosException.Coded("order_open", emptyMap())
            coEvery { r.engine.setTrainingMode(true) } throws refusal

            r.model.toggleTraining()
            runCurrent()

            assertFalse(r.state.training)
            assertEquals(ToastVariant.Error, r.env.lastToast().variant)
            assertEquals(r.env.describe(refusal), r.env.lastToast().message)
        }

    @Test
    fun theLanguageAndTheProductLabelAreKeptInTheStore() =
        runTest {
            val r = start()

            r.model.setLanguage(AppLanguage.ZH)
            r.model.setProductLabel(ProductLabel.CODE)
            runCurrent()

            assertEquals(AppLanguage.ZH, r.env.settings.language.value)
            assertEquals(AppLanguage.ZH, r.state.language)
            assertEquals(ProductLabel.CODE, r.env.settings.productLabel.value)
            assertEquals(ProductLabel.CODE, r.state.productLabel)
        }

    // ── New employee ──────────────────────────────────────────────

    @Test
    fun theInszFieldKeepsElevenDigitsOnly() =
        runTest {
            val r = start()

            r.model.setNewInsz("99.04-04 0045 6789012")

            assertEquals("99040400456", r.state.form.insz)
            assertEquals(INSZ_LENGTH, r.state.form.insz.length)
        }

    @Test
    fun aMissingNameOrInszIsRefused() =
        runTest {
            val r = start()

            r.model.addEmployee()
            assertEquals(r.env.text("settings_err_name_insz_required"), r.state.form.error)

            r.model.setNewName("   ")
            r.model.setNewInsz(NEW_INSZ)
            r.model.addEmployee()
            assertEquals(r.env.text("settings_err_name_insz_required"), r.state.form.error)

            r.model.setNewName("Dan")
            r.model.setNewInsz("")
            r.model.addEmployee()
            assertEquals(r.env.text("settings_err_name_insz_required"), r.state.form.error)
            coVerify(exactly = 0) { r.engine.createUser(any()) }
        }

    @Test
    fun anInszOfTheWrongLengthIsRefused() =
        runTest {
            val r = start()
            r.model.setNewName("Dan")
            r.model.setNewInsz("1234")

            r.model.addEmployee()

            assertEquals(r.env.text("settings_err_11_digits_required"), r.state.form.error)
            coVerify(exactly = 0) { r.engine.createUser(any()) }
        }

    @Test
    fun aWaiterIsCreatedAtOnceAndTheListReloads() =
        runTest {
            val r = start()
            val created = user(NEW_INSZ, "Dan", UserRole.WAITER)
            coEvery { r.engine.createUser(CreateUserInput(NEW_INSZ, "Dan", "waiter", null, null)) } returns created
            r.model.setNewName(" Dan ")
            r.model.setNewInsz(NEW_INSZ)
            r.model.setNewRole(NewRole.Waiter)

            r.model.addEmployee()
            runCurrent()

            coVerify(exactly = 1) { r.engine.createUser(CreateUserInput(NEW_INSZ, "Dan", "waiter", null, null)) }
            coVerify(exactly = 2) { r.engine.listUsers() }
            assertEquals("", r.state.form.name)
            assertEquals("", r.state.form.insz)
            assertNull(r.state.form.error)
            assertNull(r.state.pin)
        }

    @Test
    fun theDefaultRoleIsCashier() =
        runTest {
            val r = start()
            coEvery { r.engine.createUser(CreateUserInput(NEW_INSZ, "Dan", "cashier", null, null)) } returns
                user(NEW_INSZ, "Dan", UserRole.CASHIER)
            r.model.setNewName("Dan")
            r.model.setNewInsz(NEW_INSZ)

            r.model.addEmployee()
            runCurrent()

            assertEquals(NewRole.Cashier, NewEmployeeForm().role)
            coVerify(exactly = 1) { r.engine.createUser(CreateUserInput(NEW_INSZ, "Dan", "cashier", null, null)) }
        }

    @Test
    fun aRefusedCreationShowsTheErrorAndKeepsTheForm() =
        runTest {
            val r = start()
            val refusal = PosException.Coded("user_exists", emptyMap())
            coEvery { r.engine.createUser(any()) } throws refusal
            r.model.setNewName("Dan")
            r.model.setNewInsz(NEW_INSZ)

            r.model.addEmployee()
            runCurrent()

            assertEquals(r.env.describe(refusal), r.state.form.error)
            assertEquals("Dan", r.state.form.name)
            assertEquals(NEW_INSZ, r.state.form.insz)
        }

    @Test
    fun aManagerNeedsAChosenPinConfirmedWithTheOperatorPin() =
        runTest {
            val r = start()
            val input = CreateUserInput(NEW_INSZ, "Dan", "admin", "4321", "1234")
            coEvery { r.engine.createUser(input) } returns user(NEW_INSZ, "Dan", UserRole.ADMIN)
            r.model.setNewName("Dan")
            r.model.setNewInsz(NEW_INSZ)
            r.model.setNewRole(NewRole.Admin)

            r.model.addEmployee()
            assertEquals(SettingsPinFlow(PinPurpose.NewManager, NEW_INSZ, "Dan"), r.state.pin)
            coVerify(exactly = 0) { r.engine.createUser(any()) }

            r.model.submitChosenPin("4321")
            assertEquals(PinStage.ConfirmWithManager, r.state.pin?.stage)
            assertEquals("4321", r.state.pin?.chosen)
            coVerify(exactly = 0) { r.engine.createUser(any()) }

            r.model.confirmWithManagerPin("1234")
            coVerify(exactly = 1) { r.engine.createUser(input) }
            assertEquals("", r.state.form.name)
            assertEquals("", r.state.form.insz)

            r.model.onPinDone()
            runCurrent()
            assertNull(r.state.pin)
            coVerify(exactly = 2) { r.engine.listUsers() }
        }

    @Test
    fun aWrongOperatorPinIsThrownForTheDialogAndKeepsTheFlow() =
        runTest {
            val r = start()
            coEvery { r.engine.createUser(any()) } throws PosException.Coded("pin_invalid", emptyMap())
            r.model.setNewName("Dan")
            r.model.setNewInsz(NEW_INSZ)
            r.model.setNewRole(NewRole.Admin)
            r.model.addEmployee()
            r.model.submitChosenPin("4321")

            assertTrue(runCatching { r.model.confirmWithManagerPin("0000") }.exceptionOrNull() is PosException.Coded)

            assertEquals(PinStage.ConfirmWithManager, r.state.pin?.stage)
            assertEquals("Dan", r.state.form.name)
        }

    // ── PIN of an employee ────────────────────────────────────────

    @Test
    fun anotherEmployeesPinIsChosenThenConfirmedWithTheOperatorPin() =
        runTest {
            val r = start()
            coEvery { r.engine.setPin(CASHIER_INSZ, "4321", "1234") } returns Unit

            r.model.openPin(cashier)
            assertEquals(SettingsPinFlow(PinPurpose.Other, CASHIER_INSZ, "Bob"), r.state.pin)

            r.model.submitChosenPin("4321")
            assertEquals(PinStage.ConfirmWithManager, r.state.pin?.stage)
            coVerify(exactly = 0) { r.engine.setPin(any(), any(), any()) }

            r.model.confirmWithManagerPin("1234")
            coVerify(exactly = 1) { r.engine.setPin(CASHIER_INSZ, "4321", "1234") }

            r.model.onPinDone()
            runCurrent()
            assertNull(r.state.pin)
            coVerify(exactly = 2) { r.engine.listUsers() }
        }

    @Test
    fun theOperatorsOwnPinIsSavedAtOnceWithoutAnOperatorPin() =
        runTest {
            val r = start()
            coEvery { r.engine.setPin(ADMIN_INSZ, "4321", null) } returns Unit

            r.model.openPin(admin)
            assertEquals(PinPurpose.Own, r.state.pin?.purpose)

            r.model.submitChosenPin("4321")

            coVerify(exactly = 1) { r.engine.setPin(ADMIN_INSZ, "4321", null) }
            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
            assertEquals(r.env.text("pin_changed"), r.env.lastToast().message)
        }

    @Test
    fun aRefusedOwnPinIsThrownForTheDialog() =
        runTest {
            val r = start()
            coEvery { r.engine.setPin(ADMIN_INSZ, "4321", null) } throws PosException.Coded("pin_weak", emptyMap())
            r.model.openPin(admin)

            assertTrue(runCatching { r.model.submitChosenPin("4321") }.exceptionOrNull() is PosException.Coded)
        }

    @Test
    fun closingThePinFlowDropsIt() =
        runTest {
            val r = start()
            r.model.openPin(cashier)

            r.model.closePin()

            assertNull(r.state.pin)
        }

    @Test
    fun thePinCallsDoNothingWithoutAFlow() =
        runTest {
            val r = start()

            r.model.submitChosenPin("4321")
            r.model.confirmWithManagerPin("1234")

            coVerify(exactly = 0) { r.engine.setPin(any(), any(), any()) }
            coVerify(exactly = 0) { r.engine.createUser(any()) }
        }

    // ── Deactivation ──────────────────────────────────────────────

    @Test
    fun deactivatingTakesTwoTaps() =
        runTest {
            val r = start()
            coEvery { r.engine.deactivateUser(CASHIER_INSZ) } returns Unit

            r.model.deactivate(CASHIER_INSZ)
            assertEquals(CASHIER_INSZ, r.state.confirmDeactivate)
            coVerify(exactly = 0) { r.engine.deactivateUser(any()) }

            r.model.deactivate(CASHIER_INSZ)
            runCurrent()

            coVerify(exactly = 1) { r.engine.deactivateUser(CASHIER_INSZ) }
            assertNull(r.state.confirmDeactivate)
            coVerify(exactly = 2) { r.engine.listUsers() }
        }

    @Test
    fun tappingAnotherEmployeeMovesTheArmedButton() =
        runTest {
            val r = start()

            r.model.deactivate(CASHIER_INSZ)
            r.model.deactivate(WAITER_INSZ)

            assertEquals(WAITER_INSZ, r.state.confirmDeactivate)
            coVerify(exactly = 0) { r.engine.deactivateUser(any()) }

            r.model.cancelDeactivate()
            assertNull(r.state.confirmDeactivate)
        }

    @Test
    fun theOperatorCannotDeactivateHimself() =
        runTest {
            val r = start()

            r.model.deactivate(ADMIN_INSZ)
            r.model.deactivate(ADMIN_INSZ)
            runCurrent()

            assertNull(r.state.confirmDeactivate)
            assertTrue(r.state.isSelf(ADMIN_INSZ))
            coVerify(exactly = 0) { r.engine.deactivateUser(any()) }
        }

    @Test
    fun aRefusedDeactivationShowsTheError() =
        runTest {
            val r = start()
            val refusal = PosException.Coded("last_manager", emptyMap())
            coEvery { r.engine.deactivateUser(CASHIER_INSZ) } throws refusal

            r.model.deactivate(CASHIER_INSZ)
            r.model.deactivate(CASHIER_INSZ)
            runCurrent()

            assertEquals(r.env.describe(refusal), r.state.form.error)
            assertNull(r.state.confirmDeactivate)
        }

    // ── HubRise switch ────────────────────────────────────────────

    @Test
    fun theHubriseSwitchShowsTheEngineStateAndTheOnlineOrders() =
        runTest {
            val off = start()
            assertFalse(off.state.hubrise.enabled)
            assertFalse(off.state.hubrise.visible)
        }

    @Test
    fun theHubrisePanelStaysVisibleWhileOnlineOrdersAreOpen() =
        runTest {
            val r =
                start(who = null, open = false) { env ->
                    coEvery { env.f.engine.getIntegrations() } returns integrationsOf(false, open = 2L)
                }
            openEngine(r.env)

            assertFalse(r.state.hubrise.enabled)
            assertTrue(r.state.hubrise.visible)
            assertEquals(2L, r.state.hubrise.onlineOrdersOpen)
        }

    @Test
    fun theHubriseButtonSwitchesTheIntegrationOn() =
        runTest {
            val r = start()
            coEvery { r.engine.setHubriseEnabled(true) } returns integrationsOf(true)

            r.model.toggleHubrise()
            runCurrent()

            coVerify(exactly = 1) { r.engine.setHubriseEnabled(true) }
            assertTrue(r.state.hubrise.enabled)
            assertTrue(r.state.hubrise.visible)
            assertFalse(r.state.hubrise.busy)

            coEvery { r.engine.setHubriseEnabled(false) } returns integrationsOf(false)
            r.model.toggleHubrise()
            runCurrent()

            assertFalse(r.state.hubrise.enabled)
        }

    @Test
    fun aRefusalWhileOnlineOrdersAreUnsettledIsShownAsAnErrorToast() =
        runTest {
            val r = start(hubrise = true)
            val refusal = PosException.Coded("online_orders_open", mapOf("count" to "2"))
            coEvery { r.engine.setHubriseEnabled(false) } throws refusal

            r.model.toggleHubrise()
            runCurrent()

            assertTrue(r.state.hubrise.enabled)
            assertFalse(r.state.hubrise.busy)
            assertEquals(ToastVariant.Error, r.env.lastToast().variant)
            assertEquals(r.env.describe(refusal), r.env.lastToast().message)
        }

    @Test
    fun aSecondTapWhileTheSwitchRunsIsIgnored() =
        runTest {
            val r = start()
            val gate = CompletableDeferred<Unit>()
            coEvery { r.engine.setHubriseEnabled(true) } coAnswers
                {
                    gate.await()
                    integrationsOf(true)
                }

            r.model.toggleHubrise()
            runCurrent()
            assertTrue(r.state.hubrise.busy)
            r.model.toggleHubrise()
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { r.engine.setHubriseEnabled(any()) }
            assertFalse(r.state.hubrise.busy)
        }

    // ── Helpers ───────────────────────────────────────────────────

    @Test
    fun theAvatarInitialIsTheFirstLetterInCapitals() {
        assertEquals("A", avatarInitial("alice"))
        assertEquals("B", avatarInitial("  bob"))
        assertEquals("?", avatarInitial("   "))
        assertEquals("?", avatarInitial(""))
        assertEquals("🍣", avatarInitial("🍣 sushi"))
    }

    @Test
    fun refreshLoadsEverythingAgain() =
        runTest {
            val r = start()
            coEvery { r.engine.listUsers() } returns listOf(admin)

            r.model.refresh()

            assertEquals(listOf(admin), r.state.users)
        }
}
