package dev.nuagemagique.pilipos.feature.order

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.AnimationVector1D
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val FlashStroke = 3.dp

/** The state of the 200 ms flash of a cart line: its [alpha] and whether it is [active] (the test marker shows). */
internal class LineFlash(
    val alpha: Animatable<Float, AnimationVector1D>,
    val active: Boolean,
)

/** One green pulse each time [seq] changes to a new add (0 means no add reached the line). */
@Composable
internal fun rememberLineFlash(seq: Long): LineFlash {
    val durationMs = PiliTheme.dimens.flashDurationMs
    val alpha = remember { Animatable(0f) }
    LaunchedEffect(seq) {
        if (seq != 0L) {
            alpha.snapTo(1f)
            alpha.animateTo(0f, tween(durationMs, easing = LinearEasing))
        }
    }
    val active by remember { derivedStateOf { alpha.value > 0f } }
    return LineFlash(alpha, active)
}

/** The green frame of the flash at [alpha]; nothing once it is gone. */
internal fun DrawScope.drawFlash(
    color: Color,
    alpha: Float,
) {
    if (alpha > 0f) drawRect(color, alpha = alpha, style = Stroke(FlashStroke.toPx()))
}

/** A marker for the tests and the screen reader while the flash runs (`cart_line_<id>_flash`). */
@Composable
internal fun FlashMarker(
    lineId: Long,
    active: Boolean,
) {
    if (active) Spacer(Modifier.size(0.dp).testTag("cart_line_${lineId}_flash"))
}
