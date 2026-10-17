package dev.nuagemagique.pilipos.ui.badge

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.KeyEvent
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import javax.inject.Inject
import javax.inject.Singleton

/** Scans waiting for a collector; a reader is never faster than the engine call that follows. */
private const val SCAN_BUFFER = 4

/**
 * The USB keyboard wedge reader (plan L5-D12). `MainActivity.dispatchKeyEvent` offers each key event to
 * [dispatch] first. While the source is started, the key-downs that could be part of a badge are held and
 * swallowed (so a scan is neither typed into a field nor counted as user activity, L5-D11); a recognised badge
 * goes to [scans]; anything else is replayed, in order, through the activity's own dispatch, with a re-entry flag
 * so the detector never sees its own replay. A [Handler] releases the held keys when the gap has passed. With the
 * source stopped (reader not `usb`) every event passes.
 *
 * Main thread only, like the dispatch it hooks.
 */
@Singleton
class KeyWedgeSource internal constructor(
    private val clock: () -> Long,
    private val handler: Handler,
) : BadgeSource {
    @Inject
    constructor() : this({ SystemClock.uptimeMillis() }, Handler(Looper.getMainLooper()))

    private val detector = WedgeDetector<KeyEvent>()
    private val mutableScans = MutableSharedFlow<String>(extraBufferCapacity = SCAN_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    // The key codes whose DOWN was swallowed and whose UP has not come yet: their UP is swallowed too.
    private val swallowedUps = mutableSetOf<Int>()
    private var config = WedgeConfig()
    private var started = false
    private var replaying = false
    private var sink: ((KeyEvent) -> Boolean)? = null
    private val flushTask = Runnable { release() }

    override val scans: Flow<String> = mutableScans.asSharedFlow()

    override fun start() {
        started = true
    }

    override fun stop() {
        started = false
        release()
        swallowedUps.clear()
    }

    override fun configure(config: WedgeConfig) {
        release()
        this.config = config
        detector.configure(config)
    }

    /** Gives the replay a way back into the view hierarchy: the activity's own `super.dispatchKeyEvent`. */
    fun attach(sink: (KeyEvent) -> Boolean) {
        this.sink = sink
    }

    /** The activity is going away: held keys are let go first. */
    fun detach() {
        // Nobody is left to receive the held keys: drop them rather than replay into a dead window.
        handler.removeCallbacks(flushTask)
        detector.flush()
        sink = null
    }

    /** True when [event] was taken (held, swallowed or a badge): the activity must not dispatch it further. */
    fun dispatch(event: KeyEvent): Boolean =
        when {
            !started || replaying -> false
            event.action == KeyEvent.ACTION_UP -> swallowedUps.remove(event.keyCode)
            event.action != KeyEvent.ACTION_DOWN -> false
            else -> onDown(event)
        }

    private fun onDown(event: KeyEvent): Boolean {
        val result =
            detector.onKey(
                WedgeKey(
                    keyCode = event.keyCode,
                    event = event,
                    atMs = clock(),
                    repeat = event.repeatCount > 0,
                    ctrl = event.isCtrlPressed,
                    alt = event.isAltPressed,
                    meta = event.isMetaPressed,
                ),
            )
        replay(result.replay)
        return when (result) {
            is WedgeResult.Neutral -> false
            is WedgeResult.Pass -> {
                handler.removeCallbacks(flushTask)
                false
            }
            is WedgeResult.Hold -> {
                swallow(event)
                handler.removeCallbacks(flushTask)
                handler.postDelayed(flushTask, config.maxGapMs)
                true
            }
            is WedgeResult.Badge -> {
                swallow(event)
                handler.removeCallbacks(flushTask)
                mutableScans.tryEmit(result.token)
                true
            }
        }
    }

    private fun swallow(event: KeyEvent) {
        swallowedUps += event.keyCode
    }

    /** The gap passed with no key, or the source is going away: what is held was typing. */
    private fun release() {
        handler.removeCallbacks(flushTask)
        replay(detector.flush())
    }

    private fun replay(events: List<KeyEvent>) {
        val target = sink
        if (events.isEmpty() || target == null) return
        replaying = true
        try {
            for (down in events) {
                target(down)
                target(KeyEvent.changeAction(down, KeyEvent.ACTION_UP))
            }
        } finally {
            replaying = false
        }
    }
}
