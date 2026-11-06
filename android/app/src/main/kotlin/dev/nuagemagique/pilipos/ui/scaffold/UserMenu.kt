package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Password
import androidx.compose.material.icons.filled.Translate
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.state.LanguageChoices
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val MenuWidth = 400.dp
private val RowHeight = 56.dp
private val LanguageHeight = 48.dp
private val LanguageMinWidth = 56.dp
private val IconSize = 22.dp
private val MenuGap = 4.dp
private val MenuPadding = 8.dp
private val MenuBelowButton = 54.dp

/**
 * The menu under the user button (`UserMenu.vue`, UX audit L1 and R5): the user's own language ("FR · 中文 · EN"),
 * "Changer mon PIN", "Fin de service" (a second tap within 5 s confirms) and "Verrouiller". Every row is 56 dp; none
 * is a Volt action. A tap outside closes it. Test tags: `user_menu`, `user_menu_lang_<code>`, `user_menu_pin`,
 * `user_menu_end_shift`, `user_menu_lock`.
 */
@Composable
internal fun UserMenu(
    state: HeaderUiState,
    actions: HeaderActions,
) {
    val colors = PiliTheme.colors
    val density = LocalDensity.current
    val below = with(density) { MenuBelowButton.roundToPx() }
    Popup(
        alignment = Alignment.TopStart,
        offset = IntOffset(0, below),
        onDismissRequest = actions.onCloseUserMenu,
        properties = PopupProperties(focusable = true),
    ) {
        Column(
            modifier =
                Modifier
                    .width(MenuWidth)
                    .background(colors.bgSecondary, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .border(BorderStroke(1.dp, colors.border), RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .padding(MenuPadding)
                    .testTag("user_menu"),
            verticalArrangement = Arrangement.spacedBy(MenuGap),
        ) {
            LanguageRow(state, actions)
            MenuItem(Icons.Filled.Password, vueText("pin_change"), "user_menu_pin", onClick = actions.onOpenPin)
            MenuItem(
                icon = Icons.AutoMirrored.Filled.Logout,
                text = vueText(if (state.endShiftArmed) "end_shift_confirm" else "end_shift"),
                tag = "user_menu_end_shift",
                armed = state.endShiftArmed,
                onClick = actions.onEndShift,
            )
            MenuItem(Icons.Filled.Lock, vueText("lock"), "user_menu_lock", onClick = actions.onLock)
        }
    }
}

@Composable
private fun LanguageRow(
    state: HeaderUiState,
    actions: HeaderActions,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .defaultMinSize(minHeight = RowHeight)
                .background(colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .padding(horizontal = PiliTheme.dimens.spaceMd - PiliTheme.dimens.spaceXs)
                .testTag("user_menu_language"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
            Icon(Icons.Filled.Translate, contentDescription = null, tint = colors.textPrimary, modifier = Modifier.size(IconSize))
            Text(
                text = vueText("user_menu_language"),
                style = typography.body.copy(fontSize = typography.sizePrice, fontWeight = typography.weightBold),
                color = colors.textPrimary,
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(MenuGap)) {
            LanguageChoices.forEach { language ->
                val chosen = language == state.language
                Box(
                    modifier =
                        Modifier
                            .height(LanguageHeight)
                            .defaultMinSize(minWidth = LanguageMinWidth)
                            .background(if (chosen) colors.selectedBg else colors.bgPrimary, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                            .clickable(role = Role.RadioButton) { actions.onPickLanguage(language) }
                            .semantics { selected = chosen }
                            .testTag("user_menu_lang_${language.code}"),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = vueText("lang_short_${language.code}"),
                        style = typography.body.copy(fontWeight = typography.weightBold),
                        color = if (chosen) colors.selectedFg else colors.textPrimary,
                        modifier = Modifier.padding(horizontal = PiliTheme.dimens.spaceSm),
                    )
                }
            }
        }
    }
}

/** One 56 dp row of the menu; [armed] draws it in red, the colour of what ends something. */
@Composable
private fun MenuItem(
    icon: ImageVector,
    text: String,
    tag: String,
    armed: Boolean = false,
    onClick: () -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusSm)
    val tone = if (armed) colors.accentDanger else colors.textPrimary
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(RowHeight)
                .tapScale(source)
                .background(if (armed) Color.Transparent else colors.bgSurface, shape)
                .then(if (armed) Modifier.border(BorderStroke(2.dp, colors.accentDanger), shape) else Modifier)
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .padding(horizontal = PiliTheme.dimens.spaceMd - PiliTheme.dimens.spaceXs)
                .testTag(tag),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
    ) {
        Icon(icon, contentDescription = null, tint = tone, modifier = Modifier.size(IconSize))
        Text(
            text = text,
            style = typography.body.copy(fontSize = typography.sizePrice, fontWeight = typography.weightBold),
            color = tone,
        )
    }
}
