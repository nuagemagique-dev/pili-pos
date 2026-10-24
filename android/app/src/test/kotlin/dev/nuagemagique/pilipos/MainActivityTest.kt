package dev.nuagemagique.pilipos

import android.os.Looper
import android.os.SystemClock
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat
import dagger.hilt.android.testing.BindValue
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.engine.EngineService
import dev.nuagemagique.pilipos.kiosk.KioskState
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import io.mockk.mockk
import io.mockk.verify
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.util.concurrent.TimeUnit

@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class MainActivityTest {
    private val hiltRule = HiltAndroidRule(this)

    @BindValue
    @JvmField
    val forwarder: UserActivityForwarder = mockk(relaxed = true)

    private val composeRule = createAndroidComposeRule<MainActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(composeRule)

    @Test
    fun shellRootIsShown() {
        composeRule.onNodeWithTag("shell_root").assertIsDisplayed()
    }

    @Test
    fun shellRootShowsTheAppName() {
        assertEquals("Pili POS", composeRule.activity.getString(R.string.shell_app_name))
        composeRule.onNodeWithText("Pili POS").assertIsDisplayed()
    }

    @Test
    fun resumingTheActivityStartsTheEngineServiceInTheForeground() {
        val started = shadowOf(RuntimeEnvironment.getApplication()).nextStartedService
        assertNotNull(started)
        assertEquals(EngineService::class.java.name, started.component?.className)
    }

    @Test
    fun aTouchReachesTheForwarderThroughOnUserInteraction() {
        val now = SystemClock.uptimeMillis()
        val down = MotionEvent.obtain(now, now, MotionEvent.ACTION_DOWN, 10f, 10f, 0)
        composeRule.runOnUiThread { composeRule.activity.dispatchTouchEvent(down) }
        verify(atLeast = 1) { forwarder.onInteraction() }
    }

    @Test
    fun aKeyEventReachesTheForwarderThroughOnUserInteraction() {
        composeRule.runOnUiThread {
            composeRule.activity.dispatchKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER))
        }
        verify(atLeast = 1) { forwarder.onInteraction() }
    }

    @Test
    fun withTheReaderOffADigitKeyIsNotTakenByTheBadgeWedge() {
        composeRule.runOnUiThread {
            composeRule.activity.dispatchKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_5))
        }
        verify(atLeast = 1) { forwarder.onInteraction() }
    }

    @Test
    fun aKeyHeldByTheUsbBadgeWedgeIsSwallowedAndIsNotUserActivity() {
        composeRule.runOnUiThread {
            composeRule.activity.wedge.start()
            val taken = composeRule.activity.dispatchKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_5))
            assertEquals(true, taken)
        }
        verify(exactly = 0) { forwarder.onInteraction() }
    }

    @Test
    fun keysTheWedgeReleasesAreReplayedIntoTheViewHierarchyAndCountAsUserActivity() {
        val seen = mutableListOf<Pair<Int, Int>>()
        composeRule.runOnUiThread {
            val probe =
                View(composeRule.activity).apply {
                    isFocusable = true
                    isFocusableInTouchMode = true
                    setOnKeyListener { _, _, event ->
                        seen += event.action to event.keyCode
                        true
                    }
                }
            composeRule.activity.addContentView(probe, ViewGroup.LayoutParams(10, 10))
            probe.requestFocus()
            composeRule.activity.wedge.start()
            // A lone digit is held: nothing reaches the views, nothing is activity.
            assertEquals(true, composeRule.activity.dispatchKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_5)))
        }
        assertEquals(emptyList<Pair<Int, Int>>(), seen)
        verify(exactly = 0) { forwarder.onInteraction() }

        // The gap passes: the key is replayed through the activity's own dispatch (the attached sink).
        shadowOf(Looper.getMainLooper()).idleFor(500, TimeUnit.MILLISECONDS)

        assertEquals(listOf(KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_5, KeyEvent.ACTION_UP to KeyEvent.KEYCODE_5), seen)
        verify(atLeast = 1) { forwarder.onInteraction() }
    }

    @Test
    fun withoutTheOwnerRoleTheKioskStateIsNotOwner() {
        assertEquals(KioskState.NotOwner, composeRule.activity.kiosk.state.value)
    }

    @Test
    fun theSystemBarsAreHiddenOnResumeAndOnFocusGain() {
        val activity = composeRule.activity
        composeRule.runOnUiThread { activity.onWindowFocusChanged(true) }
        val controller = WindowCompat.getInsetsController(activity.window, activity.window.decorView)
        assertEquals(WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE, controller.systemBarsBehavior)
    }
}
