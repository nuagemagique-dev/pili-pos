@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.ui.components

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Look of a [PiliButton]; the colours come from the theme (UX guide section 4.1). */
enum class PiliButtonVariant {
    /** Volt fill: the main action of the screen, once per screen. */
    Primary,

    /** Surface fill: every other action. */
    Secondary,

    /** Red fill: delete, destructive confirmation. */
    Danger,

    /** No fill: a quiet action next to a filled one. */
    Ghost,
}

/** Minimum height of a button (UX guide section 4.1: 48 dp is the floor of every secondary action). */
private val ButtonMinHeight = 48.dp

/** Alpha of a disabled control. */
private const val DISABLED_ALPHA = 0.4f

/** Share of black mixed into a pressed Volt or red fill (UX guide section 4.2: darker on touch). */
private const val PRESSED_DARKEN = 0.2f

/**
 * A text button. The caption is a parameter resolved by the caller (the component never looks a Vue key up).
 * Pressed feedback is [tapScale] plus a darker fill; [enabled] false draws it faded and not clickable.
 */
@Composable
fun PiliButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    variant: PiliButtonVariant = PiliButtonVariant.Primary,
    enabled: Boolean = true,
) {
    PiliButton(onClick = onClick, modifier = modifier, variant = variant, enabled = enabled) {
        Text(
            text = text,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = LocalContentColor.current,
            textAlign = TextAlign.Center,
        )
    }
}

/**
 * A button around a content slot. The slot is laid out in a centered [Row]; [LocalContentColor] is set to the
 * foreground of the [variant] so a Material `Text` or `Icon` inside picks it up.
 */
@Composable
fun PiliButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    variant: PiliButtonVariant = PiliButtonVariant.Primary,
    enabled: Boolean = true,
    content: @Composable RowScope.() -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val fill =
        when (variant) {
            PiliButtonVariant.Primary -> if (pressed) darker(colors.accentPrimary) else colors.accentPrimary
            PiliButtonVariant.Secondary -> if (pressed) colors.bgPressed else colors.bgSurface
            PiliButtonVariant.Danger -> if (pressed) darker(colors.accentDanger) else colors.accentDanger
            PiliButtonVariant.Ghost -> if (pressed) colors.bgPressed else Color.Transparent
        }
    val foreground =
        when (variant) {
            PiliButtonVariant.Primary, PiliButtonVariant.Danger -> colors.textOnAccent
            PiliButtonVariant.Secondary, PiliButtonVariant.Ghost -> colors.textPrimary
        }
    val shape = RoundedCornerShape(dimens.radiusMd)
    Row(
        modifier =
            modifier
                .tapScale(interactionSource)
                .alpha(if (enabled) 1f else DISABLED_ALPHA)
                .defaultMinSize(minHeight = ButtonMinHeight)
                .background(fill, shape)
                .clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    enabled = enabled,
                    role = Role.Button,
                    onClick = onClick,
                ).padding(horizontal = dimens.spaceMd, vertical = dimens.spaceSm),
        horizontalArrangement = Arrangement.spacedBy(dimens.spaceSm, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CompositionLocalProvider(LocalContentColor provides foreground) { content() }
    }
}

/**
 * Scales the node down to `dimens.tapScale` over `dimens.tapDurationMs` while [interactionSource] reports a
 * press (UX guide section 4.2). Reusable by every pressable control of the UI lane.
 */
@Composable
fun Modifier.tapScale(interactionSource: InteractionSource): Modifier {
    val pressed by interactionSource.collectIsPressedAsState()
    val dimens = PiliTheme.dimens
    val scale by animateFloatAsState(
        targetValue = if (pressed) dimens.tapScale else 1f,
        animationSpec = tween(durationMillis = dimens.tapDurationMs, easing = LinearEasing),
        label = "tapScale",
    )
    return this.graphicsLayer {
        scaleX = scale
        scaleY = scale
    }
}

private fun darker(color: Color): Color = lerp(color, Color.Black, PRESSED_DARKEN)
