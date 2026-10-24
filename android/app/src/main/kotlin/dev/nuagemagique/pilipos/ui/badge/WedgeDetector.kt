package dev.nuagemagique.pilipos.ui.badge

import android.view.KeyEvent

/** Timing and length limits of the USB wedge reader (`wedge_max_gap_ms`, `wedge_min_len`, `wedge_max_len`). */
data class WedgeConfig(
    /** Largest gap between two keys of a burst, in milliseconds. */
    val maxGapMs: Long = DEFAULT_MAX_GAP_MS,
    val minLen: Int = DEFAULT_MIN_LEN,
    val maxLen: Int = DEFAULT_MAX_LEN,
) {
    companion object {
        const val DEFAULT_MAX_GAP_MS: Long = 50L
        const val DEFAULT_MIN_LEN: Int = 6
        const val DEFAULT_MAX_LEN: Int = 32
    }
}

/**
 * One key-down offered to the detector. [event] is the opaque payload handed back for a replay (the real
 * [KeyEvent] in the app, anything in a test); [atMs] is any monotonic clock in milliseconds.
 */
data class WedgeKey<E>(
    val keyCode: Int,
    val event: E,
    val atMs: Long,
    val repeat: Boolean = false,
    val ctrl: Boolean = false,
    val alt: Boolean = false,
    val meta: Boolean = false,
)

/** What to do with a key offered to [WedgeDetector.onKey]. [replay] is released first, in order, in every case. */
sealed interface WedgeResult<E> {
    val replay: List<E>

    /** Not for the detector: let the event through untouched. */
    data class Neutral<E>(
        override val replay: List<E>,
    ) : WedgeResult<E>

    /** Swallow the event: it is held, or part of a held burst. */
    data class Hold<E>(
        override val replay: List<E>,
    ) : WedgeResult<E>

    /** Swallow the event and hand over the badge number. */
    data class Badge<E>(
        override val replay: List<E>,
        val token: String,
    ) : WedgeResult<E>

    /** Let the event through, after the [replay]. */
    data class Pass<E>(
        override val replay: List<E>,
    ) : WedgeResult<E>
}

private const val ENTER_CODE = KeyEvent.KEYCODE_ENTER
private const val NUMPAD_ENTER_CODE = KeyEvent.KEYCODE_NUMPAD_ENTER

private val MODIFIER_CODES: Set<Int> =
    setOf(
        KeyEvent.KEYCODE_SHIFT_LEFT,
        KeyEvent.KEYCODE_SHIFT_RIGHT,
        KeyEvent.KEYCODE_CTRL_LEFT,
        KeyEvent.KEYCODE_CTRL_RIGHT,
        KeyEvent.KEYCODE_ALT_LEFT,
        KeyEvent.KEYCODE_ALT_RIGHT,
        KeyEvent.KEYCODE_META_LEFT,
        KeyEvent.KEYCODE_META_RIGHT,
        KeyEvent.KEYCODE_CAPS_LOCK,
        KeyEvent.KEYCODE_NUM_LOCK,
        KeyEvent.KEYCODE_SYM,
        KeyEvent.KEYCODE_FUNCTION,
    )

/**
 * The badge character of a PHYSICAL key (plan L5-D12), never of the character the layout types: the digit row and
 * the numeric keypad give `0` to `9`, the letter keys `A` to `F` give the uppercase letter. Any other key is null.
 * An AZERTY layout, whose digit row types `&e"'(`, reads the same badge.
 */
fun badgeChar(keyCode: Int): Char? =
    when (keyCode) {
        in KeyEvent.KEYCODE_0..KeyEvent.KEYCODE_9 -> '0' + (keyCode - KeyEvent.KEYCODE_0)
        in KeyEvent.KEYCODE_NUMPAD_0..KeyEvent.KEYCODE_NUMPAD_9 -> '0' + (keyCode - KeyEvent.KEYCODE_NUMPAD_0)
        in KeyEvent.KEYCODE_A..KeyEvent.KEYCODE_F -> 'A' + (keyCode - KeyEvent.KEYCODE_A)
        else -> null
    }

/**
 * USB "keyboard wedge" detection, the same algorithm and defaults as `ui/src/utils/badgeWedge.ts` (plan L5-D12).
 *
 * A wedge reader types the badge number and Enter like a very fast keyboard. Every candidate key is HELD (the
 * caller swallows the event); the burst is a badge when it ends with Enter, every gap between consecutive keys is
 * at most [WedgeConfig.maxGapMs] and its length is within the configured limits. Anything else is released as a
 * replay, in order, before the key that broke it goes through. Shift, Ctrl, Alt and Meta and key repeats are
 * neutral. Pure: no timer (the caller calls [flush] when the gap passed), and the time comes with each key.
 */
class WedgeDetector<E>(
    private var config: WedgeConfig = WedgeConfig(),
) {
    private val held = mutableListOf<Pair<Char, E>>()
    private var lastAt: Long? = null

    // A burst longer than the limit was replayed: the rest of it, as long as it keeps the pace, is typing too.
    private var overlong = false

    /** New timing; anything held is released first by the caller ([flush]). */
    fun configure(config: WedgeConfig) {
        this.config = config
    }

    /** True while keys are held. */
    val pending: Boolean
        get() = held.isNotEmpty()

    /** Lets go of what is held (the gap timer fired): the events to replay, in order. */
    fun flush(): List<E> {
        val events = held.map { it.second }
        held.clear()
        return events
    }

    /** Offers one key-down; the result says what the caller does with it. */
    @Suppress("CyclomaticComplexMethod", "ReturnCount")
    fun onKey(key: WedgeKey<E>): WedgeResult<E> {
        if (key.keyCode in MODIFIER_CODES) return WedgeResult.Neutral(emptyList())
        if (key.repeat) {
            // A held key repeating inside a burst is swallowed, not counted.
            return if (held.isNotEmpty()) WedgeResult.Hold(emptyList()) else WedgeResult.Neutral(emptyList())
        }
        // A gap longer than the limit ends the burst: it was typing.
        val previous = lastAt
        val gapped = previous == null || key.atMs - previous > config.maxGapMs
        val replay = if (gapped && held.isNotEmpty()) flush() else emptyList()
        if (gapped) overlong = false
        lastAt = key.atMs

        val chord = key.ctrl || key.alt || key.meta
        val ch = if (chord) null else badgeChar(key.keyCode)
        val enter = !chord && (key.keyCode == ENTER_CODE || key.keyCode == NUMPAD_ENTER_CODE)
        return when {
            enter -> onEnter(replay)
            ch == null -> {
                // Any other key breaks the burst; it goes through after the replay.
                overlong = false
                WedgeResult.Pass(replay + flush())
            }
            overlong -> WedgeResult.Pass(replay)
            held.size + 1 > config.maxLen -> {
                // The (maxLen + 1)th key: not a badge. Replay it all, in order.
                overlong = true
                WedgeResult.Pass(replay + flush())
            }
            else -> {
                held += ch to key.event
                WedgeResult.Hold(replay)
            }
        }
    }

    @Suppress("ReturnCount")
    private fun onEnter(replay: List<E>): WedgeResult<E> {
        if (held.isEmpty()) {
            overlong = false
            return WedgeResult.Pass(replay)
        }
        val length = held.size
        if (length in config.minLen..config.maxLen) {
            val token = held.joinToString("") { it.first.toString() }
            held.clear()
            return WedgeResult.Badge(replay, token)
        }
        return WedgeResult.Pass(replay + flush())
    }
}
