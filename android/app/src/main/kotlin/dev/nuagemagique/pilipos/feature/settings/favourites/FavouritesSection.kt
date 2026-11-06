package dev.nuagemagique.pilipos.feature.settings.favourites

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.feature.order.FAVOURITES_MAX
import dev.nuagemagique.pilipos.feature.settings.Panel
import dev.nuagemagique.pilipos.feature.settings.PanelText
import dev.nuagemagique.pilipos.feature.settings.PanelTitle
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.FavouritesMode

private val ControlSize = 48.dp
private val ModeHeight = 56.dp
private val RowHeight = 56.dp

/**
 * The favourites panel of the settings screen (`FavouritesSettings.vue`, "Favoris"): the mode of the first tab of the
 * order screen (the 30-day best sellers or the manager's own selection) and, in the second mode, the selected products
 * with a cross each. A tap saves at once. Local data: nothing here reaches the signed data. Test tags:
 * `favourites_section`, `favourites_error`, `favourites_mode_top_sold`, `favourites_mode_selection`,
 * `favourites_top_note`, `favourites_empty`, `favourite_<productId>` (a row) with `favourite_<productId>_remove`,
 * `favourites_count`.
 */
@Composable
internal fun FavouritesSection(
    state: FavouritesUiState,
    callbacks: FavouritesCallbacks,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Panel(modifier = modifier.testTag("favourites_section")) {
        PanelTitle(vueText("fav_title"), Modifier)
        state.error?.let {
            Text(it, style = PiliTheme.typography.body, color = colors.accentDanger, modifier = Modifier.testTag("favourites_error"))
        }
        PanelText(vueText("fav_note"))
        val mode = state.mode ?: return@Panel
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            ModeButton(FavouritesMode.TOP_SOLD, "top_sold", mode, state.busy, callbacks, Modifier.weight(1f))
            ModeButton(FavouritesMode.SELECTION, "selection", mode, state.busy, callbacks, Modifier.weight(1f))
        }
        if (mode == FavouritesMode.TOP_SOLD) {
            PanelText(vueText("fav_top_sold_note"), Modifier.testTag("favourites_top_note"))
        } else {
            PanelText(vueText("fav_selection_note"))
            if (state.selection.isEmpty()) PanelText(vueText("fav_empty"), Modifier.testTag("favourites_empty"))
            state.selection.forEach { id -> SelectedRow(id, state.nameOf(id), state.busy, callbacks) }
            PanelText(
                vueText("fav_count", "n" to state.selection.size.toString(), "max" to FAVOURITES_MAX.toString()),
                Modifier.testTag("favourites_count"),
            )
        }
    }
}

@Composable
private fun ModeButton(
    mode: FavouritesMode,
    key: String,
    current: FavouritesMode,
    busy: Boolean,
    callbacks: FavouritesCallbacks,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val active = mode == current
    Box(
        modifier =
            modifier
                .defaultMinSize(minHeight = ModeHeight)
                .background(if (active) c.selectedBg else c.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(enabled = !busy, role = Role.RadioButton) { callbacks.onMode(mode) }
                .semantics { this.selected = active }
                .testTag("favourites_mode_$key"),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            vueText("fav_mode_$key"),
            style = t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold),
            color = if (active) c.selectedFg else c.textPrimary,
            textAlign = TextAlign.Center,
        )
    }
}

/** A selected product with its cross. */
@Composable
private fun SelectedRow(
    id: Long,
    name: String,
    busy: Boolean,
    callbacks: FavouritesCallbacks,
) {
    val c = PiliTheme.colors
    Row(
        Modifier.fillMaxWidth().defaultMinSize(minHeight = RowHeight).testTag("favourite_$id"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            name,
            style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeLg, fontWeight = PiliTheme.typography.weightBold),
            color = c.textPrimary,
            modifier = Modifier.weight(1f).testTag("favourite_${id}_text"),
        )
        val description = vueText("fav_remove")
        Box(
            modifier =
                Modifier
                    .defaultMinSize(minWidth = ControlSize, minHeight = ControlSize)
                    .clickable(enabled = !busy, role = Role.Button) { callbacks.onRemove(id) }
                    .semantics { contentDescription = description }
                    .testTag("favourite_${id}_remove"),
            contentAlignment = Alignment.Center,
        ) {
            Text("✕", style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeMd), color = c.accentDanger)
        }
    }
}
