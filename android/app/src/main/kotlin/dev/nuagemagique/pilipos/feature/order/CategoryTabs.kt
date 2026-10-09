package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.format.bilingualCategoryName
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.Category

/** Tabs per row (`repeat(9, ...)` in `CategoryTabs.vue`); more categories wrap to a second row. */
private const val TAB_COLUMNS = 9

private val TabHeight = 60.dp
private val StripeHeight = 5.dp

/** Length of `#RRGGBB`. */
private const val HEX_COLOR_LENGTH = 7
private const val HEX_RADIX = 16
private const val HEX_PAIR = 2
private const val CHANNELS = 3

/**
 * The colour of a category: `#RRGGBB` as the catalog stores it, or null when it is missing or not that form (the
 * neutral border colour is used then). Pure, so a wrong catalog value never reaches the drawing.
 */
internal fun parseCategoryColor(hex: String?): Color? {
    val text = hex?.trim().orEmpty()
    val channels =
        if (text.length == HEX_COLOR_LENGTH && text.startsWith("#")) {
            (1 until HEX_COLOR_LENGTH step HEX_PAIR).mapNotNull { text.substring(it, it + HEX_PAIR).toIntOrNull(HEX_RADIX) }
        } else {
            emptyList()
        }
    return if (channels.size == CHANNELS) Color(red = channels[0], green = channels[1], blue = channels[2]) else null
}

/**
 * The category tabs (`CategoryTabs.vue`): two fixed rows of nine instead of scrolling tabs, so every category is one
 * tap away. The selected tab is filled with the selection colours; a stripe on top carries the category colour.
 * Test tags: `order_categories`, `category_<id>`.
 */
@Composable
internal fun CategoryTabs(
    categories: List<Category>,
    selectedId: Long?,
    language: AppLanguage,
    onSelect: (Long) -> Unit,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(start = d.spaceMd, end = d.spaceMd, top = d.spaceMd, bottom = d.spaceSm)
                .testTag("order_categories"),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        categories.chunked(TAB_COLUMNS).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
                row.forEach { category ->
                    CategoryTab(category, category.id == selectedId, language, { onSelect(category.id) }, Modifier.weight(1f))
                }
                repeat(TAB_COLUMNS - row.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

@Composable
private fun CategoryTab(
    category: Category,
    selected: Boolean,
    language: AppLanguage,
    onClick: () -> Unit,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val names = bilingualCategoryName(category.name, category.nameFr, category.nameZh, language)
    val foreground = if (selected) c.selectedFg else c.textPrimary
    Box(
        modifier =
            modifier
                .height(TabHeight)
                .tapScale(source)
                .clip(RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .background(if (selected) c.selectedBg else c.bgSecondary)
                .clickable(interactionSource = source, indication = null, role = Role.Tab, onClick = onClick)
                .semantics { this.selected = selected }
                .testTag("category_${category.id}"),
    ) {
        Box(Modifier.fillMaxWidth().height(StripeHeight).background(parseCategoryColor(category.color) ?: c.border))
        Column(
            modifier = Modifier.fillMaxWidth().align(Alignment.Center).padding(top = StripeHeight, start = 6.dp, end = 6.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = names.primary,
                style = t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold),
                color = foreground,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
            names.secondary?.let {
                Text(
                    text = it,
                    style = t.body.copy(fontSize = t.sizeXs),
                    color = if (selected) c.selectedFg else c.textSecondary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    textAlign = TextAlign.Center,
                )
            }
        }
    }
}
