package dev.nuagemagique.pilipos.feature.login

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.User

private const val TITLE_SCALE = 1.4f

private val PeopleVerticalPadding = 80.dp
private val PeopleHorizontalPadding = 72.dp
private val TileMinHeight = 150.dp
private val InitialSize = 72.dp
private val SelectedRing = 3.dp
private val ShiftDotSize = 8.dp
private val InszBoxWidth = 44.dp
private val InszBoxHeight = 56.dp
private val ButtonHeight = 56.dp
private val SubmitWidth = 400.dp

/** Width of the PIN panel: the 400 px of the Vue numpad. */
private val PinPanelWidth = 400.dp
private val SubmitHeight = 80.dp

/** The Vue key of the role of a user (`role_admin`, `role_cashier`, `role_waiter`). */
private fun User.roleKey(): String = "role_" + role.name.lowercase()

/** Right column: who are you (tiles or INSZ entry), the language choice, and the PIN panel beside them. */
@Composable
internal fun LoginPeopleColumn(
    state: LoginUiState,
    callbacks: LoginCallbacks,
    modifier: Modifier = Modifier,
) {
    val dimens = PiliTheme.dimens
    Row(
        modifier =
            modifier
                .background(PiliTheme.colors.bgSecondary)
                .padding(horizontal = PeopleHorizontalPadding, vertical = PeopleVerticalPadding),
        horizontalArrangement = Arrangement.spacedBy(dimens.spaceXl + dimens.spaceLg),
    ) {
        Column(
            modifier = Modifier.weight(1f).fillMaxHeight(),
            verticalArrangement = Arrangement.spacedBy(dimens.spaceLg + dimens.spaceXs),
        ) {
            PeopleHeader(state)
            PeopleBody(state, callbacks, Modifier.weight(1f))
            PeopleFooter(state, callbacks)
        }
        state.pin?.let { LoginPinPanel(it, state, callbacks.pin, Modifier.fillMaxHeight().width(PinPanelWidth)) }
    }
}

@Composable
private fun PeopleHeader(state: LoginUiState) {
    val typography = PiliTheme.typography
    Text(
        text = vueText("who_are_you"),
        style = typography.title.copy(fontSize = typography.sizeXl * TITLE_SCALE),
        color = PiliTheme.colors.textPrimary,
    )
    state.error?.let { ErrorLine(it, Modifier.testTag("login_error")) }
}

/** The tiles, or the INSZ entry when the operator types by hand or no user is known. */
@Composable
private fun PeopleBody(
    state: LoginUiState,
    callbacks: LoginCallbacks,
    modifier: Modifier,
) {
    Box(modifier) {
        if (state.numpadMode) {
            InszEntry(state, callbacks.insz)
        } else if (state.users.isNotEmpty()) {
            LazyVerticalGrid(
                columns = GridCells.Fixed(2),
                horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                items(state.users, key = { it.insz }) { user ->
                    UserTile(user, state, callbacks.onTile)
                }
            }
        }
    }
}

/** Bottom row: the "other person" / back button, and the language choice. */
@Composable
private fun PeopleFooter(
    state: LoginUiState,
    callbacks: LoginCallbacks,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.Bottom,
    ) {
        if (state.users.isNotEmpty()) {
            PiliButton(
                text = vueText(if (state.numpadMode) "back" else "other_person_insz"),
                onClick = callbacks.insz.onToggle,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.height(ButtonHeight).testTag("login_other"),
            )
        } else {
            Spacer(Modifier)
        }
        LanguageChoice(state.language, callbacks.onLanguage)
    }
}

/** One button per language of the till; the chosen one is shown in Brume (a selection, never Volt). */
@Composable
private fun LanguageChoice(
    current: AppLanguage,
    onLanguage: (AppLanguage) -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        AppLanguage.entries.forEach { language ->
            val chosen = language == current
            Box(
                modifier =
                    Modifier
                        .height(ButtonHeight)
                        .background(if (chosen) colors.selectedBg else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                        .clickable(role = Role.RadioButton) { onLanguage(language) }
                        .padding(horizontal = PiliTheme.dimens.spaceLg)
                        .testTag("login_language_${language.code}"),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = vueText("settings_lang_name_${language.code}"),
                    style = typography.body.copy(fontSize = typography.sizePrice, fontWeight = typography.weightBold),
                    color = if (chosen) colors.selectedFg else colors.textPrimary,
                )
            }
        }
    }
}

