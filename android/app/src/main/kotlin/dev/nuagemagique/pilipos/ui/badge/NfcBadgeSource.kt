package dev.nuagemagique.pilipos.ui.badge

import android.app.Activity
import android.content.Context
import android.content.pm.PackageManager
import android.nfc.NfcAdapter
import android.nfc.Tag
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import javax.inject.Inject
import javax.inject.Singleton

/** What the NFC source needs from the device; a fake in the tests. */
interface NfcSupport {
    /** True when the device reports the NFC feature (`PackageManager.FEATURE_NFC`). */
    val hasNfc: Boolean

    /** True when the NFC adapter is turned on in the device settings. */
    val isEnabled: Boolean

    /** Starts the reader mode on [activity]; [onTagId] gets the id bytes of each tag. */
    fun enableReaderMode(
        activity: Activity,
        onTagId: (ByteArray) -> Unit,
    )

    /** Ends the reader mode started on [activity]. */
    fun disableReaderMode(activity: Activity)
}

/** The real thing: the default [NfcAdapter] of the device. */
@Singleton
class AndroidNfcSupport
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
    ) : NfcSupport {
        override val hasNfc: Boolean
            get() = context.packageManager.hasSystemFeature(PackageManager.FEATURE_NFC)

        override val isEnabled: Boolean
            get() = adapter(context)?.isEnabled == true

        override fun enableReaderMode(
            activity: Activity,
            onTagId: (ByteArray) -> Unit,
        ) {
            adapter(activity)?.enableReaderMode(
                activity,
                { tag: Tag -> onTagId(tag.id) },
                READER_FLAGS,
                null,
            )
        }

        override fun disableReaderMode(activity: Activity) {
            adapter(activity)?.disableReaderMode(activity)
        }

        private fun adapter(from: Context): NfcAdapter? = NfcAdapter.getDefaultAdapter(from)

        private companion object {
            // Every tag technology, and no NDEF check: only the id is read (plan L5-D13).
            const val READER_FLAGS =
                NfcAdapter.FLAG_READER_NFC_A or
                    NfcAdapter.FLAG_READER_NFC_B or
                    NfcAdapter.FLAG_READER_NFC_F or
                    NfcAdapter.FLAG_READER_NFC_V or
                    NfcAdapter.FLAG_READER_SKIP_NDEF_CHECK
        }
    }

/** The tag id bytes as uppercase hex, in the order Android gives them (plan L5-D2). */
fun tagIdToHex(id: ByteArray): String = id.joinToString("") { "%02X".format(it.toInt() and BYTE_MASK) }

private const val BYTE_MASK = 0xFF
private const val SCAN_BUFFER = 4

/**
 * The built-in NFC reader (plan L5-D13). The reader mode runs only while the setting is `nfc` ([start]), the
 * device has NFC, the adapter is turned on and an activity is in the foreground ([attach] from `onResume`,
 * [detach] from `onPause`). Tags are never read for anything but their id.
 */
@Singleton
class NfcBadgeSource
    @Inject
    constructor(
        private val support: NfcSupport,
    ) : BadgeSource {
        private val mutableScans =
            MutableSharedFlow<String>(extraBufferCapacity = SCAN_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private var wanted = false
        private var activity: Activity? = null
        private var reading: Activity? = null

        override val scans: Flow<String> = mutableScans.asSharedFlow()

        /** True while the reader mode is on. */
        val active: Boolean
            get() = reading != null

        override fun start() {
            wanted = true
            sync()
        }

        override fun stop() {
            wanted = false
            sync()
        }

        /** An activity is in the foreground (`onResume`). */
        fun attach(activity: Activity) {
            this.activity = activity
            sync()
        }

        /** The activity left the foreground (`onPause`): the reader mode must end with it. */
        fun detach(activity: Activity) {
            if (this.activity === activity) {
                this.activity = null
                sync()
            }
        }

        private fun sync() {
            val target = activity
            val shouldRead = wanted && target != null && support.hasNfc && support.isEnabled
            val current = reading
            if (current != null && (!shouldRead || current !== target)) {
                support.disableReaderMode(current)
                reading = null
            }
            if (shouldRead && reading == null && target != null) {
                support.enableReaderMode(target) { id -> mutableScans.tryEmit(tagIdToHex(id)) }
                reading = target
            }
        }
    }
