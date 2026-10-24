package dev.nuagemagique.pilipos.ui.feedback

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.theme.PiliColors
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import javax.inject.Inject

/** Stacking order of the toasts: above every other shell overlay. */
const val TOAST_OVERLAY_ORDER: Int = 900

/** Maximum width of a toast, as in ToastStack.vue (360 px, 1 dp is 1 CSS px). */
private val ToastMaxWidth = 360.dp

/** Height of a toast at least (56 px in ToastStack.vue). */
private val ToastMinHeight = 56.dp

/** Width of the coloured edge of a toast (the 4 px left border). */
private val ToastEdgeWidth = 4.dp

/** Blur of the shadow of a toast (the 12 px of `box-shadow`). */
private val ToastShadow = 12.dp

private fun accentOf(
    variant: ToastVariant,
    colors: PiliColors,
): Color =
    when (variant) {
        ToastVariant.Info -> colors.accentInfo
        ToastVariant.Success -> colors.accentSuccess
        ToastVariant.Warning -> colors.accentWarning
        ToastVariant.Error -> colors.accentDanger
    }

/**
 * ToastStack.vue: the toasts stacked at the bottom right of the screen, the newest at the bottom. A toast is a
 * dark card with a coloured left edge by variant; a tap dismisses it. Nothing is drawn when [toasts] is empty,
 * and a tap outside the toasts goes through to what is below.
 */
@Composable
fun ToastStack(
    toasts: List<Toast>,
    onDismiss: (Long) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (toasts.isEmpty()) return
    val d = PiliTheme.dimens
    Box(modifier = modifier.fillMaxSize().padding(d.spaceMd), contentAlignment = Alignment.BottomEnd) {
        Column(
            modifier = Modifier.widthIn(max = ToastMaxWidth).semantics { liveRegion = LiveRegionMode.Polite }.testTag("toast_stack"),
            verticalArrangement = Arrangement.spacedBy(d.spaceSm),
        ) {
            toasts.forEach { toast -> ToastCard(toast, onDismiss) }
        }
    }
}

@Composable
private fun ToastCard(
    toast: Toast,
    onDismiss: (Long) -> Unit,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val shape = RoundedCornerShape(d.radiusMd)
    Row(
        modifier =
            Modifier
                .shadow(ToastShadow, shape)
                .background(c.bgSecondary, shape)
                .clickable { onDismiss(toast.id) }
                .height(IntrinsicSize.Min)
                .heightIn(min = ToastMinHeight)
                .testTag("toast_${toast.id}"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(ToastEdgeWidth).fillMaxHeight().background(accentOf(toast.variant, c)))
        Text(
            text = toast.message,
            style = t.label,
            color = c.textPrimary,
            modifier = Modifier.padding(horizontal = d.spaceMd, vertical = d.spaceSm),
        )
    }
}

/** The shell overlay that draws the [ToastStore] as a [ToastStack]. */
class ToastOverlay
    @Inject
    constructor(
        private val store: ToastStore,
    ) : ShellOverlay {
        override val order: Int = TOAST_OVERLAY_ORDER

        @Composable
        override fun Content() {
            val toasts by store.toasts.collectAsState()
            ToastStack(toasts = toasts, onDismiss = store::dismiss)
        }
    }
