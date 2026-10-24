package dev.nuagemagique.pilipos.ui.badge

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Handler
import android.os.Looper
import timber.log.Timber
import javax.inject.Inject
import kotlin.math.PI
import kotlin.math.pow
import kotlin.math.sin

/** The sound of a badge scan (plan L5-D14): a high two-tone for a registered scan, a low tone for a refusal. Never throws. */
interface BadgeCue {
    /** An arrival, a departure or a login was registered. */
    fun ok()

    /** The scan was refused or failed. */
    fun refused()
}

/** One note of a cue: frequency in hertz and length in milliseconds. */
internal data class CueNote(
    val hertz: Double,
    val lengthMs: Int,
)

/** A5 then E6, the `ok` cue of `useBadgeCue.ts`. */
@Suppress("MagicNumber")
internal val OkCue: List<CueNote> = listOf(CueNote(880.0, 120), CueNote(1318.5, 200))

/** G3, the `refused` cue of `useBadgeCue.ts`. */
@Suppress("MagicNumber")
internal val RefusedCue: List<CueNote> = listOf(CueNote(196.0, 350))

/** Sample rate of the generated cues, in hertz. */
internal const val CUE_SAMPLE_RATE: Int = 44_100

private const val CUE_START_GAIN = 0.3
private const val CUE_END_GAIN = 0.001
private const val MS_PER_SECOND = 1000
private const val RELEASE_MARGIN_MS = 200L
private const val BYTES_PER_SAMPLE = 2
private const val TWO_PI = 2.0 * PI

/**
 * The cue as 16-bit mono PCM: each note a sine whose gain falls exponentially from 0.3 to 0.001 over the note,
 * like the Web Audio synthesis of the Vue cue. No audio file.
 */
internal fun cueSamples(notes: List<CueNote>): ShortArray {
    val perNote = notes.map { CUE_SAMPLE_RATE * it.lengthMs / MS_PER_SECOND }
    val samples = ShortArray(perNote.sum())
    var offset = 0
    for ((index, note) in notes.withIndex()) {
        val count = perNote[index]
        for (i in 0 until count) {
            val seconds = i.toDouble() / CUE_SAMPLE_RATE
            val gain = CUE_START_GAIN * (CUE_END_GAIN / CUE_START_GAIN).pow(i.toDouble() / count)
            samples[offset + i] = (sin(TWO_PI * note.hertz * seconds) * gain * Short.MAX_VALUE).toInt().toShort()
        }
        offset += count
    }
    return samples
}

/** Plays the cues through an [AudioTrack] in static mode, then releases it. A missing sound never breaks the till. */
class AudioTrackBadgeCue
    @Inject
    constructor() : BadgeCue {
        override fun ok() = play(OkCue)

        override fun refused() = play(RefusedCue)

        @Suppress("TooGenericExceptionCaught")
        private fun play(notes: List<CueNote>) {
            var track: AudioTrack? = null
            try {
                val samples = cueSamples(notes)
                val built = buildTrack(samples.size * BYTES_PER_SAMPLE)
                track = built
                built.write(samples, 0, samples.size)
                built.play()
                val after = samples.size.toLong() * MS_PER_SECOND / CUE_SAMPLE_RATE + RELEASE_MARGIN_MS
                Handler(Looper.getMainLooper()).postDelayed({ release(built) }, after)
            } catch (e: Exception) {
                Timber.w(e, "Badge cue not played")
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
                        .setSampleRate(CUE_SAMPLE_RATE)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                        .build(),
                ).setBufferSizeInBytes(bytes)
                .setTransferMode(AudioTrack.MODE_STATIC)
                .build()

        @Suppress("TooGenericExceptionCaught")
        private fun release(track: AudioTrack?) {
            try {
                track?.release()
            } catch (e: Exception) {
                Timber.d(e, "Badge cue track not released")
            }
        }
    }
