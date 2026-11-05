package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Block
import androidx.compose.material.icons.filled.CardGiftcard
import androidx.compose.material.icons.filled.Dialpad
import androidx.compose.material.icons.filled.NoFood
import androidx.compose.material.icons.filled.PlaylistRemove
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Height of the whole row (`.row` of `ModeToolbar.vue`): 8 dp of air, then the 48 dp buttons. */
internal val ToolbarHeight = 56.dp
private val ButtonHeight = 48.dp
private val ButtonIcon = 22.dp
private val LegendBadge = 20.dp
private const val HINT_LINES = 2

/** What the buttons of the row do. */
internal data class ModeToolbarActions(
    /** The "Épuisé" button: the mode is turned on or off. */
    val onSoldOutMode: () -> Unit = {},
    /** The "Produits épuisés (n)" button. */
    val onSoldOutList: () -> Unit = {},
    /** The "Saisie code" button: the code pad is opened or closed. */
    val onCodePad: () -> Unit = {},
    /** The "Allergènes" button: allergen mode on or off. */
    val onAllergenMode: () -> Unit = {},
    /** The "Bon cadeau" button: ask for the amount of a voucher to sell. */
    val onSellVoucher: () -> Unit = {},
)

/**
 * The one row of mode buttons between the category tabs and the product grid (`ModeToolbar.vue`, UX audit C1): "Épuisé"
 * (cashier and manager), "Saisie code", "Allergènes", "Bon cadeau" (gift vouchers on) and, on the right, "Produits
 * épuisés (n)", each with an icon and a short label. A mode that takes over a tap on a product shows its hint at the end
 * of the row; the allergen legend opens in a strip under the row while that mode is on.
 * Test tags: `mode_toolbar` (the 56 dp row), `soldout_mode`, `soldout_hint`, `soldout_list_button`, `code_pad_toggle`,
 * `allergen_mode`, `allergen_hint`, `gift_voucher_sell`, `allergen_legend`, `allergen_legend_<n>`, `allergen_legend_error`.
 */
@Suppress("LongMethod") // One call per button of the row, nothing else.
@Composable
internal fun ModeToolbar(
    menu: MenuState,
    actions: ModeToolbarActions,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    Column(modifier.fillMaxWidth().padding(horizontal = d.spaceMd)) {
        Row(
            modifier = Modifier.fillMaxWidth().height(ToolbarHeight).testTag("mode_toolbar").padding(top = d.spaceSm),
            horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (menu.canSoldOut) {
                ModeButton(
                    Icons.Filled.Block,
                    vueText("soldout_mode"),
                    menu.soldOutMode,
                    "soldout_mode",
                    Role.Switch,
                    warn = true,
                    actions.onSoldOutMode,
                )
            }
            ModeButton(
                Icons.Filled.Dialpad,
                vueText("code_pad_toggle"),
                menu.codePad.open,
                "code_pad_toggle",
                Role.Switch,
                onClick = actions.onCodePad,
            )
            ModeButton(
                Icons.Filled.NoFood,
                vueText("allergens_mode"),
                menu.allergenMode,
                "allergen_mode",
                Role.Switch,
                onClick = actions.onAllergenMode,
            )
            if (menu.giftVouchersEnabled) {
                ModeButton(
                    Icons.Filled.CardGiftcard,
                    vueText("gv_sell_button"),
                    false,
                    "gift_voucher_sell",
                    Role.Button,
                    onClick = actions.onSellVoucher,
                )
            }
            ModeHint(menu, Modifier.weight(1f))
            if (menu.canSoldOut) {
                ModeButton(
                    Icons.Filled.PlaylistRemove,
                    vueText("soldout_list_button", "n" to menu.soldOut.size),
                    false,
                    "soldout_list_button",
                    Role.Button,
                    onClick = actions.onSoldOutList,
                )
            }
        }
        if (menu.allergenMode) AllergenLegend(menu, Modifier.padding(top = d.spaceSm))
    }
}

/** The hint of the mode that is on, at the end of the row (two lines at most); empty room when no mode is on. */
@Composable
private fun ModeHint(
    menu: MenuState,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    when {
        menu.soldOutMode ->
            Text(
                text = vueText("soldout_mode_hint"),
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
                maxLines = HINT_LINES,
                overflow = TextOverflow.Ellipsis,
                modifier = modifier.padding(horizontal = PiliTheme.dimens.spaceXs).testTag("soldout_hint"),
            )
        menu.allergenMode ->
            Text(
                text = vueText("allergens_hint"),
                style = t.body.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
                color = c.accentInfo,
                maxLines = HINT_LINES,
                overflow = TextOverflow.Ellipsis,
                modifier = modifier.padding(horizontal = PiliTheme.dimens.spaceXs).testTag("allergen_hint"),
            )
        else -> Spacer(modifier)
    }
}

/** One compact button of the row: a Material icon and a short label; [on] colours it, amber when [warn]. */
@Composable
@Suppress("LongParameterList")
private fun ModeButton(
    icon: ImageVector,
    label: String,
    on: Boolean,
    tag: String,
    role: Role,
    warn: Boolean = false,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val fill: Color =
        if (!on) {
            c.bgSurface
        } else if (warn) {
            c.accentWarning
        } else {
            c.selectedBg
        }
    val ink: Color =
        if (!on) {
            c.textPrimary
        } else if (warn) {
            c.textOnAccent
        } else {
            c.selectedFg
        }
    Row(
        modifier =
            Modifier
                .height(ButtonHeight)
                .tapScale(source)
                .clip(RoundedCornerShape(d.radiusMd))
                .background(fill)
                .clickable(interactionSource = source, indication = null, role = role, onClick = onClick)
                .semantics { selected = on }
                .padding(horizontal = 14.dp)
                .testTag(tag),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, contentDescription = null, tint = ink, modifier = Modifier.size(ButtonIcon))
        Text(text = label, style = t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold), color = ink, maxLines = 1)
    }
}

/** The error of a legend that could not be read, then the 14 numbers with their names, under the row. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun AllergenLegend(
    menu: MenuState,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(modifier.fillMaxWidth()) {
        menu.allergenLegendError?.let { error ->
            Text(
                text = error,
                style = t.body.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
                color = c.accentDanger,
                modifier = Modifier.padding(bottom = 4.dp).testTag("allergen_legend_error"),
            )
        }
        FlowRow(
            modifier = Modifier.fillMaxWidth().testTag("allergen_legend"),
            horizontalArrangement = Arrangement.spacedBy(14.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            menu.allergenLegend.forEach { entry ->
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                    NumberCircle(entry.number, Modifier.size(LegendBadge).testTag("allergen_legend_${entry.number}"))
                    Text(
                        text = vueText("allergen_${entry.code}"),
                        style = t.body.copy(fontSize = t.sizeXs),
                        color = c.textSecondary,
                    )
                }
            }
        }
    }
}
