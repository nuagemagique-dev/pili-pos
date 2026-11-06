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
import androidx.compose.foundation.layout.fillMaxWidth
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
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.theme.PiliColors
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import javax.inject.Inject

/** Stacking order of the toasts: above every other shell overlay. */
const val TOAST_OVERLAY_ORDER: Int = 900

/** Maximum width of a toast, as in ToastStack.vue (560 px, 1 dp is 1 CSS px). */
private val ToastMaxWidth = 560.dp

/** Height of the header (`--header-height` of the Vue UI): the toasts start just under it so they never cover it. */
private val HeaderHeight = 64.dp

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
 * ToastStack.vue (UX audit C5): the toasts stacked downwards at the top centre of the screen, just under the 64 dp
 * header, the newest at the bottom, at most [MAX_VISIBLE_TOASTS] of them (errors first, then the newest others,
 * see [pickVisibleToasts]). A toast is a dark card with a coloured left edge by variant. The stack never takes a
 * tap: a tap goes through to what is under it. Info, success and warning toasts let taps through too and go away
 * by themselves; an error toast stays until tapped, shows [tapToCloseText] under its message, and is the only
 * thing here that takes a tap (which closes it).
 * Nothing is drawn when [toasts] is empty. Test tags: `toast_stack`, `toast_<id>`, `toast_hint_<id>`.
 */
@Composable
fun ToastStack(
    toasts: List<Toast>,
    onDismiss: (Long) -> Unit,
    tapToCloseText: String,
    modifier: Modifier = Modifier,
) {
    if (toasts.isEmpty()) return
    val d = PiliTheme.dimens
    Box(
        modifier = modifier.fillMaxSize().padding(top = HeaderHeight + d.spaceSm, start = d.spaceMd, end = d.spaceMd),
        contentAlignment = Alignment.TopCenter,
    ) {
        Column(
            modifier =
                Modifier
                    .widthIn(max = ToastMaxWidth)
                    .fillMaxWidth()
                    .semantics { liveRegion = LiveRegionMode.Polite }
                    .testTag("toast_stack"),
            verticalArrangement = Arrangement.spacedBy(d.spaceSm),
        ) {
            pickVisibleToasts(toasts).forEach { toast -> ToastCard(toast, onDismiss, tapToCloseText) }
        }
    }
}

@Composable
private fun ToastCard(
    toast: Toast,
    onDismiss: (Long) -> Unit,
    tapToCloseText: String,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val shape = RoundedCornerShape(d.radiusMd)
    val sticky = toast.variant == ToastVariant.Error
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .shadow(ToastShadow, shape)
                .background(c.bgSecondary, shape)
                // Only an error takes the tap; every other toast lets it through.
                .then(if (sticky) Modifier.clickable { onDismiss(toast.id) } else Modifier)
                .height(IntrinsicSize.Min)
                .heightIn(min = ToastMinHeight)
                .testTag("toast_${toast.id}"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(ToastEdgeWidth).fillMaxHeight().background(accentOf(toast.variant, c)))
        Column(
            modifier = Modifier.padding(horizontal = d.spaceMd, vertical = d.spaceSm),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            Text(text = toast.message, style = t.label, color = c.textPrimary)
            if (sticky) {
                Text(
                    text = tapToCloseText,
                    style = t.body.copy(fontSize = t.sizeXs),
                    color = c.textSecondary,
                    modifier = Modifier.testTag("toast_hint_${toast.id}"),
                )
            }
        }
    }
}

/** The shell overlay that draws the [ToastStore] as a [ToastStack]. */
class ToastOverlay
    @Inject
    constructor(
        private val store: ToastStore,
        private val catalog: MessageCatalog,
        private val settings: PosSettingsStore,
    ) : ShellOverlay {
        override val order: Int = TOAST_OVERLAY_ORDER

        @Composable
        override fun Content() {
            val toasts by store.toasts.collectAsState()
            val language by settings.language.collectAsState()
            ToastStack(
                toasts = toasts,
                onDismiss = store::dismiss,
                tapToCloseText = catalog.text("toast_tap_to_close", language),
            )
        }
    }
