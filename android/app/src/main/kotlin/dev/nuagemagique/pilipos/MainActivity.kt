package dev.nuagemagique.pilipos

import android.annotation.SuppressLint
import android.os.Bundle
import android.view.KeyEvent
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import dagger.hilt.android.AndroidEntryPoint
import dev.nuagemagique.pilipos.di.AppLifecycleHooks
import dev.nuagemagique.pilipos.engine.EngineService
import dev.nuagemagique.pilipos.kiosk.KioskController
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import dev.nuagemagique.pilipos.kiosk.applyImmersive
import dev.nuagemagique.pilipos.shell.ShellRoot
import dev.nuagemagique.pilipos.ui.badge.KeyWedgeSource
import dev.nuagemagique.pilipos.ui.badge.NfcBadgeSource
import javax.inject.Inject

/**
 * The single activity (kiosk home, Lock Task). It hosts the Compose shell and the kiosk hooks (PD109,
 * PD110): the owner policies at creation, Lock Task entry and immersive bars on resume, and the
 * forwarding of every touch or key event to the engine as user activity.
 */
@SuppressLint("RestrictedApi")
@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    @Inject
    lateinit var kiosk: KioskController

    @Inject
    lateinit var forwarder: UserActivityForwarder

    @Inject
    lateinit var hooks: AppLifecycleHooks

    @Inject
    lateinit var wedge: KeyWedgeSource

    @Inject
    lateinit var nfc: NfcBadgeSource

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        kiosk.prepare()
        hooks.activityCreated(this)
        setContent { ShellRoot() }
    }

    override fun onResume() {
        super.onResume()
        kiosk.relock(this)
        applyImmersive(window)
        hooks.activityResumed(this)
        // Lane 5 (staff badges): the USB wedge replays held keys through this activity, the NFC reader mode runs
        // only while the activity is in the foreground.
        wedge.attach { event -> super.dispatchKeyEvent(event) }
        nfc.attach(this)
        // Q15: the foreground service starts after the relock, while the activity is visible.
        EngineService.start(this)
    }

    override fun onPause() {
        nfc.detach(this)
        wedge.detach()
        super.onPause()
    }

    /**
     * Offers every key event to the USB badge reader first (plan L5-D12): the keys of a possible badge are held and
     * swallowed here, so a scan is neither typed into a field nor counted as user activity. With the reader not
     * `usb` the source passes everything.
     */
    override fun dispatchKeyEvent(event: KeyEvent): Boolean = wedge.dispatch(event) || super.dispatchKeyEvent(event)

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            applyImmersive(window)
            kiosk.refreshState(this)
        }
    }

    /** Called for every touch, key and trackball event: the root interceptor of user activity. */
    override fun onUserInteraction() {
        super.onUserInteraction()
        forwarder.onInteraction()
    }
}
