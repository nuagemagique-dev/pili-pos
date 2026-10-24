package dev.nuagemagique.pilipos

import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import dagger.hilt.android.testing.BindValue
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.badge.BadgeOverlay
import dev.nuagemagique.pilipos.ui.badge.BadgeSources
import dev.nuagemagique.pilipos.ui.badge.KeyWedgeSource
import dev.nuagemagique.pilipos.ui.badge.NfcBadgeSource
import io.mockk.mockk
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import javax.inject.Inject

/** Lane 5: the real Hilt graph of the badge reader ([dev.nuagemagique.pilipos.ui.badge.BadgeModule] installed). */
@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class BadgeGraphTest {
    private val hiltRule = HiltAndroidRule(this)

    @BindValue
    @JvmField
    val forwarder: UserActivityForwarder = mockk(relaxed = true)

    private val composeRule = createAndroidComposeRule<MainActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(composeRule)

    @Inject
    lateinit var overlays: Set<@JvmSuppressWildcards ShellOverlay>

    @Inject
    lateinit var sources: BadgeSources

    @Inject
    lateinit var wedge: KeyWedgeSource

    @Inject
    lateinit var nfc: NfcBadgeSource

    @Test
    fun theShellOverlaysIncludeTheBadgeOverlayThatBuildsTheStore() {
        hiltRule.inject()
        assertTrue(overlays.any { it is BadgeOverlay })
    }

    @Test
    fun theSourcesAreTheSingletonsTheActivityFeeds() {
        hiltRule.inject()
        // The activity's dispatchKeyEvent feeds this very wedge; the store must collect the same instance.
        assertSame(composeRule.activity.wedge, sources.usb)
        assertSame(composeRule.activity.nfc, sources.nfc)
        assertSame(wedge, sources.usb)
        assertSame(nfc, sources.nfc)
    }
}
