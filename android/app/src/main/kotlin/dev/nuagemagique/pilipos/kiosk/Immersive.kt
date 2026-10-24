package dev.nuagemagique.pilipos.kiosk

import android.view.Window
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

/**
 * Hides the status and navigation bars of [window]; a swipe from the edge shows them for a moment
 * (S4: the usable height is 996 px of 1080 without it). Call it on resume and on every focus gain,
 * because the system shows the bars again after a dialog or a transient reveal.
 */
fun applyImmersive(window: Window) {
    val controller = WindowCompat.getInsetsController(window, window.decorView)
    controller.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    controller.hide(WindowInsetsCompat.Type.systemBars())
}
