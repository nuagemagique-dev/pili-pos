package dev.nuagemagique.pilipos.ui.badge

import kotlinx.coroutines.flow.Flow

/**
 * Something that reads staff badges and hands over the raw token of each scan (plan L5-D2): the typed characters
 * of a USB wedge reader, or the tag id of the built-in NFC reader as uppercase hex. The engine normalizes and
 * hashes the token; a source never interprets it.
 */
interface BadgeSource {
    /** The tokens read while the source is started; hot, nothing is replayed to a late collector. */
    val scans: Flow<String>

    /** Begins reading (the setting picked this source). */
    fun start()

    /** Stops reading and lets go of everything it holds. */
    fun stop()

    /** New wedge timing; a source that has none ignores it. */
    fun configure(config: WedgeConfig) {}
}

/** The two sources the setting can pick between: `usb` (keyboard wedge) and `nfc` (built-in reader). */
class BadgeSources(
    val usb: BadgeSource,
    val nfc: BadgeSource,
)
