package dev.nuagemagique.pilipos.ui.feedback

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.state.LineAdd
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filterNotNull
import timber.log.Timber
import javax.inject.Inject
import kotlin.math.PI
import kotlin.math.pow
import kotlin.math.sin

/** Stacking order of the add beep: below every visible overlay (it draws nothing). */
const val ADD_BEEP_OVERLAY_ORDER: Int = 11

/** Plays the short beep that confirms a product added to the cart. Never throws. */
interface AddBeepPlayer {
    fun play()
}

/** Sample rate of the generated beep, in hertz. */
internal const val BEEP_SAMPLE_RATE: Int = 44_100

/** Pitch of the beep, A5 (`ADD_BEEP_HZ` of `useAddBeep.ts`). */
internal const val ADD_BEEP_HZ: Double = 880.0

/** Length of the beep, in milliseconds (`ADD_BEEP_MS` of `useAddBeep.ts`). */
internal const val ADD_BEEP_MS: Int = 60

/** Gain at the start of the beep. */
internal const val BEEP_START_GAIN: Double = 0.2

/** Gain at the end of the beep (the exponential ramp target of the Vue beep). */
internal const val BEEP_END_GAIN: Double = 0.001

private const val MS_PER_SECOND = 1000
private const val RELEASE_MARGIN_MS = 200L
private const val BYTES_PER_SAMPLE = 2
private const val TWO_PI = 2.0 * PI

/**
 * The beep of the Vue UI (`useAddBeep.ts`) as 16-bit mono PCM at [BEEP_SAMPLE_RATE]: a sine at [ADD_BEEP_HZ] for
 * [ADD_BEEP_MS] whose gain falls exponentially from [BEEP_START_GAIN] to [BEEP_END_GAIN].
 */
internal fun beepSamples(): ShortArray {
    val count = BEEP_SAMPLE_RATE * ADD_BEEP_MS / MS_PER_SECOND
    return ShortArray(count) { i ->
        val seconds = i.toDouble() / BEEP_SAMPLE_RATE
        val gain = BEEP_START_GAIN * (BEEP_END_GAIN / BEEP_START_GAIN).pow(i.toDouble() / count)
        (sin(TWO_PI * ADD_BEEP_HZ * seconds) * gain * Short.MAX_VALUE).toInt().toShort()
    }
}

/**
 * Plays [beepSamples] through an [AudioTrack] in static mode, then releases it, like the order chime. Any failure
 * (no audio output, track refused) is logged and swallowed: a missing sound must never break the till.
 */
class AudioTrackBeepPlayer
    @Inject
    constructor() : AddBeepPlayer {
        @Suppress("TooGenericExceptionCaught")
        override fun play() {
            var track: AudioTrack? = null
            try {
                val samples = beepSamples()
                val built = buildTrack(samples.size * BYTES_PER_SAMPLE)
                track = built
                built.write(samples, 0, samples.size)
                built.play()
                releaseLater(built, samples.size.toLong() * MS_PER_SECOND / BEEP_SAMPLE_RATE + RELEASE_MARGIN_MS)
            } catch (e: Exception) {
                Timber.w(e, "Add beep not played")
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
                        .setSampleRate(BEEP_SAMPLE_RATE)
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
                Timber.d(e, "Beep track not released")
            }
        }
    }

/** Plays [player] for each add of [adds] while [enabled] says the switch is on; the switch is read at each add. */
internal suspend fun playOnAdd(
    adds: Flow<LineAdd>,
    enabled: () -> Boolean,
    player: AddBeepPlayer,
) {
    adds.collect { if (enabled()) player.play() }
}

/**
 * `useAddBeep()` of the Vue UI: a short beep on every successful add to the cart (grid tap, code pad OK, digit after
 * an add, option and price dialogs) while this terminal's "Bip à chaque ajout" switch is on (default on). Draws
 * nothing; the cart line flashes on the same add.
 */
class AddBeepOverlay
    @Inject
    constructor(
        private val orders: OrderStore,
        private val settings: PosSettingsStore,
        private val player: AddBeepPlayer,
    ) : ShellOverlay {
        override val order: Int = ADD_BEEP_OVERLAY_ORDER

        @Composable
        override fun Content() {
            // The value held when the overlay starts is an old add (a recomposition must not beep again): dropped.
            LaunchedEffect(orders, settings, player) {
                playOnAdd(orders.lastAdd.drop(1).filterNotNull(), { settings.addBeep.value }, player)
            }
        }
    }
