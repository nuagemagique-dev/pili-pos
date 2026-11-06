package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.state.IdleWarning
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Height of the banner (56 px in `IdleLockBanner.vue`). */
private val BannerHeight = 56.dp
private val BangSize = 28.dp

/** Whole seconds left of a countdown with [leftMs] milliseconds to go (rounded up, never negative). */
@Suppress("MagicNumber")
internal fun secondsLeft(leftMs: Long): Int = ((leftMs.coerceAtLeast(0L) + 999L) / 1000L).toInt()

/**
 * `IdleLockBanner.vue` (UX audit L4): under the header, "Verrouillage dans 30 s · touchez l'écran pour rester
 * connecté" with a live countdown. The ENGINE owns the lock and says when to warn; this only counts down the time
 * it was told, from the frame clock so it follows the screen (and a test clock). A touch is forwarded to the engine
 * as activity, which ends the warning and removes the banner. Amber, it takes its own row and no tap.
 * Test tag: `idle_lock_banner`.
 */
@Composable
fun IdleLockBanner(
    warning: IdleWarning,
    modifier: Modifier = Modifier,
) {
    var leftMs by remember(warning) { mutableLongStateOf(warning.lockInMs) }
    LaunchedEffect(warning) {
        val start = withFrameMillis { it }
        while (leftMs > 0L) {
            val now = withFrameMillis { it }
            leftMs = warning.lockInMs - (now - start)
        }
    }
    IdleLockBannerContent(secondsLeft(leftMs), modifier)
}

/** The stateless banner: [seconds] left before the lock. */
@Composable
internal fun IdleLockBannerContent(
    seconds: Int,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .height(BannerHeight)
                .background(colors.accentWarning)
                .padding(horizontal = PiliTheme.dimens.spaceLg)
                .semantics { liveRegion = LiveRegionMode.Polite }
                .testTag("idle_lock_banner"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier.size(BangSize).clip(CircleShape).background(colors.textOnAccent),
            contentAlignment = Alignment.Center,
        ) {
            Text("!", style = typography.body.copy(fontWeight = typography.weightHeavy), color = colors.accentWarning)
        }
        Text(
            text = vueText("idle_lock_warning", "s" to seconds),
            style = typography.body.copy(fontSize = typography.sizePrice, fontWeight = typography.weightBold),
            color = colors.textOnAccent,
            maxLines = 1,
            modifier = Modifier.testTag("idle_lock_text"),
        )
    }
}
