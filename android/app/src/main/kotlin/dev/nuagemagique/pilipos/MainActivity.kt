package dev.nuagemagique.pilipos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import dagger.hilt.android.AndroidEntryPoint
import dev.nuagemagique.pilipos.engine.EngineService
import dev.nuagemagique.pilipos.kiosk.KioskController
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import dev.nuagemagique.pilipos.kiosk.applyImmersive
import dev.nuagemagique.pilipos.shell.ShellRoot
import javax.inject.Inject

/**
 * The single activity (kiosk home, Lock Task). It hosts the Compose shell and the kiosk hooks (PD109,
 * PD110): the owner policies at creation, Lock Task entry and immersive bars on resume, and the
 * forwarding of every touch or key event to the engine as user activity.
 */
@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    @Inject
    lateinit var kiosk: KioskController

    @Inject
    lateinit var forwarder: UserActivityForwarder

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        kiosk.prepare()
        setContent { ShellRoot() }
    }

    override fun onResume() {
        super.onResume()
        kiosk.relock(this)
        applyImmersive(window)
        // Q15: the foreground service starts after the relock, while the activity is visible.
        EngineService.start(this)
    }

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
