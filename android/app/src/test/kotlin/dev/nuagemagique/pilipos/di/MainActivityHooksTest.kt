package dev.nuagemagique.pilipos.di

import android.app.Activity
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.testing.BindValue
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.kiosk.KioskController
import dev.nuagemagique.pilipos.kiosk.KioskState
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertSame
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

/** What the hook saw and in which order; static because Hilt builds the module outside the test instance. */
object HookProbe {
    val events: MutableList<String> = mutableListOf()
    var serviceStartedAtResume: Boolean? = null
    var barsBehaviorAtResume: Int? = null
    var createdActivity: Activity? = null
    var resumedActivity: Activity? = null

    fun reset() {
        events.clear()
        serviceStartedAtResume = null
        barsBehaviorAtResume = null
        createdActivity = null
        resumedActivity = null
    }
}

/** Order of the hooks of the activity relative to the kiosk calls and to the start of the engine service. */
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class MainActivityHooksTest {
    private val hiltRule = HiltAndroidRule(this)

    @BindValue
    @JvmField
    val forwarder: UserActivityForwarder = mockk(relaxed = true)

    @BindValue
    @JvmField
    val kiosk: KioskController =
        mockk<KioskController>(relaxed = true).also { mock ->
            // Runs when the test instance is built, before the rule launches the activity.
            HookProbe.reset()
            every { mock.state } returns MutableStateFlow<KioskState>(KioskState.NotOwner)
            every { mock.prepare() } answers { HookProbe.events += "kiosk.prepare" }
            every { mock.relock(any()) } answers { HookProbe.events += "kiosk.relock" }
        }

    private val composeRule = createAndroidComposeRule<MainActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(composeRule)

    @Module
    @InstallIn(SingletonComponent::class)
    object ProbeHookModule {
        @Provides
        @IntoSet
        fun provideProbeHook(): AppLifecycleHook =
            object : AppLifecycleHook {
                override val order = 0

                override fun onActivityCreated(activity: Activity) {
                    HookProbe.events += "hook.created"
                    HookProbe.createdActivity = activity
                }

                override fun onActivityResumed(activity: Activity) {
                    HookProbe.events += "hook.resumed"
                    HookProbe.resumedActivity = activity
                    HookProbe.serviceStartedAtResume = shadowOf(activity.application).peekNextStartedService() != null
                    HookProbe.barsBehaviorAtResume =
                        WindowCompat.getInsetsController(activity.window, activity.window.decorView).systemBarsBehavior
                }
            }
    }

    @Test
    fun activityCreatedRunsRightAfterKioskPrepare() {
        val events = HookProbe.events
        assertEquals(events.toString(), events.indexOf("kiosk.prepare") + 1, events.indexOf("hook.created"))
        assertSame(composeRule.activity, HookProbe.createdActivity)
    }

    @Test
    fun activityResumedRunsAfterTheRelockAndTheImmersiveBars() {
        val events = HookProbe.events
        assertEquals(events.toString(), events.indexOf("kiosk.relock") + 1, events.indexOf("hook.resumed"))
        assertEquals(
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE,
            HookProbe.barsBehaviorAtResume,
        )
        assertSame(composeRule.activity, HookProbe.resumedActivity)
    }

    @Test
    fun activityResumedRunsBeforeTheEngineServiceStarts() {
        assertEquals(false, HookProbe.serviceStartedAtResume)
        assertNotNull(shadowOf(composeRule.activity.application).nextStartedService)
    }

    @Test
    fun theCreatedHookRunsBeforeTheResumedHookExactlyOnce() {
        assertEquals(listOf("kiosk.prepare", "hook.created", "kiosk.relock", "hook.resumed"), HookProbe.events)
        assertFalse(HookProbe.events.isEmpty())
    }
}
