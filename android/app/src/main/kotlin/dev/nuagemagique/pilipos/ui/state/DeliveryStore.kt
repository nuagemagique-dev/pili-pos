package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import timber.log.Timber
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The operator's ticket-delivery choice: paper, paper and e-mail, or e-mail only (port of
 * `ui/src/stores/delivery.ts`).
 *
 * It is shared by checkout, refund and copy, so that the picker and the keypad hold no business state: only the
 * keypad's typing buffer is local to the screen. [request] is what the next checkout, refund or copy sends to the
 * engine; [mailStatus] says which choices the picker offers.
 */
@Singleton
class DeliveryStore
    @Inject
    constructor(
        private val gateway: EngineGateway,
        private val errors: ErrorPresenter,
        private val settings: PosSettingsStore,
    ) {
        private val mutableMailStatus = MutableStateFlow<MailStatus?>(null)
        private val mutableRequest = MutableStateFlow(paper())

        /** The last mail probe; null before the first refresh or after a failed one (the picker is then hidden). */
        val mailStatus: StateFlow<MailStatus?> = mutableMailStatus.asStateFlow()

        /** The delivery the next ticket asks for; paper until the operator chooses otherwise. */
        val request: StateFlow<TicketDeliveryRequest> = mutableRequest.asStateFlow()

        /**
         * Reads the mail status again. Call it when the picker is about to be shown (the engine caches the probe
         * for 60 seconds, so calling often is cheap). A failure gives a null [mailStatus].
         */
        @Suppress("TooGenericExceptionCaught")
        suspend fun refreshMailStatus() {
            try {
                mutableMailStatus.value = gateway.call { engine -> engine.checkMailStatus() }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.w(e, "Mail status not read")
                mutableMailStatus.value = null
            }
        }

        /** Chooses the paper ticket only. */
        fun setPaper() {
            mutableRequest.value = paper()
        }

        /** Chooses an e-mail delivery to [email]: [RequestedMedium.PAPER_DIGITAL] (both) or [RequestedMedium.DIGITAL]. */
        fun setEmail(
            medium: RequestedMedium,
            email: String,
        ) {
            require(medium != RequestedMedium.PAPER) { "setEmail needs an e-mail medium" }
            mutableRequest.value = TicketDeliveryRequest(medium, email)
        }

        /** Back to paper. Call it after every checkout, refund or copy so the next customer starts from the default. */
        fun reset() {
            mutableRequest.value = paper()
        }

        /**
         * Prints the paper original that an e-mail-only ticket still lacks (the one print action of the done
         * screen; the engine never prints a second original).
         *
         * @return true when the engine printed; false when it refused, after the refusal was shown as a toast
         */
        @Suppress("TooGenericExceptionCaught")
        suspend fun printMissingOriginal(fiscalEventId: Long): Boolean =
            try {
                gateway.call { engine -> engine.printMissingOriginal(fiscalEventId) }
                true
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                errors.show(e, settings.language.value)
                false
            }

        // A fresh record each time: the uniffi record has public setters, so one shared instance could be edited.
        private fun paper() = TicketDeliveryRequest(RequestedMedium.PAPER, null)
    }
