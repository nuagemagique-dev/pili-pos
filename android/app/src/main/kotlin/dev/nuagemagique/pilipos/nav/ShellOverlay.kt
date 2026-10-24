package dev.nuagemagique.pilipos.nav

import androidx.compose.runtime.Composable

/**
 * A surface drawn above the navigation host (offline banner, update prompt, lock screen). Contributed
 * through the Hilt set; the shell draws the overlays by ascending [order].
 */
interface ShellOverlay {
    /** Stacking order, lower first (drawn below the higher ones). */
    val order: Int

    /** The overlay's content; it draws nothing when it has nothing to show. */
    @Composable
    fun Content()
}
