package dev.nuagemagique.pilipos.feature.login

import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onChild
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
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
import uniffi.pos_ffi.User

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class LoginContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private val tapped = mutableListOf<User>()
    private val languages = mutableListOf<AppLanguage>()
    private val pinKeys = mutableListOf<NumPadKey>()
    private val inszKeys = mutableListOf<NumPadKey>()
    private val calls = mutableListOf<String>()
    private val submitted = mutableListOf<String>()

    private val callbacks =
        LoginCallbacks(
            onTile = { tapped += it },
            onLanguage = { languages += it },
            pin =
                PinCallbacks(
                    onKey = { pinKeys += it },
                    onBack = { calls += "pin-back" },
                    onToggleEndShift = { calls += "toggle-end-shift" },
                    onLogin = { calls += "pin-login" },
                    onEndShift = { calls += "end-shift" },
                ),
            insz =
                InszCallbacks(
                    onToggle = { calls += "toggle-insz" },
                    onKey = { inszKeys += it },
                    onSubmit = { calls += "submit-insz" },
                ),
            firstPin =
                FirstPinCallbacks(
                    onSubmit = { submitted += it },
                    describeError = { "Erreur" },
                    onDone = { calls += "first-pin-done" },
                    onCancel = { calls += "first-pin-cancel" },
                ),
        )

    private val base =
        LoginUiState(
            users = listOf(admin, cashier, waiterNoPin),
            shifts = listOf(shift(CASHIER_INSZ)),
            footer = "Chez Pili · T1",
            version = "pili-pos-android/1.2.3",
        )

    private fun show(state: LoginUiState) {
        compose.setContent {
            PiliTheme(training = state.training) {
                ProvideUiText(AppLanguage.FR, catalog) { LoginContent(state, callbacks) }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theRootCarriesTheFeatureTagAndTheVersionIsShown() {
        show(base)

        compose.onNodeWithTag("placeholder_login").assertIsDisplayed()
        compose.onNodeWithTag("login_version").assertIsDisplayed().assertTextEquals("Version pili-pos-android/1.2.3")
        compose.onNodeWithTag("login_footer").assertTextEquals("Chez Pili · T1")
        compose.onNodeWithText("Qui êtes-vous ?").assertIsDisplayed()
    }

    @Test
    fun noVersionLineWhenTheSettingsAreNotLoaded() {
        show(base.copy(version = ""))

        compose.onNodeWithTag("login_version").assertDoesNotExist()
    }

    @Test
    fun aTileShowsTheNameTheRoleAndTheShiftAndTapsThroughToTheCallback() {
        show(base)

        compose.onNodeWithTag(
            "login_user_$ADMIN_INSZ",
        ).assertTextContains("Alice", substring = true).assertTextContains("Gérant", substring = true)
        // 09:05 UTC is 11:05 in Brussels in October.
        compose.onNodeWithTag("login_user_$CASHIER_INSZ").assertTextContains("en service · 11:05", substring = true)
        compose.onNodeWithTag("login_user_$WAITER_INSZ").assertTextContains("hors service", substring = true)

        compose.onNodeWithTag("login_user_$CASHIER_INSZ").performClick()

        assertEquals(listOf(cashier), tapped)
    }

    @Test
    fun theSelectedTileIsTheOneWithThePinPanel() {
        show(base.copy(pin = PinPanel(cashier)))

        compose.onNodeWithTag("login_pin_panel").assertIsDisplayed()
        compose.onNodeWithTag("login_pin_user").assertTextEquals("Bob")
        compose.onNodeWithTag("login_pin_title").assertTextEquals("Code PIN")
    }

    @Test
    fun thePinPadSendsItsKeysAndBackClosesIt() {
        show(base.copy(pin = PinPanel(cashier)))

        compose.onNodeWithTag("pin_pad").assertIsDisplayed()
        compose.onNodeWithTag("numpad_key_7").performClick()
        compose.onNodeWithTag("numpad_clear").performClick()
        compose.onNodeWithTag("numpad_back").performClick()
        compose.onNodeWithTag("pin_cancel").performClick()

        assertEquals(listOf(NumPadKey.Digit('7'), NumPadKey.Clear, NumPadKey.Backspace), pinKeys)
        assertEquals(listOf("pin-back"), calls)
    }

    @Test
    fun theEndShiftToggleIsOnlyOfferedToAnOperatorOnShift() {
        show(base.copy(pin = PinPanel(cashier)))
        compose.onNodeWithTag("login_end_shift").assertIsDisplayed().performClick()
        assertEquals(listOf("toggle-end-shift"), calls)
    }

    @Test
    fun noEndShiftToggleForAnOperatorOffShift() {
        show(base.copy(pin = PinPanel(admin)))

        compose.onNodeWithTag("login_end_shift").assertDoesNotExist()
    }

    @Test
    fun theEndShiftModeChangesTheTitleAndTheHint() {
        show(base.copy(pin = PinPanel(cashier, endShift = true)))

        compose.onNodeWithTag("login_pin_title").assertTextEquals("Fin de service")
        compose.onNodeWithText("le code PIN termine le service").assertIsDisplayed()
    }

    // ── The badge hints (lane 5) ──────────────────────────────────

    @Test
    fun theTilesHintAtTheBadgeOnlyWithBadgeLoginAndNotInInszMode() {
        show(base.copy(badgeReaderOn = true, badgeLoginOn = true))
        compose.onNodeWithTag("login_badge_hint").assertIsDisplayed().assertTextEquals(catalog.text("login_badge_hint", AppLanguage.FR))
    }

    @Test
    fun noBadgeHintInInszMode() {
        show(base.copy(badgeReaderOn = true, badgeLoginOn = true, numpadMode = true))
        compose.onNodeWithTag("login_badge_hint").assertDoesNotExist()
    }

    @Test
    fun noBadgeHintWithoutBadgeLogin() {
        show(base.copy(badgeReaderOn = true, badgeLoginOn = false))
        compose.onNodeWithTag("login_badge_hint").assertDoesNotExist()
    }

    @Test
    fun thePinPadHintsAtTheBadgeInEndShiftModeWithAReader() {
        show(base.copy(badgeReaderOn = true, pin = PinPanel(cashier, endShift = true)))
        compose
            .onNodeWithTag("login_end_shift_badge_hint")
            .assertIsDisplayed()
            .assertTextEquals(catalog.text("end_shift_badge_hint", AppLanguage.FR))
    }

    @Test
    fun noEndShiftBadgeHintOutsideEndShiftMode() {
        show(base.copy(badgeReaderOn = true, pin = PinPanel(cashier, endShift = false)))
        compose.onNodeWithTag("login_end_shift_badge_hint").assertDoesNotExist()
    }

    @Test
    fun noEndShiftBadgeHintWithTheReaderOff() {
        show(base.copy(badgeReaderOn = false, pin = PinPanel(cashier, endShift = true)))
        compose.onNodeWithTag("login_end_shift_badge_hint").assertDoesNotExist()
    }

    @Test
    fun anOperatorWithoutAPinOnShiftGetsLoginBackAndEndShift() {
        show(base.copy(shifts = listOf(shift(WAITER_INSZ)), pin = PinPanel(waiterNoPin, error = "PIN incorrect")))

        compose.onNodeWithTag("pin_pad").assertDoesNotExist()
        compose.onNodeWithTag("login_pin_error").assertTextContains("PIN incorrect")
        compose.onNodeWithTag("login_pin_login").performClick()
        compose.onNodeWithTag("login_pin_back").performClick()
        compose.onNodeWithTag("login_end_shift").performClick()

        assertEquals(listOf("pin-login", "pin-back", "end-shift"), calls)
    }

    @Test
    fun anOperatorWithoutAPinOffShiftHasNoEndShiftButton() {
        show(base.copy(pin = PinPanel(waiterNoPin)))

        compose.onNodeWithTag("login_pin_login").assertIsDisplayed()
        compose.onNodeWithTag("login_end_shift").assertDoesNotExist()
    }

    @Test
    fun theInszEntryHasElevenBoxesAndALoginButtonEnabledOnlyWhenComplete() {
        show(base.copy(numpadMode = true, insz = "8501010012"))

        compose.onNodeWithTag("login_numpad").assertIsDisplayed()
        compose.onNodeWithTag("login_insz_digit_9").onChild().assertTextEquals("2")
        compose.onNodeWithTag("login_insz_digit_10").onChild().assertTextEquals("")
        compose.onNodeWithTag("login_insz_submit").assertIsNotEnabled()
        compose.onNodeWithTag("login_user_$ADMIN_INSZ").assertDoesNotExist()
    }

    @Test
    fun aCompleteInszCanBeSubmittedAndTheKeysAreForwarded() {
        show(base.copy(numpadMode = true, insz = "85010100123"))

        compose.onNodeWithTag("numpad_key_5").performClick()
        compose.onNodeWithTag("login_insz_submit").assertIsEnabled().performClick()

        assertEquals(listOf(NumPadKey.Digit('5')), inszKeys)
        assertEquals(listOf("submit-insz"), calls)
    }

    @Test
    fun theOtherPersonButtonIsBackInTheInszEntryAndAbsentWithoutUsers() {
        show(base)
        compose.onNodeWithTag("login_other").assertTextEquals("Autre personne (INSZ)").performClick()
        assertEquals(listOf("toggle-insz"), calls)
    }

    @Test
    fun theBackButtonLeavesTheInszEntry() {
        show(base.copy(numpadMode = true))
        compose.onNodeWithTag("login_other").assertTextEquals("Retour")
    }

    @Test
    fun withoutUsersThereIsNoOtherPersonButton() {
        show(base.copy(users = emptyList(), numpadMode = true))

        compose.onNodeWithTag("login_other").assertDoesNotExist()
        compose.onNodeWithTag("login_numpad").assertIsDisplayed()
    }

    @Test
    fun theLanguageButtonsChooseTheLanguage() {
        show(base)

        AppLanguage.entries.forEach { compose.onNodeWithTag("login_language_${it.code}").assertIsDisplayed() }
        compose.onNodeWithTag("login_language_en").performClick()
        compose.onNodeWithTag("login_language_zh").performClick()

        assertEquals(listOf(AppLanguage.EN, AppLanguage.ZH), languages)
    }

    @Test
    fun theChecksPanelListsEachDeviceWithItsAnswer() {
        val checks =
            listOf(
                StartupCheck(CheckId.Fdm, CheckState.Ok, "check_connected"),
                StartupCheck(CheckId.Ticket, CheckState.Err, "check_unreachable"),
                StartupCheck(CheckId.Kitchen, CheckState.None, "check_not_configured"),
                StartupCheck(CheckId.Hubrise, CheckState.Warn, "check_not_connected"),
                StartupCheck(CheckId.Mail, CheckState.Ok, "check_reachable"),
            )
        show(base.copy(checks = checks))

        compose.onNodeWithTag("login_checks").assertIsDisplayed()
        compose.onNodeWithTag("login_check_text_fdm").assertTextEquals("connecté")
        compose.onNodeWithTag("login_check_text_ticket").assertTextEquals("injoignable")
        compose.onNodeWithTag("login_check_text_kitchen").assertTextEquals("non configuré")
        compose.onNodeWithTag("login_check_text_hubrise").assertTextEquals("non connecté")
        compose.onNodeWithTag("login_check_text_mail").assertTextEquals("joignable")
        compose.onNodeWithText("FDM").assertIsDisplayed()
        compose.onNodeWithText("HubRise").assertIsDisplayed()
        compose.onNodeWithText("Imprimante ticket").assertIsDisplayed()
    }

    @Test
    fun theTerminalLineSaysWhyItDoesNotAnswer() {
        val checks =
            listOf(
                StartupCheck(CheckId.Fdm, CheckState.Ok, "check_connected"),
                StartupCheck(CheckId.Terminal, CheckState.Err, "check_unreachable", "connection refused"),
            )
        show(base.copy(checks = checks))

        compose.onNodeWithText("Terminal de paiement").assertIsDisplayed()
        compose.onNodeWithTag("login_check_text_terminal").assertTextEquals("injoignable")
        compose.onNodeWithTag("login_check_terminal").assert(
            SemanticsMatcher.expectValue(SemanticsProperties.ContentDescription, listOf("connection refused")),
        )
    }

    @Test
    fun anErrorLineIsShownUnderTheTitle() {
        show(base.copy(error = "PIN requis"))

        compose.onNodeWithTag("login_error").assertTextEquals("PIN requis")
    }

    @Test
    fun theTrainingBannerShowsOnlyInTrainingMode() {
        show(base)
        compose.onNodeWithTag("training_banner").assertDoesNotExist()
    }

    @Test
    fun theTrainingBannerIsShownInTrainingMode() {
        show(base.copy(training = true))
        compose.onNodeWithTag("training_banner").assertIsDisplayed()
    }

    @Test
    fun theFirstPinDialogAsksForThePinTwiceAndSubmitsItOnce() {
        show(base.copy(firstPin = FirstPinTarget(ADMIN_INSZ, "Alice")))

        compose.onNodeWithTag("pili_dialog").assertIsDisplayed()
        compose.onAllNodesWithText("Choisissez votre PIN")[0].assertExists()
        "43214321".forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.waitForIdle()

        assertEquals(listOf("4321"), submitted)
        assertEquals(listOf("first-pin-done"), calls)
    }

    @Test
    fun cancellingTheFirstPinDialogCallsBack() {
        show(base.copy(firstPin = FirstPinTarget(ADMIN_INSZ, "Alice")))

        compose.onNodeWithTag("pin_cancel").performClick()

        assertEquals(listOf("first-pin-cancel"), calls)
    }
}
