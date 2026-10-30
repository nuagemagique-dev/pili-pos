package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.format.bilingualName
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.MenuItem

/** Buttons per row (`repeat(5, ...)` in `MenuGrid.vue`). */
private const val GRID_COLUMNS = 5

private val ButtonMinHeight = 120.dp
private val ColorBand = 4.dp
private const val NAME_LINES = 2

/**
 * The product buttons (`MenuGrid.vue`): five columns, vertical scroll. [categoryColor] is the colour of the left
 * band of every button; [cartQuantities] gives the badge on the products already in the cart. When the category has
 * no product (and the products were read) the empty text is shown. Test tags: `menu_grid`, `menu_empty`.
 */
@Composable
@Suppress("LongParameterList")
internal fun MenuGrid(
    items: List<MenuItem>,
    categoryColor: Color?,
    cartQuantities: Map<Long, Int>,
    language: AppLanguage,
    labelMode: ProductLabelMode,
    loaded: Boolean,
    onAdd: (MenuItem) -> Unit,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    Box(modifier.testTag("menu_grid")) {
        LazyVerticalGrid(
            columns = GridCells.Fixed(GRID_COLUMNS),
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(d.spaceMd),
            horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
            verticalArrangement = Arrangement.spacedBy(d.spaceSm),
        ) {
            items(items, key = { it.id }) { item ->
                MenuItemButton(item, categoryColor, cartQuantities[item.id] ?: 0, language, labelMode, { onAdd(item) })
            }
        }
        if (loaded && items.isEmpty()) {
            Text(
                text = vueText("no_items"),
                style = PiliTheme.typography.title,
                color = PiliTheme.colors.textSecondary,
                modifier = Modifier.align(Alignment.Center).testTag("menu_empty"),
            )
        }
    }
}

/**
 * One product button (`MenuItemButton.vue`): the name in the UI language with the other one underneath (or the menu
 * code when the till shows codes), the price or the open-price badge, the category colour as a band on the left and
 * the quantity already in the cart bottom right. A sold-out product is greyed with an "Épuisé" badge top right. Test
 * tags: `menu_item_<id>`, `menu_item_<id>_qty`, `menu_item_<id>_soldout`.
 */
@Composable
@Suppress("LongParameterList")
private fun MenuItemButton(
    item: MenuItem,
    categoryColor: Color?,
    inCart: Int,
    language: AppLanguage,
    labelMode: ProductLabelMode,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    val source = remember { MutableInteractionSource() }
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = ButtonMinHeight)
                .tapScale(source)
                .clip(RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .alpha(if (item.soldOut) SOLD_OUT_ALPHA else 1f)
                .background(if (item.soldOut) c.bgSurface else c.bgSecondary)
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .testTag("menu_item_${item.id}"),
    ) {
        Box(Modifier.width(ColorBand).fillMaxHeight().background(categoryColor ?: c.border))
        ItemLabel(item, language, labelMode)
        if (item.soldOut) {
            Text(
                text = vueText("soldout_badge"),
                style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeSm, fontWeight = FontWeight.Bold),
                color = c.textOnAccent,
                modifier =
                    Modifier
                        .align(Alignment.TopEnd)
                        .padding(PiliTheme.dimens.spaceSm)
                        .background(c.accentWarning, RoundedCornerShape(6.dp))
                        .padding(horizontal = PiliTheme.dimens.spaceSm, vertical = 2.dp)
                        .testTag("menu_item_${item.id}_soldout"),
            )
        }
        if (item.allergens.isNotEmpty()) {
            AllergenBadges(
                codes = item.allergens,
                modifier =
                    Modifier
                        .align(Alignment.BottomStart)
                        .padding(start = ColorBand + PiliTheme.dimens.spaceSm, bottom = PiliTheme.dimens.spaceSm, end = AllergenEndGap),
            )
        }
        if (inCart > 0) {
            Text(
                text = inCart.toString(),
                style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeSm, fontWeight = FontWeight.Bold),
                color = c.selectedFg,
                textAlign = TextAlign.Center,
                modifier =
                    Modifier
                        .align(Alignment.BottomEnd)
                        .padding(PiliTheme.dimens.spaceSm)
                        .heightIn(min = QtyBadge)
                        .background(c.selectedBg, RoundedCornerShape(QtyBadge))
                        .padding(horizontal = PiliTheme.dimens.spaceSm, vertical = PiliTheme.dimens.spaceXs)
                        .testTag("menu_item_${item.id}_qty"),
            )
        }
    }
}

/** The centred texts of a product button: name, other language, price or open-price badge. */
@Composable
private fun ItemLabel(
    item: MenuItem,
    language: AppLanguage,
    labelMode: ProductLabelMode,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val names = bilingualName(item, language, labelMode)
    Column(
        modifier = Modifier.fillMaxWidth().heightIn(min = ButtonMinHeight).padding(horizontal = d.spaceMd, vertical = d.spaceSm),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(d.spaceXs, Alignment.CenterVertically),
    ) {
        Text(
            text = names.primary,
            style =
                if (names.isCode) {
                    t.figure.copy(fontSize = t.sizeLg, fontWeight = t.weightBold)
                } else {
                    t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold)
                },
            color = c.textPrimary,
            maxLines = NAME_LINES,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
        )
        names.secondary?.let {
            Text(
                text = it,
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
                maxLines = NAME_LINES,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
        }
        if (item.openPrice) {
            Text(
                text = vueText("open_price_badge"),
                style = t.body.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
                color = c.textPrimary,
                modifier = Modifier.border(1.dp, c.border, RoundedCornerShape(6.dp)).padding(horizontal = d.spaceSm),
            )
        } else {
            Text(
                text = if (item.soldByWeight) formatCents(item.priceCents) + " €/kg" else formatCents(item.priceCents),
                style = t.figure.copy(fontSize = t.sizeMd, fontWeight = t.weightBold),
                color = c.textSecondary,
            )
        }
    }
}

private val QtyBadge = 15.dp

/** A sold-out product button is greyed (`.item-btn.sold-out` of `MenuItemButton.vue`). */
private const val SOLD_OUT_ALPHA = 0.55f

/** Room kept free on the right of the allergen badges for the quantity badge. */
private val AllergenEndGap = 56.dp
