package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Height of a tab, a choice and a row action: well above the 48 dp floor of a touch target. */
internal val ChoiceHeight = 64.dp

private val SpinnerSize = 28.dp
private val SpinnerStroke = 3.dp

/**
 * A choice among others (a tab, a direction, a reason, an amount): the selected one is Brume on dark
 * ([selected] true), the others are the surface fill. The caption is resolved by the caller.
 */
@Composable
internal fun ChoiceButton(
    text: String,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    val c = PiliTheme.colors
    val source = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusMd)
    Box(
        modifier =
            modifier
                .tapScale(source)
                .defaultMinSize(minHeight = ChoiceHeight)
                .background(if (selected) c.selectedBg else c.bgSurface, shape)
                .clickable(interactionSource = source, indication = null, enabled = enabled, role = Role.Button, onClick = onClick)
                .padding(horizontal = PiliTheme.dimens.spaceMd, vertical = PiliTheme.dimens.spaceSm),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = if (selected) c.selectedFg else c.textPrimary,
            textAlign = TextAlign.Center,
        )
    }
}

/** A card of the screen: Graphite fill, hairline border and the padding of a section. */
@Composable
internal fun Panel(
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    val d = PiliTheme.dimens
    val shape = RoundedCornerShape(d.radiusLg)
    Column(
        modifier = modifier.background(PiliTheme.colors.bgSecondary, shape).border(1.dp, PiliTheme.colors.border, shape).padding(d.spaceLg),
        verticalArrangement = Arrangement.spacedBy(d.spaceMd),
        content = content,
    )
}

/** The caption of a step of the cash form ("1 · Direction"). */
@Composable
internal fun StepLabel(text: String) {
    Text(text, style = PiliTheme.typography.label, color = PiliTheme.colors.textSecondary)
}

/** A small spinner for a report or a signature in progress. */
@Composable
internal fun Spinner(
    modifier: Modifier = Modifier,
    color: Color = PiliTheme.colors.textPrimary,
) {
    CircularProgressIndicator(modifier = modifier.size(SpinnerSize), color = color, strokeWidth = SpinnerStroke)
}
