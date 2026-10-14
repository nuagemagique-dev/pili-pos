package dev.nuagemagique.pilipos.ui.fdm

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.FdmHealth
import uniffi.pos_ffi.FdmMessageDisplay
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.MessageItem
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.SignMessages
import javax.inject.Inject
import javax.inject.Singleton

/** Lifetime of the toast of an FDM message attached to a signed event, in milliseconds (`SIGN_MESSAGE_TTL_MS`). */
const val SIGN_MESSAGE_TOAST_MS: Long = 8000L

/**
 * The FDM health (port of `ui/src/stores/fdm.ts`).
 *
 * The engine monitors the FDM and classifies what it finds; this keeps the last reading ([state]) for the banner,
 * the blocking veil and the login checks, and shows the messages the FDM attached to a signed event as toasts.
 * It loads when the engine turns ready and again on a resync, and follows the `FdmStatus` events.
 */
@Singleton
class FdmStore
    @Inject
    @Suppress("LongParameterList") // Hilt constructor: the engine, its events, the three feedback services, the scope.
    constructor(
        private val gateway: EngineGateway,
        events: EngineEvents,
        private val errors: ErrorPresenter,
        private val toasts: ToastStore,
        private val fdmTexts: FdmTexts,
        private val settings: PosSettingsStore,
        @ApplicationScope scope: CoroutineScope,
    ) {
        private val mutableState = MutableStateFlow(FdmUiState())

        // POS description v2.2, Message: a warning is shown at least once per bookingDate; each code once per day.
        private val shownSignMessages = mutableSetOf<String>()

        /** The last reading of the FDM. */
        val state: StateFlow<FdmUiState> = mutableState.asStateFlow()

        init {
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event ->
                    when (event) {
                        is PosEvent.Resync -> load()
                        is PosEvent.FdmStatus -> apply(event.health)
                        is PosEvent.FdmSignMessages -> showSignMessages(event.messages)
                        else -> Unit
                    }
                }
            }
            scope.launch {
                gateway.ready.collect { ready -> if (ready) load() }
            }
        }

        /** Takes the last reading the engine has; no reading, or a failure, keeps the state as it is. */
        @Suppress("TooGenericExceptionCaught")
        suspend fun load() {
            try {
                gateway.call { engine -> engine.fdmHealth() }?.let(::apply)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // No engine to tell: the next event will.
                Timber.d(e, "FDM health not loaded")
            }
        }

        /**
         * Asks the engine to check the FDM now. True when the FDM answered, even with an error; false when it did
         * not (transport failure), when there is no engine, or when the command itself failed (the error is shown, the
         * state is kept).
         */
        suspend fun checkNow(): Boolean {
            mutableState.update { it.copy(checking = true) }
            try {
                return attemptCheck()
            } finally {
                mutableState.update { it.copy(checking = false) }
            }
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun attemptCheck(): Boolean =
            try {
                val health = gateway.call { engine -> engine.fdmCheckNow() }
                apply(health)
                health.answered
            } catch (e: CancellationException) {
                throw e
            } catch (e: EngineUnavailableException) {
                // No engine, nothing to check: not a failed command, so no error toast (the shell says why).
                Timber.d(e, "FDM check without an engine")
                false
            } catch (e: Exception) {
                Timber.w(e, "FDM check failed")
                errors.show(e, settings.language.value)
                false
            }

        /** Takes the engine's reading of the FDM; it classified it already. */
        private fun apply(health: FdmHealth) {
            mutableState.update {
                it.copy(
                    connected = health.connected,
                    severity = health.severity,
                    code = health.code,
                    mandatory = health.mandatory,
                    nextCheckAt = health.nextCheckAt?.let(PosDateTime::parseTimestamp),
                )
            }
        }

        private fun showSignMessages(signed: SignMessages) {
            showGroup(signed.bookingDate, signed.warnings, ToastVariant.Warning)
            showGroup(signed.bookingDate, signed.informations, ToastVariant.Info)
        }

        private fun showGroup(
            bookingDate: String?,
            items: List<MessageItem>,
            variant: ToastVariant,
        ) {
            for (item in items) {
                val code = item.extensions.code.name
                if (!shownSignMessages.add("${bookingDate.orEmpty()}|$code")) continue
                toasts.show(textOf(code, item), variant, SIGN_MESSAGE_TOAST_MS)
                // The header indicator shows a warning too, until the next status. A MANDATORY one keeps its banner
                // open, as the monitor's reading of the same code would (a sale can bring it before the next check).
                if (variant == ToastVariant.Warning) {
                    val keep = item.extensions.showPos == FdmMessageDisplay.MANDATORY
                    mutableState.update {
                        when {
                            it.severity == FdmSeverity.INFO ->
                                it.copy(severity = FdmSeverity.WARNING, code = code, mandatory = keep)
                            it.severity == FdmSeverity.WARNING && keep && !it.mandatory ->
                                it.copy(code = code, mandatory = true)
                            else -> it
                        }
                    }
                }
            }
        }

        /** `ERROR_MESSAGES[code] ?? item.message ?? code`: the French text of a known code, else the FDM's own text. */
        private fun textOf(
            code: String,
            item: MessageItem,
        ): String {
            val known = fdmTexts.fdmCode(code) == code
            return (if (known) fdmTexts.translate(code) else null) ?: item.message.ifEmpty { code }
        }
    }
