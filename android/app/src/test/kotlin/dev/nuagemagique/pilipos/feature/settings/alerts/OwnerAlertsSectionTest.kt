package dev.nuagemagique.pilipos.feature.settings.alerts

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
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
class OwnerAlertsSectionTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        OwnerAlertsCallbacks(
            onToggleEnabled = { calls += "enabled" },
            onToggleKind = { calls += "kind:$it" },
            onLanguage = { calls += "lang:$it" },
            onLabel = { calls += "label:$it" },
            onEdit = { calls += "edit:${it.tag}" },
            onSave = { calls += "save" },
            onTest = { calls += "test" },
        )

    private fun text(
        key: String,
        vars: Map<String, Any> = emptyMap(),
    ): String = catalog.text(key, AppLanguage.FR, vars)

    private fun stateOf(
        settings: uniffi.pos_ffi.OwnerAlerts = ownerAlerts(),
        edited: uniffi.pos_ffi.OwnerAlerts = settings,
    ) = OwnerAlertsUiState(manager = true, settings = settings, draft = edited.toDraft())

    private fun show(state: OwnerAlertsUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Column(Modifier.verticalScroll(rememberScrollState())) { OwnerAlertsSection(state, callbacks) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun offWithoutATokenItSaysSoAndOffersNoTest() {
        show(stateOf(ownerAlerts(token = false)))

        compose.onNodeWithTag("alerts_enabled").assertTextContains(text("alerts_state_off"))
        compose.onNodeWithTag("alerts_token").assertTextEquals(text("alerts_token_missing_pill"))
        compose.onNodeWithTag("alerts_test").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithTag("alerts_save").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithTag("alerts_log_empty").performScrollTo().assertTextEquals(text("alerts_log_empty"))
    }

    @Test
    fun aTokenAndAChatAllowTheTest() {
        show(stateOf(ownerAlerts(enabled = true, chatId = "-100")).copy(settings = ownerAlerts(enabled = true, chatId = "-100")))

        compose.onNodeWithTag("alerts_token").assertTextEquals(text("alerts_token_ok"))
        compose.onNodeWithTag("alerts_chat_id").assertTextEquals("-100")
        compose.onNodeWithTag("alerts_test").performScrollTo().assertIsEnabled().performClick()

        assertEquals(listOf("test"), calls)
    }

    @Test
    fun withoutAChatTheTestIsOff() {
        show(stateOf(ownerAlerts(chatId = "")))

        compose.onNodeWithTag("alerts_chat_id").assertTextEquals(text("alerts_chat_id_unset"))
        compose.onNodeWithTag("alerts_test").performScrollTo().assertIsNotEnabled()
    }

    @Test
    fun aBusyPanelOffersNeitherSaveNorTest() {
        show(stateOf(ownerAlerts(chatId = "7"), ownerAlerts(enabled = true, chatId = "7")).copy(busy = true))

        compose.onNodeWithTag("alerts_save").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithTag("alerts_test").performScrollTo().assertIsNotEnabled()
    }

    @Test
    fun theSwitchesTheLanguageAndTheLabelCallBack() {
        show(stateOf())

        compose.onNodeWithTag("alerts_enabled").performClick()
        compose.onNodeWithTag("alerts_lang_zh").performScrollTo().performClick()
        compose.onNodeWithTag("alerts_kind_printer").performScrollTo().performClick()
        compose.onNodeWithTag("alerts_kind_online_waiting").performScrollTo().performClick()
        compose.onNodeWithTag("alerts_label").performScrollTo().performTextInput("Salle")

        assertEquals(listOf("enabled", "lang:zh", "kind:printer", "kind:online_waiting", "label:Salle"), calls)
    }

    @Test
    fun theChosenLanguageIsSelected() {
        show(stateOf(ownerAlerts(language = "zh")))

        compose.onNodeWithTag("alerts_lang_zh").performScrollTo().assertIsSelected()
    }

    @Test
    fun theNumbersShowTheirUnitAndOpenTheirDialog() {
        show(stateOf(ownerAlerts(reminder = 0L)))

        compose.onNodeWithTag("alerts_z_hour").performScrollTo().assertTextEquals(text("alerts_value_hour", mapOf("n" to 5)))
        compose.onNodeWithTag("alerts_online_wait").performScrollTo().assertTextEquals(text("alerts_value_minutes", mapOf("n" to 5)))
        compose.onNodeWithTag("alerts_reminder").performScrollTo().assertTextEquals(text("alerts_value_never"))

        compose.onNodeWithTag("alerts_chat_id").performScrollTo().performClick()
        compose.onNodeWithTag("alerts_z_hour").performClick()
        compose.onNodeWithTag("alerts_online_wait").performClick()
        compose.onNodeWithTag("alerts_reminder").performClick()

        assertEquals(listOf("edit:chat_id", "edit:z_hour", "edit:online_wait", "edit:reminder"), calls)
    }

    @Test
    fun aChangedDraftCanBeSavedAndSaysSoAfterwards() {
        show(stateOf(ownerAlerts(), ownerAlerts(enabled = true)))
        compose.onNodeWithTag("alerts_save").performScrollTo().assertIsEnabled().performClick()
        assertEquals(listOf("save"), calls)
    }

    @Test
    fun theSavedMarkShowsOnlyOnACleanDraft() {
        show(stateOf().copy(saved = true))
        compose.onNodeWithTag("alerts_saved").performScrollTo().assertTextEquals(text("alerts_saved"))
    }

    @Test
    fun theSavedMarkIsHiddenWhileTheDraftIsDirty() {
        show(stateOf(ownerAlerts(), ownerAlerts(enabled = true)).copy(saved = true))
        compose.onNodeWithTag("alerts_saved").assertDoesNotExist()
    }

    @Test
    fun aSentTestAndARefusalShowInThePanel() {
        show(stateOf().copy(testSent = true, error = "Le message n'est pas parti : HTTP 401"))

        compose.onNodeWithTag("alerts_test_sent").performScrollTo().assertTextEquals(text("alerts_test_sent"))
        compose.onNodeWithTag("alerts_error").assertTextEquals("Le message n'est pas parti : HTTP 401")
    }

    @Test
    fun theLogListsEachMessageWithItsOutcome() {
        val rows = listOf(logRow(2L, "test", "test"), logRow(1L, "raised", "fdm", "suppressed"))
        show(stateOf().copy(log = rows))

        compose.onNodeWithTag("alerts_log").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("alerts_log_row_2").assertTextContains(text("alerts_transition_test"), substring = true)
        compose.onNodeWithTag("alerts_log_row_2").assertTextContains(text("alerts_outcome_sent"), substring = true)
        compose.onNodeWithTag("alerts_log_row_1").assertTextContains(text("alerts_kind_fdm"), substring = true)
        compose.onNodeWithTag("alerts_log_row_1").assertTextContains(text("alerts_outcome_suppressed"), substring = true)
        compose.onNodeWithTag("alerts_log_empty").assertDoesNotExist()
    }

    @Test
    fun aPanelNotLoadedYetShowsOnlyItsTitle() {
        show(OwnerAlertsUiState(manager = true))

        compose.onNodeWithTag("alerts_section").assertIsDisplayed()
        compose.onNodeWithTag("alerts_enabled").assertDoesNotExist()
        compose.onNodeWithTag("alerts_token").assertDoesNotExist()
    }

    @Test
    fun theNumberDialogShowsTheValueAndTheMinusOnlyForAChatId() {
        val actions = mutableListOf<String>()
        val entry = numberEntryFor(AlertNumber.ChatId, "-42")
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    NumberEntryDialog(
                        entry,
                        NumberEntryActions(
                            onKey = { actions += "key:$it" },
                            onMinus = { actions += "minus" },
                            onConfirm = { actions += "ok" },
                            onCancel = { actions += "cancel" },
                        ),
                    )
                }
            }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("number_entry_value").assertTextEquals("-42")
        compose.onNodeWithTag("number_entry_minus").performClick()
        compose.onNodeWithTag("numpad_key_7").performClick()
        compose.onNodeWithTag("number_entry_ok").assertIsEnabled().performClick()
        compose.onNodeWithTag("number_entry_cancel").performClick()

        assertEquals(listOf("minus", "key:${NumPadKey.Digit('7')}", "ok", "cancel"), actions)
    }

    @Test
    fun aSmallNumberHasNoMinusAndTheConfirmWaitsForAValidValue() {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    NumberEntryDialog(NumberEntry(AlertNumber.OnlineWait), NumberEntryActions())
                }
            }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("number_entry_minus").assertDoesNotExist()
        compose.onNodeWithTag("number_entry_value").assertTextEquals("0")
        compose.onNodeWithTag("number_entry_ok").assertIsNotEnabled()
    }
}
