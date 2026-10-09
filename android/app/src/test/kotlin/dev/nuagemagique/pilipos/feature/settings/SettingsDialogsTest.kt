package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
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

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsDialogsTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val pinActions =
        SettingsPinActions(
            onChosen = { calls += "chosen:$it" },
            onManagerPin = { calls += "manager:$it" },
            describeError = { "err:${it.message}" },
            onDone = { calls += "done" },
            onCancel = { calls += "cancel" },
        )

    private val deactivate =
        DeactivateActions(
            onConfirm = { calls += "deactivate:$it" },
            onCancel = { calls += "cancelDeactivate" },
        )

    private fun show(state: SettingsUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) { SettingsDialogs(state, pinActions, deactivate) }
                }
            }
        }
        compose.waitForIdle()
    }

    private fun typePin(pin: String) {
        pin.forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.waitForIdle()
    }

    @Test
    fun withoutAFlowNoDialogIsDrawn() {
        show(managerState())

        compose.onNodeWithTag("settings_pin_dialog").assertDoesNotExist()
        compose.onNodeWithTag("settings_deactivate_dialog").assertDoesNotExist()
    }

    @Test
    fun anArmedDeactivationAsksForConfirmationNamingTheEmployee() {
        show(managerState().copy(confirmDeactivate = CASHIER_INSZ))

        compose.onNodeWithTag("settings_deactivate_dialog").assertIsDisplayed()
        compose.onNodeWithText("Bob", substring = true).assertIsDisplayed()

        compose.onNodeWithTag("confirm_ok").performClick()

        assertEquals(listOf("deactivate:$CASHIER_INSZ"), calls)
    }

    @Test
    fun theDeactivationCanBeCancelled() {
        show(managerState().copy(confirmDeactivate = CASHIER_INSZ))

        compose.onNodeWithTag("confirm_cancel").performClick()

        assertEquals(listOf("cancelDeactivate"), calls)
    }

    @Test
    fun theOwnPinIsTypedTwiceThenSavedAndTheFlowCloses() {
        show(managerState().copy(pin = SettingsPinFlow(PinPurpose.Own, ADMIN_INSZ, "Alice")))
        compose.onNodeWithTag("settings_pin_dialog").assertIsDisplayed()

        typePin("1234")
        typePin("1234")

        assertEquals(listOf("chosen:1234", "done"), calls)
    }

    @Test
    fun aMismatchedConfirmationSavesNothing() {
        show(managerState().copy(pin = SettingsPinFlow(PinPurpose.Own, ADMIN_INSZ, "Alice")))

        typePin("1234")
        typePin("1235")

        assertEquals(emptyList<String>(), calls)
    }

    @Test
    fun theChoiceForAnotherEmployeeLeavesTheClosingToTheManagerConfirmation() {
        show(managerState().copy(pin = SettingsPinFlow(PinPurpose.Other, CASHIER_INSZ, "Bob")))

        typePin("4321")
        typePin("4321")

        // The choose dialog moved the flow on: it does not close it.
        assertEquals(listOf("chosen:4321"), calls)
    }

    @Test
    fun theManagerConfirmationAsksForTheManagersOwnPinAndFinishesTheFlow() {
        val flow = SettingsPinFlow(PinPurpose.Other, CASHIER_INSZ, "Bob", PinStage.ConfirmWithManager, chosen = "4321")
        show(managerState().copy(pin = flow))
        compose.onNodeWithText(catalog.text("pin_confirm_manager", AppLanguage.FR)).assertIsDisplayed()

        typePin("1234")

        assertEquals(listOf("manager:1234", "done"), calls)
    }

    @Test
    fun theNewManagerFlowConfirmsWithTheCreatingManagersPin() {
        val flow = SettingsPinFlow(PinPurpose.NewManager, NEW_INSZ, "Eve", PinStage.ConfirmWithManager, chosen = "9999")
        show(managerState().copy(pin = flow))

        typePin("1234")

        assertEquals(listOf("manager:1234", "done"), calls)
    }

    @Test
    fun theCancelButtonOfAPinDialogClosesTheFlow() {
        show(managerState().copy(pin = SettingsPinFlow(PinPurpose.Own, ADMIN_INSZ, "Alice")))

        compose.onNodeWithTag("pin_cancel").performClick()

        assertEquals(listOf("cancel"), calls)
    }
}
