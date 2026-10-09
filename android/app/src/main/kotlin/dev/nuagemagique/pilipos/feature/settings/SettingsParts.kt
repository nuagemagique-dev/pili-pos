@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Side of the avatar circle of an employee. */
private val AvatarSize = 48.dp

/** Side of the dot of a status pill. */
private val DotSize = 8.dp

/** Height of a status pill (36 in `SettingsView.vue`). */
private val PillHeight = 36.dp

/** Height of a text field (56 in `SettingsView.vue`). */
private val FieldHeight = 56.dp

/** Height of a segmented button (56 for the language and the label, 48 for the roles in `SettingsView.vue`). */
internal val SegmentHeight = 56.dp

/** Height of the roles buttons. */
internal val RoleSegmentHeight = 48.dp

/** Height of a row of the information panel (48 in `SettingsView.vue`). */
private val InfoRowHeight = 48.dp

/** Width of the toggle track and its thumb travel (64, 40 and 32 in `SettingsView.vue`). */
private val TrackWidth = 64.dp
private val TrackHeight = 40.dp
private val ThumbSize = 32.dp
private val ThumbTravel = 24.dp

/** A grouped panel of the settings screen: Graphite fill, hairline border, large radius; [scroll] scrolls its content inside. */
@Composable
internal fun Panel(
    modifier: Modifier = Modifier,
    borderColor: Color = PiliTheme.colors.border,
    scroll: Boolean = false,
    content: @Composable ColumnScope.() -> Unit,
) {
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusLg)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(PiliTheme.colors.bgSecondary, shape)
                .border(1.dp, borderColor, shape)
                .then(if (scroll) Modifier.verticalScroll(rememberScrollState()) else Modifier)
                .padding(PiliTheme.dimens.spaceLg),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        content = content,
    )
}

/** The title of a panel (`.panel h2`). */
@Composable
internal fun PanelTitle(
    text: String,
    modifier: Modifier = Modifier,
) {
    Text(text, style = PiliTheme.typography.title, color = PiliTheme.colors.textPrimary, modifier = modifier)
}

/** A secondary paragraph of a panel (`.panel-text`, `.panel-note`). */
@Composable
internal fun PanelText(
    text: String,
    modifier: Modifier = Modifier,
    color: Color = PiliTheme.colors.textSecondary,
) {
    Text(text, style = PiliTheme.typography.body, color = color, modifier = modifier)
}

/** The small upper-case caption of a group (`.section-label`). */
@Composable
internal fun SectionLabel(
    text: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = text.uppercase(),
        style = PiliTheme.typography.label.copy(fontFamily = PiliTheme.typography.familyFigures, letterSpacing = 0.08.em),
        color = PiliTheme.colors.textSecondary,
        modifier = modifier,
    )
}

/** A status pill: a dot and a short text in [color] on a hairline outline (`.pill`). */
@Composable
internal fun StatusPill(
    text: String,
    color: Color,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier =
            modifier
                .height(PillHeight)
                .border(1.dp, PiliTheme.colors.border, RoundedCornerShape(PillHeight / 2))
                .semantics(mergeDescendants = true) {}
                .padding(horizontal = 14.dp),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(DotSize).background(color, CircleShape))
        Text(
            text,
            style = PiliTheme.typography.label.copy(fontFamily = PiliTheme.typography.familyFigures),
            color = color,
        )
    }
}

/** A row of the information panel: the label on the left, the monospace value on the right (`.info-row`). */
@Composable
internal fun InfoRow(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth().heightIn(min = InfoRowHeight),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
        Text(
            value,
            style = PiliTheme.typography.figure,
            color = PiliTheme.colors.textPrimary,
            textAlign = TextAlign.End,
            modifier = Modifier.weight(1f),
        )
    }
    Hairline()
}

/** A one pixel separator in the border colour. */
@Composable
internal fun Hairline() {
    Box(Modifier.fillMaxWidth().height(1.dp).background(PiliTheme.colors.border))
}

/**
 * A segmented control (`.segmented`): equal buttons on a Ardoise strip, the chosen one in Brume. Each button has
 * the tag `<tag>_<key>` and the selected semantic.
 */
