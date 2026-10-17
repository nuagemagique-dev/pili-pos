package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.compositeOver
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.ModifierGroup
import uniffi.pos_ffi.Modifier as ProductOption

/** Options per row of a group (`repeat(auto-fill, minmax(180px, 1fr))` at the width of the dialog). */
private const val OPTIONS_PER_ROW = 3

/** Share of the accent colour mixed into the background of a picked option. */
private const val PICKED_ALPHA = 0.18f

private val DialogMaxWidth = 720.dp
private val GroupsMaxHeight = 560.dp
private val OptionMinHeight = 64.dp

/**
 * The options picked in a [ModifierDialog] (`picked` of `ModifierDialog.vue`). A group whose maximum is 1 is a
 * single choice: a new pick replaces the previous one. In the other groups an option toggles, and a pick above the
 * group maximum is refused. Pure, so the rules are tested without a screen. No amount is involved (PD208, PD219).
 */
internal data class ModifierPicks(
    val ids: Set<Long> = emptySet(),
) {
    /** How many options of [group] are picked. */
    fun countIn(group: ModifierGroup): Int = group.options.count { it.id in ids }

    /** The picks after a tap on [optionId] of [group]. */
    fun toggle(
        group: ModifierGroup,
        optionId: Long,
    ): ModifierPicks {
        val max = group.max
        return when {
            optionId in ids -> ModifierPicks(ids - optionId)
            max == 1L -> ModifierPicks(ids - group.options.map { it.id }.toSet() + optionId)
            max != null && countIn(group) >= max -> this
            else -> ModifierPicks(ids + optionId)
        }
    }

    /** True when every group has between its minimum and its maximum picks. */
    fun isValid(groups: List<ModifierGroup>): Boolean =
        groups.all { group ->
            val picked = countIn(group).toLong()
            val max = group.max
            picked >= group.min && (max == null || picked <= max)
        }
}

/** The rule line of a group (`rule()` of `ModifierDialog.vue`). */
@Composable
private fun ruleText(group: ModifierGroup): String {
    val min = group.min
    val max = group.max
    if (max == null) return if (min > 0) vueText("modifier_required", "min" to min) else vueText("modifier_optional")
    return when {
        min > 0 && max == min -> vueText("modifier_required", "min" to min)
        min > 0 -> vueText("modifier_range", "min" to min, "max" to max)
        else -> vueText("modifier_up_to", "max" to max)
    }
}

/**
 * The option picker of a product (`ModifierDialog.vue`): the groups of the mirrored catalog, a group with a maximum
 * of one being a single choice. The product price and the price of each option are shown as they are; no total is
 * computed (PD208, PD219), the engine prices the line. Confirm is enabled when every group is within its limits and
 * calls [onConfirm] with the picked option ids.
 *
 * Test tags: `modifier_dialog` (root), `modifier_group_<id>`, `modifier_option_<id>`, `modifier_confirm`,
 * `modifier_cancel`.
 */
@Composable
internal fun ModifierDialog(
    item: MenuItem,
    groups: List<ModifierGroup>,
    onConfirm: (List<Long>) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    var picks by remember(item.id) { mutableStateOf(ModifierPicks()) }
    val valid = picks.isValid(groups)
    Box(modifier.fillMaxSize().testTag("modifier_dialog")) {
        PiliDialog(
            title = item.name,
            onDismissRequest = onCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("modifier_cancel"),
                )
                PiliButton(
                    text = vueText("modifier_add"),
                    onClick = { if (valid) onConfirm(picks.ids.toList()) },
                    enabled = valid,
                    modifier = Modifier.testTag("modifier_confirm"),
                )
            },
        ) {
            Text(formatEuro(item.priceCents), style = PiliTheme.typography.figure, color = c.textSecondary)
            Column(
                modifier = Modifier.fillMaxWidth().heightIn(max = GroupsMaxHeight).verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(d.spaceMd),
            ) {
                groups.forEach { group ->
                    GroupSection(group, picks) { optionId -> picks = picks.toggle(group, optionId) }
                }
            }
        }
    }
}

@Composable
private fun GroupSection(
    group: ModifierGroup,
    picks: ModifierPicks,
    onToggle: (Long) -> Unit,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Column(Modifier.fillMaxWidth().testTag("modifier_group_${group.id}"), verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.Bottom) {
            Text(group.name, style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
            Text(
                ruleText(group),
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
                modifier = Modifier.testTag("modifier_group_${group.id}_rule"),
            )
        }
        group.options.chunked(OPTIONS_PER_ROW).forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
                row.forEach { option -> OptionButton(option, option.id in picks.ids, { onToggle(option.id) }, Modifier.weight(1f)) }
                repeat(OPTIONS_PER_ROW - row.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

@Composable
private fun OptionButton(
    option: ProductOption,
    picked: Boolean,
    onClick: () -> Unit,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(d.radiusMd)
    val fill = if (picked) c.accentPrimary.copy(alpha = PICKED_ALPHA).compositeOver(c.bgSurface) else c.bgSurface
    val price = formatEuro(option.priceCents)
    Column(
        modifier =
            modifier
                .heightIn(min = OptionMinHeight)
                .tapScale(source)
                .clip(shape)
                .background(fill)
                .border(2.dp, if (picked) c.accentPrimary else c.border, shape)
                .clickable(interactionSource = source, indication = null, role = Role.Checkbox, onClick = onClick)
                .semantics { selected = picked }
                .padding(horizontal = 14.dp, vertical = d.spaceSm)
                .testTag("modifier_option_${option.id}"),
        verticalArrangement = Arrangement.spacedBy(2.dp, Alignment.CenterVertically),
    ) {
        Text(option.name, style = t.body, color = c.textPrimary)
        if (option.priceCents > 0L) {
            Text("+$price", style = t.figure.copy(fontSize = t.sizeSm), color = c.textSecondary)
        }
        if (option.allergens.isNotEmpty()) AllergenBadges(option.allergens)
    }
}
