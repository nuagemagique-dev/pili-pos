package dev.nuagemagique.pilipos.kiosk

import android.app.Activity
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class ImmersiveTest {
    @Test
    fun systemBarsAreHiddenAndComeBackTransientlyOnSwipe() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        applyImmersive(activity.window)
        val controller = WindowCompat.getInsetsController(activity.window, activity.window.decorView)
        assertEquals(WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE, controller.systemBarsBehavior)
    }
}
