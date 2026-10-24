package dev.nuagemagique.pilipos.feature.login

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadState
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.UserRole

/** Goldens of the login screen at 1920 x 1080 dp: the tiles, the PIN panel, the INSZ entry and the first-PIN dialog. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class LoginScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val checks =
        listOf(
            StartupCheck(CheckId.Fdm, CheckState.Ok, "check_connected"),
            StartupCheck(CheckId.Ticket, CheckState.Ok, "check_ready"),
            StartupCheck(CheckId.Kitchen, CheckState.None, "check_not_configured"),
            StartupCheck(CheckId.Hubrise, CheckState.Warn, "check_not_connected"),
            StartupCheck(CheckId.Mail, CheckState.Err, "check_unreachable"),
        )

    private val base =
        LoginUiState(
            users = listOf(admin, cashier, waiterNoPin, user("96040400456", "Dominique Lefebvre-Martin", UserRole.CASHIER)),
            shifts = listOf(shift(CASHIER_INSZ)),
            checks = checks,
            footer = "Chez Pili · T1",
            version = "pili-pos-android/1.2.3",
            language = AppLanguage.FR,
        )

    private fun capture(
        state: LoginUiState,
        name: String,
    ) {
        val catalog = I18nTestSupport.catalog()
        rule.setContent {
            PiliTheme(training = state.training) {
                ProvideUiText(AppLanguage.FR, catalog) { LoginContent(state, LoginCallbacks()) }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/login/$name.png")
    }

    @Test
    fun tiles() {
        capture(base, "login_tiles")
    }

    @Test
    fun tilesWithBadgeHint() {
        // The reader is on and a badge logs the cashiers in: "Ou présentez votre badge" under the tiles.
        capture(base.copy(badgeReaderOn = true, badgeLoginOn = true), "login_badge")
    }

    @Test
    fun endShiftBadgeHint() {
        // The PIN pad in "Fin de service" mode with a reader on: a badge ends the shift too.
        val pad = PinPadState(PinMode.Verify)
        capture(
            base.copy(badgeReaderOn = true, pin = PinPanel(cashier, endShift = true, pad = pad)),
            "login_end_shift_badge",
        )
    }

    @Test
    fun pin() {
        val pad = PinPadState(PinMode.Verify)
        pad.apply(NumPadKey.Digit('4'))
        pad.apply(NumPadKey.Digit('2'))
        capture(base.copy(pin = PinPanel(cashier, pad = pad)), "login_pin")
    }

    @Test
    fun numpad() {
        capture(base.copy(numpadMode = true, insz = "8501010012"), "login_numpad")
    }

    @Test
    fun firstPin() {
        capture(base.copy(firstPin = FirstPinTarget(ADMIN_INSZ, "Alice")), "login_first_pin")
    }
}
