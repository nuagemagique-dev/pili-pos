package dev.nuagemagique.pilipos.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * The Pili layout tokens, ported from `ui/src/styles/tokens.css`. By DECISIONS Q19, 1 dp equals 1 CSS px of
 * the 1920 px wide Vue design (see [PiliDensity]), so the values are the CSS pixel values.
 */
@Immutable
data class PiliDimens(
    /** 4 dp. */
    val spaceXs: Dp = 4.dp,
    /** 8 dp. */
    val spaceSm: Dp = 8.dp,
    /** 16 dp. */
    val spaceMd: Dp = 16.dp,
    /** 24 dp. */
    val spaceLg: Dp = 24.dp,
    /** 32 dp. */
    val spaceXl: Dp = 32.dp,
    /** 8 dp corner radius. */
    val radiusSm: Dp = 8.dp,
    /** 10 dp corner radius. */
    val radiusMd: Dp = 10.dp,
    /** 16 dp corner radius. */
    val radiusLg: Dp = 16.dp,
    /** Height of the header bar. */
    val headerHeight: Dp = 64.dp,
    /** Width of the cart column. */
    val cartWidth: Dp = 670.dp,
    /** Scale of a pressed control (UX guide section 4.2). */
    val tapScale: Float = 0.95f,
    /** Duration of the tap scale animation, in milliseconds. */
    val tapDurationMs: Int = 100,
    /** Duration of a confirmation flash, in milliseconds. */
    val flashDurationMs: Int = 200,
)
