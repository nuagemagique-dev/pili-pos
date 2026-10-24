package dev.nuagemagique.pilipos.feature.login

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PinDialog
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadTexts
import dev.nuagemagique.pilipos.ui.components.TrainingBanner
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.text.PosUiRoot
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.User

/** Width of the brand and checks column (720 px in `LoginView.vue`). */
private val BrandWidth = 720.dp

/** The actions of the PIN panel. */
internal data class PinCallbacks(
    val onKey: (NumPadKey) -> Unit = {},
    val onBack: () -> Unit = {},
    val onToggleEndShift: () -> Unit = {},
    val onLogin: () -> Unit = {},
    val onEndShift: () -> Unit = {},
)

/** The actions of the INSZ entry. */
internal data class InszCallbacks(
    val onToggle: () -> Unit = {},
    val onKey: (NumPadKey) -> Unit = {},
    val onSubmit: () -> Unit = {},
)

/** The actions of the first-PIN dialog. */
internal data class FirstPinCallbacks(
    val onSubmit: suspend (String) -> Unit = {},
    val describeError: (Throwable) -> String = { it.message.orEmpty() },
    val onDone: () -> Unit = {},
    val onCancel: () -> Unit = {},
)

/** Everything the operator can do on the login screen. */
internal data class LoginCallbacks(
    val onTile: (User) -> Unit = {},
    val onLanguage: (AppLanguage) -> Unit = {},
    val pin: PinCallbacks = PinCallbacks(),
    val insz: InszCallbacks = InszCallbacks(),
    val firstPin: FirstPinCallbacks = FirstPinCallbacks(),
)

/** What the effects of [LoginViewModel] ask of the navigation: after a login, Tables replaces the login screen. */
internal fun LoginEffect.perform(navigator: AppNavigator) {
    when (this) {
        LoginEffect.GoToTables -> navigator.replaceAll(TablesRoutes.TABLES)
    }
}

/**
 * The login route (`LoginView.vue`): builds [LoginViewModel] from the application graph and draws [LoginContent].
 * It does not navigate when it starts; it leaves for Tables only after an action of the operator succeeded.
 */
@Composable
internal fun LoginRoute(navigator: AppNavigator) {
    val entry = rememberEntryPoint<LoginEntryPoint>()
    val model = viewModel { LoginViewModel(entry) }
    val state by model.state.collectAsState()
    LaunchedEffect(model) { model.effects.collect { it.perform(navigator) } }
    LaunchedEffect(model) { model.pollChecks() }
    val callbacks =
        LoginCallbacks(
            onTile = model::onTile,
            onLanguage = model::setLanguage,
            pin =
                PinCallbacks(
                    onKey = model::onPinKey,
                    onBack = model::cancelPin,
                    onToggleEndShift = model::toggleEndShift,
                    onLogin = model::loginFromPanel,
                    onEndShift = model::endShiftFromPanel,
                ),
            insz = InszCallbacks(onToggle = model::toggleInsz, onKey = model::onInszKey, onSubmit = model::submitInsz),
            firstPin =
                FirstPinCallbacks(
                    onSubmit = model::chooseFirstPin,
                    describeError = model::describeError,
                    onDone = model::onFirstPinDone,
                    onCancel = model::cancelFirstPin,
                ),
        )
    PosUiRoot { LoginContent(state, callbacks) }
}

/**
 * The login screen, stateless: the brand and the start-up checks on the left, who is it and their PIN on the right,
 * laid out for 1920 x 1080 dp. The root carries the tag `placeholder_login` (PD204: the tag means "feature root").
 * Test tags: `login_user_<insz>`, `login_numpad`, `login_checks`, `login_version`, `login_language_<code>`.
 */
@Composable
internal fun LoginContent(
    state: LoginUiState,
    callbacks: LoginCallbacks,
    modifier: Modifier = Modifier,
) {
    Box(modifier = modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag("placeholder_login")) {
        Column(Modifier.fillMaxSize()) {
            if (state.training) TrainingBanner()
            Row(Modifier.fillMaxWidth().weight(1f)) {
                LoginBrandColumn(state, Modifier.width(BrandWidth).fillMaxHeight())
                LoginPeopleColumn(state, callbacks, Modifier.weight(1f).fillMaxHeight())
            }
        }
        state.firstPin?.let { FirstPinDialog(it, callbacks.firstPin) }
    }
}

/** The first-PIN dialog: a manager without a PIN chooses one, typed twice (`firstLogin` of `PinDialog.vue`). */
@Composable
private fun FirstPinDialog(
    target: FirstPinTarget,
    callbacks: FirstPinCallbacks,
) {
    val hint = vueText("pin_first_login_hint")
    val texts =
        PinPadTexts(
            title = vueText("pin_choose"),
            hint = if (target.name.isBlank()) hint else target.name + "\n" + hint,
            enterPrompt = vueText("pin_choose"),
            confirmPrompt = vueText("pin_confirm"),
            mismatch = vueText("pin_mismatch"),
            cancel = vueText("back"),
            clearLabel = uiText(R.string.ui_common_numpad_clear),
            backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
        )
    PinDialog(
        mode = PinMode.Choose,
        texts = texts,
        onSubmit = callbacks.onSubmit,
        describeError = callbacks.describeError,
        onDone = callbacks.onDone,
        onCancel = callbacks.onCancel,
    )
}
