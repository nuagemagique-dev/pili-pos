package dev.nuagemagique.pilipos.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.NonRestartableComposable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.Density

/** Width in px of the Vue design; 1 dp equals 1 CSS px of it (DECISIONS Q19). */
const val DESIGN_WIDTH_PX: Float = 1920f

/** The colours of the current [PiliTheme]. */
val LocalPiliColors = staticCompositionLocalOf { PiliColors.Standard }

/** The layout tokens of the current [PiliTheme]. */
val LocalPiliDimens = staticCompositionLocalOf { PiliDimens() }

/** The type tokens of the current [PiliTheme]. */
val LocalPiliTypography = staticCompositionLocalOf { PiliTypography() }

/**
 * Provides the Pili tokens and a dark Material 3 colour scheme mapped from them. [training] swaps the surfaces
 * for the violet training palette (UX guide section 9).
 */
@Composable
@NonRestartableComposable
fun PiliTheme(
    training: Boolean = false,
    content: @Composable () -> Unit,
) {
    val colors = if (training) PiliColors.Training else PiliColors.Standard
    val colorScheme =
        darkColorScheme(
            background = colors.bgPrimary,
            onBackground = colors.textPrimary,
            surface = colors.bgSecondary,
            onSurface = colors.textPrimary,
            surfaceVariant = colors.bgSurface,
            onSurfaceVariant = colors.textSecondary,
            primary = colors.accentPrimary,
            onPrimary = colors.textOnAccent,
            secondary = colors.selectedBg,
            onSecondary = colors.selectedFg,
            tertiary = colors.accentInfo,
            error = colors.accentDanger,
            onError = colors.textOnAccent,
            outline = colors.border,
        )
    CompositionLocalProvider(
        LocalPiliColors provides colors,
        LocalPiliDimens provides PiliDimens(),
        LocalPiliTypography provides PiliTypography(),
    ) {
        MaterialTheme(colorScheme = colorScheme, content = content)
    }
}

/** Accessors for the tokens of the current theme. */
object PiliTheme {
    /** The colours. */
    val colors: PiliColors
        @Composable @ReadOnlyComposable
        get() = LocalPiliColors.current

    /** The layout tokens. */
    val dimens: PiliDimens
        @Composable @ReadOnlyComposable
        get() = LocalPiliDimens.current

    /** The type tokens. */
    val typography: PiliTypography
        @Composable @ReadOnlyComposable
        get() = LocalPiliTypography.current
}

/**
 * Maps the container width to the 1920 px design: provides `Density(widthPx / 1920f, 1f)` so 1 dp is 1 CSS px
 * of the Vue design (density 1.0 on the till at 1920 px), with the font scale fixed at 1 (DECISIONS Q19).
 * The width is read from the window container; when it is not known yet the parent density is kept.
 * Not restartable: it only provides a local, so it needs no skipping logic.
 */
@Composable
@NonRestartableComposable
fun PiliDensity(content: @Composable () -> Unit) {
    PiliDensity(containerWidthPx = LocalWindowInfo.current.containerSize.width, content = content)
}

/** Same as [PiliDensity], with an explicit container width in px (non-positive keeps the parent density). */
@Composable
@NonRestartableComposable
fun PiliDensity(
    containerWidthPx: Int,
    content: @Composable () -> Unit,
) {
    if (containerWidthPx <= 0) {
        content()
    } else {
        CompositionLocalProvider(LocalDensity provides piliDensity(containerWidthPx), content = content)
    }
}

/** The density of a container [widthPx] wide: `Density(widthPx / 1920f, 1f)`. */
fun piliDensity(widthPx: Int): Density = Density(density = widthPx / DESIGN_WIDTH_PX, fontScale = 1f)
