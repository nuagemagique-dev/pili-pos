package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
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
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.SoldOutItem

private val DialogWidth = 480.dp
private val ListWidth = 620.dp
private val ListMaxHeight = 560.dp
private val ChoiceMinHeight = 64.dp
private val BarButtonMinHeight = 56.dp

/** The end of a business day ("2026-10-18T04:00:00+02:00") as "18/10 04:00" in Brussels time (`endOfDayLabel`). */
internal fun endOfDayLabel(until: String): String = PosDateTime.formatTimestamp(until, PosDateTime::formatDayTime)

/** The status line of a sold-out product: until the end of the day (with the time) or until re-enabled. */
@Composable
private fun statusText(until: String?): String =
    if (until != null) vueText("soldout_status_day", "time" to endOfDayLabel(until)) else vueText("soldout_status_reenabled")

/**
 * The bar above the product grid (cashier and manager): the "Épuisé" mode button with its hint while the mode is on,
 * and "Produits épuisés (n)" on the right. Test tags: `soldout_mode`, `soldout_hint`, `soldout_list_button`.
 */
@Composable
internal fun SoldOutBar(
    mode: Boolean,
    count: Int,
    onMode: () -> Unit,
    onList: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(
        modifier.fillMaxWidth().padding(horizontal = d.spaceMd, vertical = d.spaceSm),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        BarButton(
            text = vueText("soldout_mode"),
            active = mode,
            onClick = onMode,
            modifier = Modifier.testTag("soldout_mode"),
        )
        if (mode) {
            Text(
                text = vueText("soldout_mode_hint"),
                style = t.body.copy(fontSize = t.sizeMd),
                color = c.textSecondary,
                modifier = Modifier.testTag("soldout_hint"),
            )
        }
        Box(Modifier.weight(1f), contentAlignment = Alignment.CenterEnd) {
            BarButton(
                text = vueText("soldout_list_button", "n" to count),
                active = false,
                onClick = onList,
                modifier = Modifier.testTag("soldout_list_button"),
            )
        }
    }
}

@Composable
private fun BarButton(
    text: String,
    active: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    Box(
        modifier =
            modifier
                .defaultMinSize(minHeight = BarButtonMinHeight)
                .tapScale(source)
                .clip(RoundedCornerShape(d.radiusMd))
                .background(if (active) c.accentWarning else c.bgSecondary)
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .semantics { selected = active }
                .padding(horizontal = d.spaceMd),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold),
            color = if (active) c.textOnAccent else c.textPrimary,
        )
    }
}

/**
 * The availability of one product (`SoldOutDialog.vue`): sold out until the end of the business day, sold out until
 * re-enabled, or available; the current state is outlined. A choice equal to the current one just closes the dialog.
 * Test tags: `soldout_dialog` (root), `soldout_status`, `soldout_choice_day`, `soldout_choice_until`,
 * `soldout_choice_available`, `soldout_cancel`.
 */
@Composable
internal fun SoldOutDialog(
    item: MenuItem,
    onChoose: (menuItemId: Long, choice: SoldOutChoice) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val current = item.availability()
    Box(modifier.fillMaxSize().testTag("soldout_dialog")) {
        PiliDialog(
            title = item.name,
            onDismissRequest = onCancel,
            maxWidth = DialogWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("soldout_cancel"),
                )
            },
        ) {
            if (item.soldOut) {
                Text(
                    text = statusText(item.soldOutUntil),
                    style = PiliTheme.typography.body,
                    color = c.textSecondary,
                    modifier = Modifier.testTag("soldout_status"),
                )
            }
            Choice(SoldOutChoice.DAY, "soldout_until_day", "soldout_choice_day", current) { pick(item, it, current, onChoose, onCancel) }
            Choice(SoldOutChoice.UNTIL, "soldout_until_reenabled", "soldout_choice_until", current) {
                pick(item, it, current, onChoose, onCancel)
            }
            Choice(SoldOutChoice.AVAILABLE, "soldout_available", "soldout_choice_available", current) {
                pick(item, it, current, onChoose, onCancel)
            }
        }
    }
}

private fun pick(
    item: MenuItem,
    choice: SoldOutChoice,
    current: SoldOutChoice,
    onChoose: (Long, SoldOutChoice) -> Unit,
    onCancel: () -> Unit,
) {
    if (choice == current) onCancel() else onChoose(item.id, choice)
}

@Composable
private fun Choice(
    choice: SoldOutChoice,
    labelKey: String,
    tag: String,
    current: SoldOutChoice,
    onClick: (SoldOutChoice) -> Unit,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(d.radiusMd)
    val isCurrent = choice == current
    val outline = if (choice == SoldOutChoice.AVAILABLE) c.accentPrimary else c.accentWarning
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = ChoiceMinHeight)
                .tapScale(source)
                .clip(shape)
                .background(c.bgSurface)
                .border(2.dp, if (isCurrent) outline else c.bgSurface, shape)
                .clickable(interactionSource = source, indication = null, role = Role.Button) { onClick(choice) }
                .semantics { selected = isCurrent }
                .padding(horizontal = d.spaceMd)
                .testTag(tag),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = vueText(labelKey),
            style = t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold),
            color = c.textPrimary,
        )
    }
}

/**
 * "Produits épuisés" (`SoldOutList.vue`): every product sold out now with its status; "Remettre en vente" puts one
 * back on sale and the row goes. Test tags: `soldout_list` (root), `soldout_list_empty`, `soldout_row_<id>`,
 * `soldout_reenable_<id>`, `soldout_list_close`.
 */
@Composable
internal fun SoldOutList(
    items: List<SoldOutItem>,
    onReenable: (menuItemId: Long, choice: SoldOutChoice) -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Box(modifier.fillMaxSize().testTag("soldout_list")) {
        PiliDialog(
            title = vueText("soldout_list_title"),
            onDismissRequest = onClose,
            maxWidth = ListWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = onClose,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("soldout_list_close"),
                )
            },
        ) {
            if (items.isEmpty()) {
                Text(
                    text = vueText("soldout_list_empty"),
                    style = t.body.copy(fontSize = t.sizeMd),
                    color = c.textSecondary,
                    modifier = Modifier.padding(d.spaceLg).testTag("soldout_list_empty"),
                )
            } else {
                Column(
                    modifier = Modifier.fillMaxWidth().heightIn(max = ListMaxHeight).verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(d.spaceSm),
                ) {
                    items.forEach { SoldOutRow(it, onReenable) }
                }
            }
        }
    }
}

@Composable
private fun SoldOutRow(
    item: SoldOutItem,
    onReenable: (Long, SoldOutChoice) -> Unit,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(c.bgSurface, RoundedCornerShape(d.radiusMd))
                .padding(horizontal = d.spaceMd, vertical = d.spaceSm)
                .testTag("soldout_row_${item.menuItemId}"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(item.name, style = t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold), color = c.textPrimary)
            Text(statusText(item.until), style = t.body.copy(fontSize = t.sizeSm), color = c.textSecondary)
        }
        PiliButton(
            text = vueText("soldout_reenable"),
            onClick = { onReenable(item.menuItemId, SoldOutChoice.AVAILABLE) },
            modifier = Modifier.width(ReenableWidth).testTag("soldout_reenable_${item.menuItemId}"),
        )
    }
}

private val ReenableWidth = 200.dp
