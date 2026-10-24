package dev.nuagemagique.pilipos.ui.fdm

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.delay
import uniffi.pos_ffi.FdmSeverity
import java.time.Clock
import java.time.Duration
import java.time.Instant

/** Alpha of the veil over the screen (the 0.86 of `rgba(11, 13, 14, 0.86)`). */
private const val VEIL_ALPHA = 0.86f

private const val TICK_MS = 1000L
private const val MS_PER_S = 1000L

private val CardWidth = 880.dp
private val CardBorder = 2.dp
private val BangSize = 96.dp
private val ButtonHeight = 72.dp
private val ButtonMinWidth = 360.dp

/**
 * The blocking part of `FdmAlerts.vue` (UX guide section 8.2): when the FDM is in blocking state the sales screens
 * are covered by a veil that cannot be dismissed. It shows the message, the code with the seconds until the
 * engine's next try (ticking each second, from [clock]) and "Reessayer maintenant", which calls [onCheckNow] and is
 * disabled while [FdmUiState.checking]. Nothing is drawn unless [FdmUiState.severity] is blocking; the scaffold
 * decides which screens get it (Reports and Settings stay reachable). Test tag: `fdm_veil`.
 */
@Composable
fun FdmBlockingVeil(
    state: FdmUiState,
    onCheckNow: () -> Unit,
    modifier: Modifier = Modifier,
    clock: Clock = Clock.systemUTC(),
) {
    if (state.severity != FdmSeverity.BLOCKING) return
    VeilContent(state, onCheckNow, modifier, clock)
}

@Composable
private fun VeilContent(
    state: FdmUiState,
    onCheckNow: () -> Unit,
    modifier: Modifier,
    clock: Clock,
) {
    val alert = rememberFdmAlertText(state.code)

    // Seconds until the engine's next try, refreshed every second.
    var now by remember { mutableStateOf(clock.instant()) }
    LaunchedEffect(clock) {
        while (true) {
            delay(TICK_MS)
            now = clock.instant()
        }
    }
    val retryIn = secondsUntil(state.nextCheckAt, now)

    Box(
        modifier =
            modifier
                .fillMaxSize()
                .background(PiliTheme.colors.bgPrimary.copy(alpha = VEIL_ALPHA))
                // Swallow every touch: the veil cannot be dismissed, nothing below it can be reached.
                .pointerInput(Unit) { detectTapGestures { } }
                .semantics { liveRegion = LiveRegionMode.Assertive }
                .testTag("fdm_veil"),
        contentAlignment = Alignment.Center,
    ) {
        VeilCard(state, alert, retryIn, onCheckNow)
    }
}

@Composable
private fun VeilCard(
    state: FdmUiState,
    alert: FdmAlertText,
    retryIn: Long?,
    onCheckNow: () -> Unit,
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
                .border(CardBorder, c.accentDanger, shape)
                .padding(d.spaceXl + d.spaceLg),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(d.spaceLg),
    ) {
        Box(
            modifier = Modifier.size(BangSize).background(c.accentDanger, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Text(text = "!", style = t.title.copy(fontSize = t.sizeXl * 2), color = c.textOnAccent)
        }
        Text(
            text = alert.message?.french.orEmpty(),
            style = t.title,
            color = c.textPrimary,
            textAlign = TextAlign.Center,
            modifier = Modifier.testTag("fdm_veil_message"),
        )
        alert.message?.chinese?.let { zh ->
            Text(
                text = zh,
                style = t.body.copy(fontSize = t.sizeXl, fontFamily = t.familyZh),
                color = c.textSecondary,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("fdm_veil_message_zh"),
            )
        }
        val retry = retryIn?.let { vueText("fdm_retry_in", "s" to it) }
        Text(
            text = listOfNotNull(alert.technicianCode, retry).joinToString(" \u00B7 "),
            style = t.body.copy(fontSize = t.sizeSm, fontFamily = t.familyFigures),
            color = c.textSecondary,
            modifier = Modifier.testTag("fdm_veil_code"),
        )
        PiliButton(
            text = vueText("fdm_retry_now"),
            onClick = onCheckNow,
            variant = PiliButtonVariant.Secondary,
            enabled = !state.checking,
            modifier = Modifier.height(ButtonHeight).widthIn(min = ButtonMinWidth).testTag("fdm_veil_retry"),
        )
    }
}

/** Whole seconds from [now] to [next], rounded up and never below zero; null when the engine did not say. */
internal fun secondsUntil(
    next: Instant?,
    now: Instant,
): Long? {
    if (next == null) return null
    val millis = Duration.between(now, next).toMillis()
    return if (millis <= 0L) 0L else (millis + MS_PER_S - 1L) / MS_PER_S
}
