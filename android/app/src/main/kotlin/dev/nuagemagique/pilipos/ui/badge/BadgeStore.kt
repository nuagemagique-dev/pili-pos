package dev.nuagemagique.pilipos.ui.badge

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.BadgeLink
import uniffi.pos_ffi.BadgeOutcome
import uniffi.pos_ffi.BadgeOutcomeKind
import uniffi.pos_ffi.BadgeOverview
import uniffi.pos_ffi.BadgeSettingsInput
import uniffi.pos_ffi.BadgeSettingsView
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.User
import java.time.Instant
import javax.inject.Inject
import javax.inject.Singleton

/** Lifetime of the toast of a registered arrival or departure, in milliseconds (4000 in `stores/badge.ts`). */
private const val REGISTERED_TOAST_MS: Long = 4000L

/** Lifetime of the toast of an FDM refusal of a scan, in milliseconds (6000 in `stores/badge.ts`). */
private const val FAILED_TOAST_MS: Long = 6000L

private const val EVENT_BUFFER = 4

/** The reader values of `badge_settings.reader`. */
const val READER_OFF: String = "off"
const val READER_NFC: String = "nfc"
const val READER_USB: String = "usb"

/** A manager's badge on the login screen: the PIN pad opens for them (plan L5-D6). */
data class PinRequest(
    val insz: String,
    val name: String,
)

/** A pending enrolment capture: the next scan completes [token] instead of being sent to the engine. */
class BadgeCapture(
    val token: Deferred<String>,
    private val onCancel: () -> Unit,
) {
    /** Stops waiting; the next scan is an ordinary scan again. */
    fun cancel() = onCancel()
}

/**
 * The badge reader as the screens see it (port of `ui/src/stores/badge.ts`, plan L5-D6 to L5-D14): the reader
 * setting (read when the engine turns ready, also with no session), the source that follows it, the enrolment
 * capture, the end-of-shift flag of the login screen and what a scan does.
 *
 * `off` starts no source; `usb` starts the keyboard wedge; `nfc` starts the built-in reader only when the device has
 * NFC. The engine owns every rule of a scan: this only shows the outcome (toast, cue) and, for a badge login, opens
 * the session in the [SessionStore] like a PIN login.
 */
