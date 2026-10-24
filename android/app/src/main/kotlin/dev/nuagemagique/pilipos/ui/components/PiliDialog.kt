package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Default maximum width of the dialog card. */
val PiliDialogMaxWidth: Dp = 640.dp

/** Alpha of the scrim behind a dialog (the Vue backdrop is 60 percent black; Encre at 70 percent here). */
private const val SCRIM_ALPHA = 0.7f

private val NoActions: @Composable RowScope.() -> Unit = {}

/**
 * A modal dialog drawn in the caller's layout, not in a `Dialog` window: a window would bring the system bars
 * back in the kiosk (PD214). Put it last in a `Box` that fills the screen, above the content it covers.
 *
 * A full-size scrim (`bgPrimary` at 70 percent) consumes every tap; a tap on it calls [onDismissRequest] when it
 * is not null. A tap on the card is consumed too. The card (`bgSecondary`, `radiusLg`, `spaceLg` padding, at most
 * [maxWidth] wide) shows the [title], the [content], then the [actions] aligned to the end.
 * Test tags: `pili_dialog` (root), `pili_dialog_scrim`, `pili_dialog_card`.
 */
@Composable
fun PiliDialog(
    title: String,
    onDismissRequest: (() -> Unit)?,
    modifier: Modifier = Modifier,
    maxWidth: Dp = PiliDialogMaxWidth,
    actions: @Composable RowScope.() -> Unit = NoActions,
    content: @Composable ColumnScope.() -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Box(modifier = modifier.fillMaxSize().testTag("pili_dialog"), contentAlignment = Alignment.Center) {
        Box(
            Modifier
                .fillMaxSize()
                .testTag("pili_dialog_scrim")
                .background(colors.bgPrimary.copy(alpha = SCRIM_ALPHA))
                .pointerInput(onDismissRequest) { detectTapGestures { onDismissRequest?.invoke() } },
        )
        Column(
            modifier =
                Modifier
                    .padding(dimens.spaceLg)
                    .widthIn(max = maxWidth)
                    .testTag("pili_dialog_card")
                    .semantics { paneTitle = title }
                    .background(colors.bgSecondary, RoundedCornerShape(dimens.radiusLg))
                    .pointerInput(Unit) { detectTapGestures { } }
                    .padding(dimens.spaceLg),
            verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
        ) {
            if (title.isNotEmpty()) {
                Text(title, style = PiliTheme.typography.title, color = colors.textPrimary)
            }
            content()
            if (actions !== NoActions) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(dimens.spaceSm, Alignment.End),
                    verticalAlignment = Alignment.CenterVertically,
                    content = actions,
                )
            }
        }
    }
}
