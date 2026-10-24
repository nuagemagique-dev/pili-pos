package dev.nuagemagique.pilipos.ui.eft

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.state.EFT_CANCELLED
import dev.nuagemagique.pilipos.ui.state.EFT_DECLINED
import dev.nuagemagique.pilipos.ui.state.EftOverlayState
import dev.nuagemagique.pilipos.ui.state.EftPhase
import dev.nuagemagique.pilipos.ui.state.EftStore
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Alpha of the dark layer over the screen (0.78 as the Vue overlay). */
private const val BACKDROP_ALPHA = 0.78f

private val CardWidth = 640.dp
private val SpinnerSize = 56.dp
private val SpinnerStroke = 4.dp
private val ButtonHeight = 72.dp
private val CardBorder = 1.dp

/** What the operator can do on the overlay. */
internal data class EftOverlayCallbacks(
    val onCancel: () -> Unit = {},
    val onCheck: () -> Unit = {},
    val onDismiss: () -> Unit = {},
)

/**
 * The overlay of the linked terminal, tied to [store]: nothing is drawn unless a session is open. Put it in the
 * overlay slot of a screen that charges cards (checkout, online orders).
 */
@Composable
internal fun EftOverlayHost(store: EftStore) {
    val state by store.overlay.collectAsState()
    EftOverlay(state, EftOverlayCallbacks(store::abort, store::check, store::dismiss))
}

/**
 * The blocking overlay of a card payment on the linked terminal (`EftOverlay.vue`). It covers the screen while the
 * terminal waits for the card, then shows the answer:
 *
 *  - waiting: the amount, a spinner, "Annuler" (a request to the terminal),
 *  - checking: a spinner while `eftCheck` runs,
 *  - declined or cancelled: the reason, OK (the card part stays editable),
 *  - unknown: the sale waits for the terminal's answer, "Verifier",
 *  - refused before anything was charged: the error, OK.
 *
 * An approved payment closes the overlay by itself. Test tags: `eft_overlay`, `eft_title`, `eft_amount`,
 * `eft_progress`, `eft_spinner`, `eft_present`, `eft_cancel`, `eft_failure`, `eft_detail`, `eft_unknown_text`,
 * `eft_not_charged`, `eft_check`, `eft_ok`.
 */
@Composable
internal fun EftOverlay(
    state: EftOverlayState,
    callbacks: EftOverlayCallbacks,
    modifier: Modifier = Modifier,
) {
    if (!state.visible) return
    Box(
        modifier =
            modifier
                .fillMaxSize()
                .background(Color.Black.copy(alpha = BACKDROP_ALPHA))
                // Swallow every touch: nothing below the overlay can be reached while the terminal works.
                .pointerInput(Unit) { detectTapGestures { } }
                .semantics { liveRegion = LiveRegionMode.Assertive }
                .testTag("eft_overlay"),
        contentAlignment = Alignment.Center,
    ) {
        EftCard(state, callbacks)
    }
}

@Composable
private fun titleOf(state: EftOverlayState): String =
    if (state.phase == EftPhase.RESULT) {
        when {
            state.unknown -> uiText(R.string.ui_eft_unknown_title)
            state.payment?.status == EFT_DECLINED -> uiText(R.string.ui_eft_declined)
            state.payment?.status == EFT_CANCELLED -> uiText(R.string.ui_eft_cancelled)
            else -> uiText(R.string.ui_eft_refused)
        }
    } else {
        uiText(R.string.ui_eft_title)
    }

@Composable
private fun EftCard(
    state: EftOverlayState,
    callbacks: EftOverlayCallbacks,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val shape = RoundedCornerShape(d.radiusLg)
    Column(
        modifier =
            Modifier
                .width(CardWidth)
                .background(c.bgSecondary, shape)
                .border(CardBorder, c.border, shape)
                .padding(horizontal = d.spaceLg, vertical = d.spaceXl),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(d.spaceLg),
    ) {
        Text(
            text = titleOf(state),
            style = t.title,
            color = c.textPrimary,
            textAlign = TextAlign.Center,
            modifier = Modifier.testTag("eft_title"),
        )
        if (state.part != null && state.parts > 1) {
            Text(
                text = uiText(R.string.ui_eft_part, "n" to state.part, "count" to state.parts),
                style = t.label,
                color = c.textSecondary,
                modifier = Modifier.testTag("eft_progress"),
            )
        }
        Text(
            text = formatEuro(state.amountCents),
            style = t.price.copy(fontSize = t.sizeXl * 2),
            color = c.textPrimary,
            modifier = Modifier.testTag("eft_amount"),
        )
        if (state.busy) BusyBody(state, callbacks) else AnswerBody(state, callbacks)
    }
}

@Composable
private fun BusyBody(
    state: EftOverlayState,
    callbacks: EftOverlayCallbacks,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    CircularProgressIndicator(
        color = c.accentPrimary,
        strokeWidth = SpinnerStroke,
        modifier = Modifier.size(SpinnerSize).testTag("eft_spinner"),
    )
    val waiting = state.phase == EftPhase.WAITING
    Text(
        text = if (waiting) uiText(R.string.ui_eft_present_card) else uiText(R.string.ui_eft_checking),
        style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeLg),
        color = c.textSecondary,
        textAlign = TextAlign.Center,
        modifier = Modifier.testTag(if (waiting) "eft_present" else "eft_checking"),
    )
    if (waiting) {
        PiliButton(
            text = if (state.abortRequested) uiText(R.string.ui_eft_cancel_requested) else uiText(R.string.ui_eft_cancel),
            onClick = callbacks.onCancel,
            variant = PiliButtonVariant.Secondary,
            enabled = !state.abortRequested,
            modifier = Modifier.fillMaxWidth().height(ButtonHeight).padding(horizontal = d.spaceLg).testTag("eft_cancel"),
        )
    }
}

@Composable
private fun AnswerBody(
    state: EftOverlayState,
    callbacks: EftOverlayCallbacks,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val body = t.body.copy(fontSize = t.sizeLg)
    state.failure?.let {
        Text(
            text = it,
            style = body.copy(fontWeight = t.weightBold),
            color = c.accentDanger,
            textAlign = TextAlign.Center,
            modifier = Modifier.testTag("eft_failure"),
        )
    }
    if (state.unknown) {
        Text(
            text = uiText(R.string.ui_eft_unknown_explain),
            style = body,
            color = c.textSecondary,
            textAlign = TextAlign.Center,
            modifier = Modifier.testTag("eft_unknown_text"),
        )
    } else {
        state.payment?.detail?.let {
            Text(it, style = body, color = c.textSecondary, textAlign = TextAlign.Center, modifier = Modifier.testTag("eft_detail"))
        }
        if (state.payment != null && state.failure == null) {
            Text(
                text = uiText(R.string.ui_eft_not_charged),
                style = t.body,
                color = c.textSecondary,
                modifier = Modifier.testTag("eft_not_charged"),
            )
        }
    }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceMd)) {
        if (state.unknown) {
            PiliButton(
                text = uiText(R.string.ui_eft_check),
                onClick = callbacks.onCheck,
                modifier = Modifier.weight(1f).height(ButtonHeight).testTag("eft_check"),
            )
        }
        PiliButton(
            text = if (state.unknown) uiText(R.string.ui_eft_close) else uiText(R.string.ui_eft_ok),
            onClick = callbacks.onDismiss,
            variant = PiliButtonVariant.Secondary,
            modifier = Modifier.weight(1f).height(ButtonHeight).testTag("eft_ok"),
        )
    }
}
