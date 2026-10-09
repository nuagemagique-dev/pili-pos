package dev.nuagemagique.pilipos.ui.feedback

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.collectLatest
import timber.log.Timber
import javax.inject.Inject
import kotlin.math.PI
import kotlin.math.pow
import kotlin.math.sin

/** Stacking order of the order chime: below every visible overlay (it draws nothing). */
const val ORDER_CHIME_OVERLAY_ORDER: Int = 10

/** Plays the sound that tells the operator a new online order is waiting. Never throws. */
interface OrderChimePlayer {
    fun play()
}

/** Sample rate of the generated chime, in hertz. */
internal const val CHIME_SAMPLE_RATE: Int = 44_100

/** C5, the first note of the chime. */
internal const val CHIME_FIRST_NOTE_HZ: Double = 523.25

/** E5, the second note of the chime. */
internal const val CHIME_SECOND_NOTE_HZ: Double = 659.25

/** Length of each note, in milliseconds. */
internal const val CHIME_NOTE_MS: Int = 150

/** Gain at the start of a note. */
internal const val CHIME_START_GAIN: Double = 0.3

/** Gain at the end of a note (the Web Audio exponential ramp target of the Vue chime). */
internal const val CHIME_END_GAIN: Double = 0.001

private const val MS_PER_SECOND = 1000
private const val RELEASE_MARGIN_MS = 200L
private const val BYTES_PER_SAMPLE = 2
private const val TWO_PI = 2.0 * PI

/**
 * The chime of the Vue UI (`useOrderChime.ts`) as 16-bit mono PCM at [CHIME_SAMPLE_RATE]: C5 for 150 ms, then E5
 * for 150 ms, each a sine whose gain falls exponentially from 0.3 to 0.001 over the note.
 */
internal fun chimeSamples(): ShortArray {
    val perNote = CHIME_SAMPLE_RATE * CHIME_NOTE_MS / MS_PER_SECOND
    val notes = listOf(CHIME_FIRST_NOTE_HZ, CHIME_SECOND_NOTE_HZ)
    val samples = ShortArray(perNote * notes.size)
    for ((index, hertz) in notes.withIndex()) {
        for (i in 0 until perNote) {
            val seconds = i.toDouble() / CHIME_SAMPLE_RATE
            val progress = i.toDouble() / perNote
            val gain = CHIME_START_GAIN * (CHIME_END_GAIN / CHIME_START_GAIN).pow(progress)
            val value = sin(TWO_PI * hertz * seconds) * gain
            samples[index * perNote + i] = (value * Short.MAX_VALUE).toInt().toShort()
        }
    }
    return samples
}

/**
 * Plays [chimeSamples] through an [AudioTrack] in static mode, then releases it. No sound file and no dependency
 * (UI-PD304). Any failure (no audio output, track refused) is logged and swallowed: a missing sound must never
 * break the till.
 */
class AudioTrackChimePlayer
    @Inject
    constructor() : OrderChimePlayer {
        @Suppress("TooGenericExceptionCaught")
        override fun play() {
            var track: AudioTrack? = null
            try {
                val samples = chimeSamples()
                val built = buildTrack(samples.size * BYTES_PER_SAMPLE)
                track = built
                built.write(samples, 0, samples.size)
                built.play()
                releaseLater(built, samples.size.toLong() * MS_PER_SECOND / CHIME_SAMPLE_RATE + RELEASE_MARGIN_MS)
            } catch (e: Exception) {
                Timber.w(e, "Order chime not played")
                release(track)
            }
        }

        private fun buildTrack(bytes: Int): AudioTrack =
            AudioTrack
                .Builder()
                .setAudioAttributes(
                    AudioAttributes
                        .Builder()
                        .setUsage(AudioAttributes.USAGE_NOTIFICATION)
                        .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                        .build(),
                ).setAudioFormat(
                    AudioFormat
                        .Builder()
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setSampleRate(CHIME_SAMPLE_RATE)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                        .build(),
                ).setBufferSizeInBytes(bytes)
                .setTransferMode(AudioTrack.MODE_STATIC)
                .build()

        private fun releaseLater(
            track: AudioTrack,
            afterMs: Long,
        ) {
            Handler(Looper.getMainLooper()).postDelayed({ release(track) }, afterMs)
        }

        @Suppress("TooGenericExceptionCaught")
        private fun release(track: AudioTrack?) {
            try {
                track?.release()
            } catch (e: Exception) {
                Timber.d(e, "Chime track not released")
            }
        }
    }

/**
 * Plays [player] each time a value of [counts] is higher than the one before. The first value is only a starting
 * point; a lower or equal value is silent.
 */
internal suspend fun playOnIncrease(
    counts: Flow<Int>,
    player: OrderChimePlayer,
) {
    var previous: Int? = null
    counts.collect { count ->
        val before = previous
        previous = count
        if (before != null && count > before) player.play()
    }
}

/**
 * `useOrderChime()` of the Vue UI: a two-tone chime each time the number of online orders waiting for the operator
 * goes up, so that a new order is noticed on any screen. Draws nothing.
 *
 * The count the store holds when its first fetch ends is the starting point (orders already waiting at start are
 * not announced); the starting point is taken again after the engine went away and came back.
 */
class OrderChimeOverlay
    @Inject
    constructor(
        private val store: WebshopOrdersStore,
        private val player: OrderChimePlayer,
    ) : ShellOverlay {
        override val order: Int = ORDER_CHIME_OVERLAY_ORDER

        @Composable
        override fun Content() {
            LaunchedEffect(store, player) {
                store.loaded.collectLatest { loaded -> if (loaded) playOnIncrease(store.pendingCount, player) }
            }
        }
    }
