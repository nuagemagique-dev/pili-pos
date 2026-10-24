package dev.nuagemagique.pilipos

import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.Lifecycle
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.testing.BindValue
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dagger.hilt.android.testing.UninstallModules
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.badge.AudioTrackBadgeCue
import dev.nuagemagique.pilipos.ui.badge.BadgeCue
import dev.nuagemagique.pilipos.ui.badge.BadgeModule
import dev.nuagemagique.pilipos.ui.badge.BadgeOverlay
import dev.nuagemagique.pilipos.ui.badge.BadgeSources
import dev.nuagemagique.pilipos.ui.badge.FakeNfcSupport
import dev.nuagemagique.pilipos.ui.badge.KeyWedgeSource
import dev.nuagemagique.pilipos.ui.badge.NfcBadgeSource
import dev.nuagemagique.pilipos.ui.badge.NfcSupport
import io.mockk.mockk
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import javax.inject.Singleton

/**
 * Lane 5, activity wiring of the built-in NFC reader (plan L5-D13): the reader mode starts while the activity is
 * resumed and ends with it. [BadgeModule] is swapped for the same bindings minus the device NFC, which is a fake.
 */
@HiltAndroidTest
@UninstallModules(BadgeModule::class)
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class MainActivityNfcTest {
    private val hiltRule = HiltAndroidRule(this)

    @BindValue
    @JvmField
    val forwarder: UserActivityForwarder = mockk(relaxed = true)

    private val fake = FakeNfcSupport(hasNfc = true, isEnabled = true)

    @BindValue
    @JvmField
    val support: NfcSupport = fake

    private val composeRule = createAndroidComposeRule<MainActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(composeRule)

    /** [BadgeModule] without the [NfcSupport] binding. */
    @Module
    @InstallIn(SingletonComponent::class)
    object TestBadgeModule {
        @Provides
        @IntoSet
        fun badgeOverlay(overlay: BadgeOverlay): ShellOverlay = overlay

        @Provides
        fun badgeCue(cue: AudioTrackBadgeCue): BadgeCue = cue

        @Provides
        @Singleton
        fun badgeSources(
            wedge: KeyWedgeSource,
            nfc: NfcBadgeSource,
        ): BadgeSources = BadgeSources(wedge, nfc)
    }

    @Test
    fun theReaderModeRunsWhileTheActivityIsResumedAndEndsWithIt() {
        val nfc = composeRule.activity.nfc
        composeRule.runOnUiThread { nfc.start() }
        assertEquals(listOf("enable"), fake.calls)
        assertTrue(nfc.active)

        // onPause: the reader mode must end with the activity.
        composeRule.activityRule.scenario.moveToState(Lifecycle.State.STARTED)
        assertEquals(listOf("enable", "disable"), fake.calls)
        assertFalse(nfc.active)

        // onResume: it comes back (the setting is still nfc).
        composeRule.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        assertEquals(listOf("enable", "disable", "enable"), fake.calls)
        assertTrue(nfc.active)
    }
}