/** The red framed refusal line (`error-msg` of `LoginView.vue`). */
@Composable
internal fun ErrorLine(
    message: String,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Text(
        text = message,
        style = PiliTheme.typography.body,
        color = colors.accentDanger,
        modifier =
            modifier
                .border(BorderStroke(1.dp, colors.accentDanger), RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .padding(horizontal = PiliTheme.dimens.spaceMd, vertical = PiliTheme.dimens.spaceSm),
    )
}

/** The tile of one operator: initial, name, role and when the shift started (`user-btn`). */
@Composable
private fun UserTile(
    user: User,
    state: LoginUiState,
    onTile: (User) -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val selected = state.pin?.user?.insz == user.insz
    val since = state.shiftStart(user.insz)
    val interaction = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusLg)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TileMinHeight)
                .tapScale(interaction)
                .background(colors.bgSurface, shape)
                .then(if (selected) Modifier.border(SelectedRing, colors.selectedBg, shape) else Modifier)
                .clickable(interactionSource = interaction, indication = null, role = Role.Button) { onTile(user) }
                .padding(PiliTheme.dimens.spaceLg)
                .testTag("login_user_${user.insz}"),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd + PiliTheme.dimens.spaceXs),
    ) {
        Box(
            modifier = Modifier.size(InitialSize).background(if (selected) colors.selectedBg else colors.border, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = user.name.take(1),
                style = typography.title,
                color = if (selected) colors.selectedFg else colors.textPrimary,
            )
        }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs / 2)) {
            Text(
                text = user.name,
                style = typography.title,
                color = colors.textPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(vueText(user.roleKey()), style = typography.body, color = colors.textSecondary)
            ShiftLine(since)
        }
    }
}

/** "On shift since 14:05" in green, or "off shift" in grey, with a dot. */
@Composable
private fun ShiftLine(since: String?) {
    val colors = PiliTheme.colors
    val tint = if (since != null) colors.accentSuccess else colors.textSecondary
    val text =
        if (since != null) {
            vueText("on_shift_since", "time" to PosDateTime.formatTimestamp(since, PosDateTime::formatTime))
        } else {
            vueText("off_shift")
        }
    Row(
        modifier = Modifier.padding(top = PiliTheme.dimens.spaceXs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        Box(Modifier.size(ShiftDotSize).background(tint, CircleShape))
        Text(
            text = text,
            style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeSm),
            color = tint,
        )
    }
}

/** The INSZ entry: 11 boxes, the numeric pad and the login button (`numpad-section`). */
@Composable
private fun InszEntry(
    state: LoginUiState,
    callbacks: InszCallbacks,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
        Text(
            text = uiText(R.string.ui_login_insz),
            style = typography.figure.copy(fontSize = typography.sizeSm, fontWeight = typography.weightBold, letterSpacing = 0.08.em),
            color = colors.textSecondary,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs + PiliTheme.dimens.spaceXs / 2)) {
            repeat(LoginUiState.INSZ_LENGTH) { i ->
                val digit = state.insz.getOrNull(i)
                Box(
                    modifier =
                        Modifier
                            .size(InszBoxWidth, InszBoxHeight)
                            .border(
                                1.dp,
                                if (digit != null) colors.textPrimary else colors.border,
                                RoundedCornerShape(PiliTheme.dimens.radiusSm),
                            )
                            .testTag("login_insz_digit_$i"),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = digit?.toString().orEmpty(),
                        style = typography.figure.copy(fontSize = typography.sizeXl, fontWeight = typography.weightBold),
                        color = colors.textPrimary,
                    )
                }
            }
        }
        NumPad(
            onKey = callbacks.onKey,
            clearLabel = uiText(R.string.ui_common_numpad_clear),
            backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
            modifier = Modifier.testTag("login_numpad"),
        )
        PiliButton(
            text = vueText("login"),
            onClick = callbacks.onSubmit,
            enabled = state.insz.length == LoginUiState.INSZ_LENGTH,
            modifier = Modifier.width(SubmitWidth).height(SubmitHeight).testTag("login_insz_submit"),
        )
    }
}
