package dev.nuagemagique.pilipos.ui.badge

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.KeyEvent
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.onCompletion
import kotlinx.coroutines.flow.onStart
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
 * A screen can also [claim] the scans (UX audit P7, the gift voucher dialog): while a claim is collected the source
 * reads even with the badge reader off, with the widest limits (a voucher code is 12 digits, whatever the badge
 * length setting says), and every recognised scan goes to the claim instead of the badge store.
 *
 * Main thread only, like the dispatch it hooks.
 */
@Singleton
@Suppress("TooManyFunctions") // The detector hooks, the held keys, the replay and the claim of a screen.
class KeyWedgeSource internal constructor(
    private val clock: () -> Long,
    private val handler: Handler,
) : BadgeSource,
    ScanClaims {
    @Inject
    constructor() : this({ SystemClock.uptimeMillis() }, Handler(Looper.getMainLooper()))

    private val detector = WedgeDetector<KeyEvent>()
    private val mutableScans = MutableSharedFlow<String>(extraBufferCapacity = SCAN_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
    private val claimedScans = MutableSharedFlow<String>(extraBufferCapacity = SCAN_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    // The key codes whose DOWN was swallowed and whose UP has not come yet: their UP is swallowed too.
    private val swallowedUps = mutableSetOf<Int>()
    private var config = WedgeConfig()
    private var started = false
    private var claims = 0
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
        detector.configure(effectiveConfig())
    }

    /**
     * The scans of a screen that claimed them: collecting starts the claim, the end of the collection gives it back.
     * Each recognised scan goes here and not to [scans] while at least one claim is collected.
     */
    override fun claim(): Flow<String> =
        claimedScans
            .asSharedFlow()
            .onStart { changeClaims(+1) }
            .onCompletion { changeClaims(-1) }

    private fun changeClaims(by: Int) {
        // Whatever is held was read under the old limits: let it go before the limits change.
        release()
        claims += by
        detector.configure(effectiveConfig())
    }

    /** A claim widens the limits to the defaults, so a code longer than the badge length setting is still one scan. */
    private fun effectiveConfig(): WedgeConfig =
        if (claims > 0) {
            config.copy(minLen = WedgeConfig.DEFAULT_MIN_LEN, maxLen = WedgeConfig.DEFAULT_MAX_LEN)
        } else {
            config
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
            !(started || claims > 0) || replaying -> false
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
                if (claims > 0) claimedScans.tryEmit(result.token) else mutableScans.tryEmit(result.token)
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