@Composable
internal fun <T> SegmentedControl(
    options: List<Triple<T, String, String>>,
    selected: T,
    onSelect: (T) -> Unit,
    tag: String,
    modifier: Modifier = Modifier,
    height: Dp = SegmentHeight,
) {
    val colors = PiliTheme.colors
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusSm)
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .background(colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .padding(PiliTheme.dimens.spaceXs),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
    ) {
        options.forEach { (value, key, label) ->
            val active = value == selected
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .height(height)
                        .background(if (active) colors.selectedBg else Color.Transparent, shape)
                        .clickable(role = Role.RadioButton) { onSelect(value) }
                        .semantics { this.selected = active }
                        .testTag(tag + "_" + key),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    label,
                    style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
                    color = if (active) colors.selectedFg else colors.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

/** The on/off switch of the training panel (`.toggle`): the state text and a track with a thumb. */
@Composable
internal fun TrainingToggle(
    on: Boolean,
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Row(
        modifier =
            modifier
                .height(SegmentHeight)
                .background(colors.bgSurface, RoundedCornerShape(SegmentHeight / 2))
                .clickable(role = Role.Switch, onClick = onClick)
                .padding(start = PiliTheme.dimens.spaceMd, end = PiliTheme.dimens.spaceSm),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = if (on) colors.textPrimary else colors.textSecondary,
        )
        Box(
            Modifier
                .size(TrackWidth, TrackHeight)
                .background(if (on) colors.accentTraining else colors.border, RoundedCornerShape(TrackHeight / 2)),
        ) {
            Box(
                Modifier
                    .padding(start = PiliTheme.dimens.spaceXs + if (on) ThumbTravel else 0.dp, top = PiliTheme.dimens.spaceXs)
                    .size(ThumbSize)
                    .background(if (on) colors.textPrimary else colors.textSecondary, CircleShape),
            )
        }
    }
}

/** A single line text field on a hairline outline (`.field`); [mono] draws the value in the figure font. */
@Composable
internal fun SettingsTextField(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    mono: Boolean = false,
    numeric: Boolean = false,
    onDone: (() -> Unit)? = null,
) {
    val colors = PiliTheme.colors
    val style =
        PiliTheme.typography.body.copy(
            color = colors.textPrimary,
            fontFamily = if (mono) PiliTheme.typography.familyFigures else FontFamily.Default,
        )
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        singleLine = true,
        textStyle = style,
        cursorBrush = SolidColor(colors.textPrimary),
        keyboardOptions =
            KeyboardOptions(
                keyboardType = if (numeric) KeyboardType.Number else KeyboardType.Text,
                imeAction = if (onDone != null) ImeAction.Done else ImeAction.Default,
            ),
        keyboardActions = KeyboardActions(onDone = { onDone?.invoke() }),
        modifier = modifier,
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(FieldHeight)
                    .border(1.dp, colors.border, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .padding(horizontal = PiliTheme.dimens.spaceMd),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.isEmpty()) {
                    Text(placeholder, style = PiliTheme.typography.body, color = colors.textSecondary, maxLines = 1)
                }
                inner()
            }
        },
    )
}

/** A secondary action button on the Ardoise fill, the common look of the small buttons of the screen. */
@Composable
internal fun ActionButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    variant: PiliButtonVariant = PiliButtonVariant.Secondary,
) {
    PiliButton(text = text, onClick = onClick, modifier = modifier, enabled = enabled, variant = variant)
}

/** The avatar of an employee: the [initial] in a circle (`.avatar`). */
@Composable
internal fun Avatar(
    initial: String,
    modifier: Modifier = Modifier,
) {
    Box(modifier.size(AvatarSize).background(PiliTheme.colors.bgSurface, CircleShape), contentAlignment = Alignment.Center) {
        Text(initial, style = PiliTheme.typography.title.copy(fontSize = PiliTheme.typography.sizeLg), color = PiliTheme.colors.textPrimary)
    }
}

/** A row of items placed left to right with the standard gap. */
@Composable
internal fun GapRow(
    modifier: Modifier = Modifier,
    content: @Composable RowScope.() -> Unit,
) {
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
        content = content,
    )
}
