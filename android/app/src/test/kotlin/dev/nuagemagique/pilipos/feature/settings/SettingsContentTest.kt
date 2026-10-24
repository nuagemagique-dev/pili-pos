package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
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
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.User

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val navigator = RecordingNavigator()
    private val taps = mutableListOf<String>()

    private val callbacks =
        SettingsCallbacks(
            onToggleTraining = { taps += "training" },
            onLanguage = { taps += "language:${it.code}" },
            onProductLabel = { taps += "label:${it.code}" },
            onTablePlanInput = { taps += "tables:$it" },
            onSaveTablePlan = { taps += "tables:save" },
            onRow = { navigator.navigate(it.route) },
            onNewName = { taps += "name:$it" },
            onNewInsz = { taps += "insz:$it" },
            onNewRole = { taps += "role:${it.code}" },
            onAddEmployee = { taps += "add" },
            onPin = { taps += "pin:${it.insz}" },
            onDeactivate = { taps += "deactivate:$it" },
            onPresence = { taps += "presence:${it.insz}" },
            onToggleHubrise = { taps += "hubrise" },
            onReloadDeliveries = { taps += "reload" },
        )

    private fun text(
        key: String,
        vars: Map<String, Any> = emptyMap(),
    ): String = catalog.text(key, AppLanguage.FR, vars)

    private fun show(
        state: SettingsUiState,
        hubriseSection: @Composable () -> Unit = {},
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { SettingsContent(state, callbacks, hubriseSection = hubriseSection) }
            }
        }
        compose.waitForIdle()
    }

    private fun top(tag: String): Float = compose.onNodeWithTag(tag).fetchSemanticsNode().positionInRoot.y

    // ── Sections by role ──────────────────────────────────────────

    @Test
    fun aManagerSeesEveryPanelOfTheThreeColumns() {
        show(managerState())

        listOf(
            "settings_training",
            "settings_language",
            "settings_labels",
            "settings_rows",
            "settings_workers",
            "settings_info",
            "integration_switch",
            "settings_deliveries",
        ).forEach { compose.onNodeWithTag(it).assertIsDisplayed() }
    }

    @Test
    fun aNonManagerSeesNoEmployeesButTheRowsAndTheOtherPanels() {
        show(workerState())

        compose.onNodeWithTag("settings_workers").assertDoesNotExist()
        compose.onNodeWithTag("settings_rows").assertIsDisplayed()
        compose.onNodeWithTag("settings_training").assertIsDisplayed()
        compose.onNodeWithTag("settings_language").assertIsDisplayed()
        compose.onNodeWithTag("settings_info").assertIsDisplayed()
        compose.onNodeWithTag("integration_switch").assertIsDisplayed()
    }

    @Test
    fun theHubriseSectionIsDrawnOnlyWhenTheSwitchMakesItVisible() {
        val section: @Composable () -> Unit = { Box(Modifier.fillMaxSize().testTag("hubrise_slot")) }
        show(managerState(hubrise = HubriseSwitchState(enabled = false, visible = false)), section)
        compose.onNodeWithTag("hubrise_slot").assertDoesNotExist()
    }

    @Test
    fun theHubriseSectionShowsWhileTheSwitchIsOn() {
        val section: @Composable () -> Unit = { Text("slot", Modifier.testTag("hubrise_slot")) }
        show(managerState(hubrise = hubriseOn()), section)
        compose.onNodeWithTag("hubrise_slot").assertExists()
    }

    // ── Rows (UI-PD308) ───────────────────────────────────────────

    @Test
    fun theRowsAreDrawnInTheOrderGivenWithATagEach() {
        show(managerState())

        listOf(shownOpen, shownAbout, shownExport).forEach { compose.onNodeWithTag("settings_row_" + it.id).assertIsDisplayed() }
        assertTrue(top("settings_row_a.open") < top("settings_row_platform.about"))
        assertTrue(top("settings_row_platform.about") < top("settings_row_platform.export"))
    }

    @Test
    fun aNonManagerGetsOnlyTheRowsOpenToEveryRole() {
        val all = listOf(shownOpen, shownAbout, shownExport)
        show(workerState(rows = visibleRows(all, isManager = false)))

        compose.onNodeWithTag("settings_row_a.open").assertIsDisplayed()
        compose.onNodeWithTag("settings_row_platform.about").assertDoesNotExist()
        compose.onNodeWithTag("settings_row_platform.export").assertDoesNotExist()
    }

    @Test
    fun aRowTapOpensItsRouteOnTheNavigatorAndNothingElse() {
        show(managerState())

        compose.onNodeWithTag("settings_row_platform.about").performClick()
        compose.onNodeWithTag("settings_row_platform.export").performClick()

        assertEquals(listOf("navigate:${shownAbout.route}", "navigate:${shownExport.route}"), navigator.calls)
    }

    @Test
    fun aRowShowsItsTitle() {
        show(managerState())

        compose.onNodeWithTag("settings_row_a.open").assertTextContains("Aucune table configurée.")
    }

    // ── Training, language, product label ────────────────────────

    @Test
    fun theTrainingToggleShowsTheStateAndCallsBack() {
        show(managerState())
        compose.onNodeWithTag("settings_training_toggle").assertTextContains(text("settings_training_off"))

        compose.onNodeWithTag("settings_training_toggle").performClick()

        assertEquals(listOf("training"), taps)
    }

    @Test
    fun theTrainingToggleShowsOnWhenTrainingIsOn() {
        show(managerState().copy(training = true))

        compose.onNodeWithTag("settings_training_toggle").assertTextContains(text("settings_training_on"))
    }

    @Test
    fun theLanguageButtonsMarkTheCurrentOneAndCallBack() {
        show(managerState().copy(language = AppLanguage.FR))

        compose.onNodeWithTag("settings_lang_fr").assertIsSelected()
        compose.onNodeWithTag("settings_lang_zh").performClick()
        compose.onNodeWithTag("settings_lang_en").performClick()

        assertEquals(listOf("language:zh", "language:en"), taps)
    }

    @Test
    fun theProductLabelButtonsMarkTheCurrentOneAndCallBack() {
        show(managerState().copy(productLabel = ProductLabel.NAME))

        compose.onNodeWithTag("settings_label_name").assertIsSelected()
        compose.onNodeWithTag("settings_label_code").performClick()

        assertEquals(listOf("label:code"), taps)
    }

    // ── Table plan ────────────────────────────────────────────────

    @Test
    fun aManagerSeesTheTablePlanAndSavesWhatTheFieldHolds() {
        show(managerState().copy(tablePlan = TablePlanState(plan = "1-12, 20-25", count = 18L, input = "1-12, 20-25")))

        compose.onNodeWithTag("settings_tables_current").assertTextEquals("18 tables : 1-12, 20-25")
        compose.onNodeWithTag("settings_tables_input").performTextInput("9")
        compose.onNodeWithTag("settings_tables_save").performClick()

        assertEquals(listOf("tables:91-12, 20-25", "tables:save"), taps)
    }

    @Test
    fun aRefusedTablePlanShowsItsReason() {
        show(managerState().copy(tablePlan = TablePlanState(plan = "1-20", count = 20L, input = "A1", error = "format")))
        compose.onNodeWithTag("settings_tables_error").assertTextEquals("format")
    }

    @Test
    fun aWorkerSeesNoTablePlan() {
        show(workerState())
        compose.onNodeWithTag("settings_tables").assertDoesNotExist()
    }

    // ── Employees ─────────────────────────────────────────────────

    private fun User.tag(suffix: String) = "worker_$insz$suffix"

    @Test
    fun everyEmployeeIsListedWithRoleAndPinState() {
        show(managerState())

        compose.onNodeWithTag(admin.tag("")).assertIsDisplayed()
        compose.onNodeWithTag(admin.tag("_meta")).assertTextEquals("Gérant · ${text("settings_pin_set")}")
        compose.onNodeWithTag(waiter.tag("_meta")).assertTextEquals("Apprenti · ${text("settings_pin_missing")}")
    }

    @Test
    fun theOperatorHasNoDeactivateButtonButTheOthersDo() {
        show(managerState())

        compose.onNodeWithTag(admin.tag("_pin")).assertIsDisplayed()
        compose.onNodeWithTag(admin.tag("_deactivate")).assertDoesNotExist()
        compose.onNodeWithTag(cashier.tag("_deactivate")).assertIsDisplayed()
    }

    @Test
    fun thePinAndDeactivateButtonsCallBackWithTheEmployee() {
        show(managerState())

        compose.onNodeWithTag(cashier.tag("_pin")).performClick()
        compose.onNodeWithTag(cashier.tag("_deactivate")).performClick()

        assertEquals(listOf("pin:$CASHIER_INSZ", "deactivate:$CASHIER_INSZ"), taps)
    }

    @Test
    fun theNoteAndTheStateOfEachEmployeeShow() {
        val since = "2026-10-17T10:02:00Z"
        show(managerState().copy(fdmConnected = true, openShifts = mapOf(CASHIER_INSZ to since)))

        compose.onNodeWithTag("settings_presence_note").assertTextEquals(text("settings_presence_note"))
        compose.onNodeWithTag("settings_presence_block").assertDoesNotExist()
        compose.onNodeWithTag(cashier.tag("_presence")).assertTextEquals(text("settings_on_shift_since", mapOf("time" to "12:02")))
        compose.onNodeWithTag(waiter.tag("_presence")).assertTextEquals(text("settings_off_shift"))
    }

    @Test
    fun aManagerGetsArriveeOrDepartButOnlyForTheOthers() {
        show(managerState().copy(fdmConnected = true, openShifts = mapOf(CASHIER_INSZ to "2026-10-17T10:02:00Z")))

        compose.onNodeWithTag(admin.tag("_presence_in")).assertDoesNotExist()
        compose.onNodeWithTag(admin.tag("_presence_out")).assertDoesNotExist()
        compose.onNodeWithTag(waiter.tag("_presence_in")).assertTextEquals(text("settings_presence_in"))
        compose.onNodeWithTag(cashier.tag("_presence_out")).assertTextEquals(text("settings_presence_out"))
        compose.onNodeWithTag(waiter.tag("_presence_in")).performClick()
        compose.onNodeWithTag(cashier.tag("_presence_out")).performClick()

        assertEquals(listOf("presence:$WAITER_INSZ", "presence:$CASHIER_INSZ"), taps)
    }

    @Test
    fun theButtonsAreOffWithTheNoteWhileTheFdmIsDown() {
        show(managerState().copy(fdmConnected = false))

        compose.onNodeWithTag("settings_presence_block").assertTextEquals(text("settings_presence_fdm_down"))
        compose.onNodeWithTag(waiter.tag("_presence_in")).assertIsNotEnabled()
    }

    @Test
    fun theButtonsAreOffWithTheTrainingTextInTrainingMode() {
        show(managerState().copy(fdmConnected = true, training = true))

        compose.onNodeWithTag("settings_presence_block").assertTextEquals(text("err_presence_training"))
        compose.onNodeWithTag(waiter.tag("_presence_in")).assertIsNotEnabled()
    }

    @Test
    fun aNonManagerSeesNoPresenceButton() {
        show(workerState().copy(users = listOf(admin, cashier)))

        compose.onNodeWithTag(admin.tag("_presence_in")).assertDoesNotExist()
    }

    @Test
    fun aPresenceOnlyRowHasItsLabelNoPinButtonAndKeepsDeactivate() {
        show(managerState().copy(fdmConnected = true, users = listOf(admin, cook)))

        compose.onNodeWithTag(cook.tag("_meta")).assertTextEquals(text("role_presence"))
        compose.onNodeWithTag(cook.tag("_pin")).assertDoesNotExist()
        compose.onNodeWithTag(cook.tag("_deactivate")).assertIsDisplayed()
        compose.onNodeWithTag(cook.tag("_presence_in")).assertIsEnabled()
    }

    @Test
    fun theFourthProfileChoiceIsOfferedAndForwarded() {
        show(managerState().copy(form = NewEmployeeForm(role = NewRole.Presence)))

        compose.onNodeWithTag("settings_role_presence").assertIsSelected()
        compose.onNodeWithTag("settings_role_waiter").performClick()
        compose.onNodeWithTag("settings_role_presence").performClick()
        assertEquals(listOf("role:waiter", "role:presence"), taps)
    }

    @Test
    fun theAddFormForwardsTheFieldsTheRoleAndTheAddButton() {
        compose.setContent {
            var state by remember { mutableStateOf(managerState()) }
            val typing =
                callbacks.copy(
                    onNewName = {
                        taps += "name:$it"
                        state = state.copy(form = state.form.copy(name = it))
                    },
                    onNewInsz = {
                        taps += "insz:$it"
                        state = state.copy(form = state.form.copy(insz = it))
                    },
                )
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { SettingsContent(state, typing) } }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("settings_new_name").performTextInput("Eve")
        compose.onNodeWithTag("settings_new_insz").performTextInput("99040400456")
        compose.onNodeWithTag("settings_role_waiter").performClick()
        compose.onNodeWithTag("settings_role_admin").performClick()
        compose.onNodeWithTag("settings_add").performClick()

        assertEquals(listOf("name:Eve", "insz:99040400456", "role:waiter", "role:admin", "add"), taps)
    }

    @Test
    fun theChosenRoleIsMarked() {
        show(managerState().copy(form = NewEmployeeForm(role = NewRole.Waiter)))

        compose.onNodeWithTag("settings_role_waiter").assertIsSelected()
    }

    @Test
    fun theFormErrorShowsUnderTheFields() {
        show(managerState().copy(form = NewEmployeeForm(error = text("settings_err_11_digits_required"))))

        compose.onNodeWithTag("settings_add_error").assertTextEquals(text("settings_err_11_digits_required"))
    }

    // ── Information, switch, deliveries ───────────────────────────

    @Test
    fun theInformationPanelShowsTheEngineValues() {
        show(managerState())

        compose.onNodeWithTag("settings_info").assertIsDisplayed()
        compose.onNodeWithText("POS1").assertIsDisplayed()
        compose.onNodeWithText("T1").assertIsDisplayed()
        compose.onNodeWithText("Chez Pili").assertIsDisplayed()
        compose.onNodeWithText("http://fdm").assertIsDisplayed()
        compose.onNodeWithText(text("printer_none")).assertIsDisplayed()
    }

    private fun terminalRow(
        kind: String,
        label: String,
    ) {
        show(managerState().copy(info = engineSettings().copy(paymentTerminal = kind)))
        compose.onNodeWithText(catalog.text(R.string.ui_eft_terminal_label, AppLanguage.FR)).assertIsDisplayed()
        compose.onNodeWithText(label).assertIsDisplayed()
    }

    @Test
    fun theInformationPanelShowsAWorldlineTerminal() = terminalRow("worldline_terminal_api", "Worldline")

    @Test
    fun theInformationPanelShowsACcvCloudTerminal() = terminalRow("ccv_payment_api", "CCV (cloud)")

    @Test
    fun theInformationPanelShowsACcvLocalNetworkTerminal() = terminalRow("ccv_opi", "CCV (réseau local)")

    @Test
    fun withoutATerminalTheInformationPanelSaysCardsAreEnteredByHand() {
        show(managerState())

        compose.onNodeWithText(catalog.text(R.string.ui_eft_terminal_none, AppLanguage.FR)).assertIsDisplayed()
    }

    @Test
    fun theSwitchShowsOffAndTurnsOnWithOneTap() {
        show(managerState())

        compose.onNodeWithTag("integration_pill").assertTextEquals(text("integrations_off"))
        compose.onNodeWithTag("integration_toggle").assertTextContains(text("integrations_turn_on"))
        compose.onNodeWithTag("integration_toggle").performClick()

        assertEquals(listOf("hubrise"), taps)
    }

    @Test
    fun theSwitchShowsOnWithTheSettleNoteAndTheLogInBrusselsTime() {
        val log = listOf(switchChange(true), switchChange(false, source = "config", by = null, at = "2026-10-08T22:30:00Z"))
        show(managerState(hubrise = hubriseOn(open = 2L, log = log)))

        compose.onNodeWithTag("integration_pill").assertTextEquals(text("integrations_on"))
        compose.onNodeWithTag("integration_toggle").assertTextContains(text("integrations_turn_off"))
        compose.onNodeWithTag("integration_settle").assertTextEquals(text("integrations_settle_first", mapOf("count" to 2L)))
        // 09:05 UTC is 11:05 in Brussels in October; 22:30 UTC on the 8th is 00:30 on the 9th.
        compose.onNodeWithTag("integration_log_0").assertTextContains("09/10 11:05", substring = true)
        compose.onNodeWithTag(
            "integration_log_0",
        ).assertTextContains(text("integrations_source_admin", mapOf("who" to "Alice")), substring = true)
        compose.onNodeWithTag("integration_log_1").assertTextContains("09/10 00:30", substring = true)
        compose.onNodeWithTag("integration_log_1").assertTextContains(text("integrations_source_config"), substring = true)
    }

    @Test
    fun theSwitchButtonIsDisabledWhileASwitchRuns() {
        show(managerState(hubrise = hubriseOn().copy(busy = true)))

        compose.onNodeWithTag("integration_toggle").assertIsNotEnabled()
    }

    @Test
    fun theLogShowsAtMostFiveChanges() {
        val log = List(7) { switchChange(it % 2 == 0, by = "Gérant$it") }
        show(managerState(hubrise = hubriseOn(log = log)))

        compose.onNodeWithTag("integration_log_4").assertTextContains("Gérant4", substring = true)
        compose.onNodeWithTag("integration_log_5").assertDoesNotExist()
    }

    @Test
    fun theDeliveriesShowStatusDateAttemptsAndTheLastErrorInBrusselsTime() {
        show(managerState())

        compose.onNodeWithTag("delivery_1").assertExists()
        compose.onNodeWithTag("delivery_1_status").assertTextEquals(text("delivery_status_sent"))
        compose.onNodeWithTag("delivery_1_date").assertTextEquals("09/10 11:05")
        compose.onNodeWithTag("delivery_2_status").assertTextEquals(text("delivery_status_failed"))
        compose.onNodeWithTag("delivery_2_error").assertTextContains("smtp 550", substring = true)
        compose.onNodeWithTag("delivery_1_error").assertDoesNotExist()
        compose.onNodeWithTag("deliveries_empty").assertDoesNotExist()
    }

    @Test
    fun withoutDeliveriesTheEmptyNoteShows() {
        show(managerState().copy(deliveries = emptyList()))

        compose.onNodeWithTag("deliveries_empty").assertTextEquals(text("delivery_settings_empty"))
    }

    @Test
    fun theMailPillsFollowTheMailStatus() {
        show(managerState().copy(mailStatus = mail(configured = true, reachable = false)))
        compose.onNodeWithTag("mail_status_configured").assertTextEquals(text("delivery_settings_configured"))
        compose.onNodeWithTag("mail_status_reachable").assertTextEquals(text("delivery_settings_unreachable"))
    }

    @Test
    fun withoutAMailServiceOnlyTheNotConfiguredPillShows() {
        show(managerState().copy(mailStatus = null))

        compose.onNodeWithTag("mail_status_configured").assertTextEquals(text("delivery_settings_not_configured"))
        compose.onNodeWithTag("mail_status_reachable").assertDoesNotExist()
    }

    @Test
    fun theRefreshButtonCallsBackAndIsOffWhileLoading() {
        show(managerState())
        compose.onNodeWithTag("deliveries_refresh").assertIsEnabled().performClick()
        assertEquals(listOf("reload"), taps)
    }

    @Test
    fun theRefreshButtonIsDisabledWhileLoading() {
        show(managerState().copy(deliveriesLoading = true))

        compose.onNodeWithTag("deliveries_refresh").assertIsNotEnabled()
    }

    // ── Narrow window ─────────────────────────────────────────────

    @Test
    @Config(qualifiers = "w320dp-h470dp-mdpi")
    fun onANarrowWindowTheRowsComeFirstAndStayVisibleWithoutScrolling() {
        show(workerState())

        compose.onNodeWithTag("settings_row_a.open").assertIsDisplayed()
        assertTrue(top("settings_rows") < top("settings_training"))
    }

    @Test
    @Config(qualifiers = "w320dp-h470dp-mdpi")
    fun onANarrowWindowTheEmployeesJoinTheSingleColumnForAManager() {
        show(managerState())

        compose.onNodeWithTag("settings_rows").assertIsDisplayed()
        compose.onNodeWithTag("settings_workers").assertExists()
        compose.onNodeWithTag("settings_deliveries").assertExists()
    }
}