@Singleton
@Suppress("LongParameterList", "TooManyFunctions")
class BadgeStore
    @Inject
    constructor(
        private val gateway: EngineGateway,
        events: EngineEvents,
        private val session: SessionStore,
        private val toasts: ToastStore,
        private val errors: ErrorPresenter,
        private val settingsStore: PosSettingsStore,
        private val catalog: MessageCatalog,
        private val cue: BadgeCue,
        private val sources: BadgeSources,
        private val support: NfcSupport,
        @ApplicationScope private val scope: CoroutineScope,
    ) {
        private val mutableSettings = MutableStateFlow<BadgeSettingsView?>(null)
        private val mutableOverview = MutableStateFlow<BadgeOverview?>(null)
        private val mutableEndShift = MutableStateFlow(false)
        private val mutablePresence = MutableStateFlow(0)
        private val mutableCapturing = MutableStateFlow(false)
        private val pinRequestFlow =
            MutableSharedFlow<PinRequest>(extraBufferCapacity = EVENT_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private val loggedInFlow =
            MutableSharedFlow<User>(extraBufferCapacity = EVENT_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)

        @Volatile
        private var capture: CompletableDeferred<String>? = null

        /** The reader setting; null before the first answer of the engine. */
        val settings: StateFlow<BadgeSettingsView?> = mutableSettings.asStateFlow()

        /** The active badges and the log (managers only); null until loaded. */
        val overview: StateFlow<BadgeOverview?> = mutableOverview.asStateFlow()

        /** True while the login screen is in its "Fin de service" mode: a scan clocks its holder out. */
        val endShiftMode: StateFlow<Boolean> = mutableEndShift.asStateFlow()

        /** Bumped after a registered arrival or departure, for the screens that show who is on shift. */
        val presenceVersion: StateFlow<Int> = mutablePresence.asStateFlow()

        /** True while an enrolment waits for a badge. */
        val capturing: StateFlow<Boolean> = mutableCapturing.asStateFlow()

        /** A manager's badge on the login screen; each request is delivered once. */
        val pinRequests: SharedFlow<PinRequest> = pinRequestFlow.asSharedFlow()

        /** A badge login opened this session; the login screen leaves for Tables. Each one is delivered once. */
        val loggedIn: SharedFlow<User> = loggedInFlow.asSharedFlow()

        /** True when the setting is on (`nfc` or `usb`). */
        val readerOn: Boolean
            get() = (mutableSettings.value?.reader ?: READER_OFF) != READER_OFF

        /** True when the device reports the NFC feature: the Settings offer "NFC intégré" only then. */
        val hasNfc: Boolean
            get() = support.hasNfc

        /** True when the NFC adapter is turned on in the device settings. */
        val nfcEnabled: Boolean
            get() = support.isEnabled

        init {
            scope.launch(start = CoroutineStart.UNDISPATCHED) { sources.usb.scans.collect { onScan(it) } }
            scope.launch(start = CoroutineStart.UNDISPATCHED) { sources.nfc.scans.collect { onScan(it) } }
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event -> if (event is PosEvent.Resync) load() }
            }
            scope.launch { gateway.ready.collect { ready -> if (ready) load() } }
        }

        /** True when the reader is on and badge login is enabled. */
        val loginOn: Boolean
            get() = mutableSettings.value?.let { it.reader != READER_OFF && it.loginEnabled } ?: false

        /** Reads the setting from the engine; a failure leaves the reader as it was. */
        @Suppress("TooGenericExceptionCaught")
        suspend fun load() {
            try {
                publish(gateway.call { engine -> engine.getBadgeSettings() })
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.d(e, "Badge settings not loaded")
            }
        }

        /** Saves a new setting (managers; a refusal propagates) and reads the overview again. */
        suspend fun save(input: BadgeSettingsInput) {
            publish(gateway.call { engine -> engine.setBadgeSettings(input) })
            loadOverview()
        }

        /** The active badges and the log; a failure (not a manager, engine away) gives a null overview. */
        @Suppress("TooGenericExceptionCaught", "SwallowedException")
        suspend fun loadOverview() {
            try {
                mutableOverview.value = fetchOverview()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                mutableOverview.value = null
            }
        }

        /** Asks the engine for the badge list without keeping it: a screen that reloads several lists at once. */
        suspend fun fetchOverview(): BadgeOverview = gateway.call { engine -> engine.listBadges() }

        /** Keeps [overview] as the list the screens show. */
        fun publishOverview(overview: BadgeOverview) {
            mutableOverview.value = overview
        }

        /** Links the scanned [token] to [employeeInsz] with the manager's own PIN. A refusal propagates. */
        suspend fun link(
            employeeInsz: String,
            token: String,
            operatorPin: String,
        ): BadgeLink {
            val link = gateway.call { engine -> engine.linkBadge(employeeInsz, token, operatorPin) }
            loadOverview()
            return link
        }

        /** Removes the badge of [employeeInsz]. A refusal propagates. */
        suspend fun revoke(employeeInsz: String) {
            gateway.call { engine -> engine.revokeBadge(employeeInsz) }
            loadOverview()
        }

        /** The login screen is in (or out of) its "Fin de service" mode. */
        fun setEndShiftMode(on: Boolean) {
            mutableEndShift.value = on
        }

        // ── Enrolment capture: the next scan is resolved here, not sent ──

        /** Waits for the next scan and hands its token back (no engine call). */
        fun startCapture(): BadgeCapture {
            cancelCapture()
            val pending = CompletableDeferred<String>()
            capture = pending
            mutableCapturing.value = true
            return BadgeCapture(pending) { if (capture === pending) cancelCapture() }
        }

        private fun cancelCapture() {
            capture = null
            mutableCapturing.value = false
        }

        // ── A scan ──

        /** Handles one scanned [token]: an enrolment capture takes it, otherwise the engine decides. */
        @Suppress("TooGenericExceptionCaught")
        suspend fun onScan(token: String) {
            val waiting = capture
            if (waiting != null) {
                capture = null
                mutableCapturing.value = false
                waiting.complete(token)
                return
            }
            val outcome =
                try {
                    gateway.call { engine -> engine.badgeTap(token, mutableEndShift.value) }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    errors.show(e, language())
                    cue.refused()
                    return
                }
            handle(outcome)
        }

        private fun handle(outcome: BadgeOutcome) {
            val name = outcome.name.orEmpty()
            when (outcome.kind) {
                BadgeOutcomeKind.IGNORED -> Unit
                BadgeOutcomeKind.LOGGED_IN ->
                    outcome.user?.let { user ->
                        session.remember(user)
                        cue.ok()
                        loggedInFlow.tryEmit(user)
                    }
                BadgeOutcomeKind.PIN_REQUIRED -> pinRequestFlow.tryEmit(PinRequest(outcome.insz.orEmpty(), name))
                BadgeOutcomeKind.CLOCK_IN, BadgeOutcomeKind.CLOCK_OUT -> {
                    val key = if (outcome.kind == BadgeOutcomeKind.CLOCK_IN) "badge_clock_in" else "badge_clock_out"
                    val time = PosDateTime.formatTimestamp(outcome.at ?: Instant.now().toString(), PosDateTime::formatTime)
                    toasts.show(text(key, "name" to name, "time" to time), ToastVariant.Success, REGISTERED_TOAST_MS)
                    cue.ok()
                    mutablePresence.update { it + 1 }
                }
                BadgeOutcomeKind.PRESENCE_FAILED -> {
                    val reason = errors.textRaw(outcome.error.orEmpty(), language())
                    toasts.show(text("badge_presence_failed", "name" to name, "error" to reason), ToastVariant.Error, FAILED_TOAST_MS)
                    cue.refused()
                }
            }
        }

        // ── Source selection ──

        private fun publish(view: BadgeSettingsView) {
            mutableSettings.value = view
            applyReader(view)
        }

        /** Starts the source the setting picks and stops the other; `nfc` on a device without NFC reads nothing. */
        private fun applyReader(view: BadgeSettingsView) {
            sources.usb.configure(WedgeConfig(view.wedgeMaxGapMs, view.wedgeMinLen.toInt(), view.wedgeMaxLen.toInt()))
            when (view.reader) {
                READER_USB -> {
                    sources.nfc.stop()
                    sources.usb.start()
                }
                READER_NFC -> {
                    sources.usb.stop()
                    if (support.hasNfc) sources.nfc.start() else sources.nfc.stop()
                }
                else -> {
                    sources.usb.stop()
                    sources.nfc.stop()
                }
            }
        }

        private fun language(): AppLanguage = settingsStore.language.value

        private fun text(
            key: String,
            vararg vars: Pair<String, Any>,
        ): String = catalog.text(key, language(), vars.toMap())
    }
