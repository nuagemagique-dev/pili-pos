@file:OptIn(ExperimentalLayoutApi::class)

package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val ToggleHeight = 44.dp
private val LegendBadge = 20.dp

/**
 * The allergen strip above the products (`allergen-bar` of `OrderView.vue`): the "Allergènes" toggle and, while the
 * mode is on, the hint and the legend of the 14 numbers. Test tags: `allergen_mode`, `allergen_legend`,
 * `allergen_legend_<n>`.
 */
@Composable
internal fun AllergenBar(
    menu: MenuState,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    FlowRow(
        modifier = modifier.fillMaxWidth().padding(start = d.spaceMd, end = d.spaceMd, top = d.spaceSm),
        horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
        itemVerticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = vueText("allergens_mode"),
            style = t.body.copy(fontWeight = t.weightBold),
            color = if (menu.allergenMode) c.selectedFg else c.textPrimary,
            modifier =
                Modifier
                    .heightIn(min = ToggleHeight)
                    .tapScale(source)
                    .clip(RoundedCornerShape(d.radiusMd))
                    .background(if (menu.allergenMode) c.selectedBg else c.bgSurface)
                    .clickable(interactionSource = source, indication = null, role = Role.Switch, onClick = onToggle)
                    .semantics { selected = menu.allergenMode }
                    .padding(horizontal = 18.dp, vertical = 10.dp)
                    .testTag("allergen_mode"),
        )
        if (menu.allergenMode) {
            Text(
                text = vueText("allergens_hint"),
                style = t.body.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
                color = c.accentInfo,
            )
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
}
