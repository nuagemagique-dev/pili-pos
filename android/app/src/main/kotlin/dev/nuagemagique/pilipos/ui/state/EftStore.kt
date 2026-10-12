package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.EftPayment
import javax.inject.Inject
import javax.inject.Singleton

/** The status of a payment the terminal has not answered for yet (`sent`), or answered nothing sure about (`unknown`). */
internal fun EftPayment.isOpen(): Boolean = status == EFT_SENT || status == EFT_UNKNOWN

/** An approved payment no sale has settled yet: the sale must use it. */
internal fun EftPayment.isUsable(): Boolean = status == EFT_APPROVED && !used

internal const val EFT_SENT = "sent"
internal const val EFT_UNKNOWN = "unknown"
internal const val EFT_APPROVED = "approved"
internal const val EFT_DECLINED = "declined"
internal const val EFT_CANCELLED = "cancelled"

/** What the blocking overlay of a terminal session is doing. */
enum class EftPhase {
    /** No overlay. */
    IDLE,

    /** The terminal waits for the card. */
    WAITING,

    /** `eftCheck` runs. */
    CHECKING,

    /** The overlay shows the answer, or why nothing was charged. */
    RESULT,
}

/**
 * What the overlay of the linked card terminal shows: one terminal session at a time.
 *
 * [part] and [parts] say which part of a split sale is being paid ("2 of 3"); [part] is null when the payment is not
 * one part of a split.
 */
data class EftOverlayState(
    val phase: EftPhase = EftPhase.IDLE,
    val amountCents: Long = 0L,
    /** The payment the overlay is about (its last answer); null before the first answer or when nothing was charged. */
    val payment: EftPayment? = null,
    /** Why nothing was charged, or why the check failed (readable text). */
    val failure: String? = null,
    val abortRequested: Boolean = false,
    val part: Int? = null,
    val parts: Int = 1,
) {
    /** True while the overlay covers the screen. */
    val visible: Boolean get() = phase != EftPhase.IDLE

    /** True while a spinner shows (the terminal works or the check runs). */
    val busy: Boolean get() = phase == EftPhase.WAITING || phase == EftPhase.CHECKING

    /** The payment waits for the terminal's answer: the sale is blocked until it is checked. */
    val unknown: Boolean get() = phase == EftPhase.RESULT && payment?.isOpen() == true
}

/**
 * The linked card payment terminal, as the screens use it (port of `ui/src/stores/eft.ts`).
 *
 * The engine rules (`crates/pos-engine/src/commands/eft.rs`): a payment is stored before the terminal is called; an
 * approved payment no sale has settled MUST be used by the sale of its order (exact amount, its own card part); a
 * payment waiting for its answer blocks the sale until [check] gets the terminal's answer. This store holds the
 * payments of the order on screen and drives the blocking overlay of one session at a time:
 *
 *  - [charge] asks the terminal for an amount and waits for the card,
 *  - [verify] asks the terminal where a payment stands.
 *
 * Both answer the approved payment, or null (declined, cancelled, refused, or the operator closed the overlay: the
 * payment, if it still waits for its answer, stays in [payments]). The engine call runs in the application scope: a
 * screen that goes away never aborts a payment (the operator uses [abort]); the answer is stored by the engine and
 * found again by [load].
 */
@Suppress("TooManyFunctions")
@Singleton
class EftStore
    @Inject
    constructor(
        private val gateway: EngineGateway,
        private val errors: ErrorPresenter,
        private val settings: PosSettingsStore,
        @ApplicationScope private val scope: CoroutineScope,
    ) {
        private val mutableOverlay = MutableStateFlow(EftOverlayState())
        private val mutablePayments = MutableStateFlow<List<EftPayment>>(emptyList())

        @Volatile
        private var finish: CompletableDeferred<EftPayment?>? = null

        /** The overlay of the current session; [EftOverlayState.visible] is false when there is none. */
        val overlay: StateFlow<EftOverlayState> = mutableOverlay.asStateFlow()

        /** The order's approved unused payments and those awaiting an answer, oldest first. */
        val payments: StateFlow<List<EftPayment>> = mutablePayments.asStateFlow()

        /** The kind of the linked terminal as in `pos.json`, or null for a till whose cards are typed by hand. */
        fun kind(): String? = settings.settings.value?.paymentTerminal

        /** A terminal is linked (the installation's setting). */
        fun linked(): Boolean = kind() != null

        /** Card parts go to the terminal: linked, and not in training (the engine refuses `eftPay` there). */
        fun active(): Boolean = linked() && !settings.trainingMode.value

        /** The payments still waiting for the terminal's answer. */
        fun open(): List<EftPayment> = payments.value.filter { it.isOpen() }

        /** The approved payments no sale has settled. */
        fun approved(): List<EftPayment> = payments.value.filter { it.isUsable() }

        private fun record(payment: EftPayment) {
            mutablePayments.update { list ->
                val others = list.filter { it.id != payment.id }
                val keep = payment.isOpen() || payment.isUsable()
                (if (keep) others + payment else others).sortedBy { it.id }
            }
        }

        /** Reads the order's payments from the engine (also what a crash or a restart left); a failure shows a toast. */
        @Suppress("TooGenericExceptionCaught")
        suspend fun load(orderId: Long) {
            try {
                mutablePayments.value = gateway.call { engine -> engine.listEftPayments(orderId) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                mutablePayments.value = emptyList()
                errors.show(e, settings.language.value)
            }
        }

        /** Forgets the payments of the order that was on screen. */
        fun clear() {
            mutablePayments.value = emptyList()
        }

        private fun close(result: EftPayment?) {
            val done = finish
            finish = null
            mutableOverlay.value = EftOverlayState()
            done?.complete(result)
        }

        /** The terminal answered [payment]: an approval closes the session, anything else is shown. */
        private fun answered(payment: EftPayment) {
            record(payment)
            mutableOverlay.update { it.copy(payment = payment, failure = null) }
            if (payment.status == EFT_APPROVED) {
                close(payment)
            } else {
                mutableOverlay.update { it.copy(phase = EftPhase.RESULT) }
            }
        }

        private fun begin(state: EftOverlayState): CompletableDeferred<EftPayment?> {
            // A session still open belongs to a caller that is gone: it ends without a payment.
            finish?.complete(null)
            val done = CompletableDeferred<EftPayment?>()
            finish = done
            mutableOverlay.value = state
            return done
        }

        /** Waits for the end of the session [done]; a caller that is cancelled takes its overlay down with it. */
        private suspend fun await(done: CompletableDeferred<EftPayment?>): EftPayment? =
            try {
                done.await()
            } catch (e: CancellationException) {
                if (finish === done) close(null)
                throw e
            }

        /**
         * Charges [cents] of the order [orderId] on the terminal and waits for the card. [part] of [parts] is shown
         * on the overlay when the sale is split.
         */
        suspend fun charge(
            orderId: Long,
            cents: Long,
            part: Int? = null,
            parts: Int = 1,
        ): EftPayment? {
            val done = begin(EftOverlayState(phase = EftPhase.WAITING, amountCents = cents, part = part, parts = parts))
            scope.launch { runCharge(done, orderId, cents) }
            return await(done)
        }

        /** Asks the terminal where [payment] (sent or unknown) stands. */
        suspend fun verify(payment: EftPayment): EftPayment? {
            val done = begin(EftOverlayState(phase = EftPhase.CHECKING, amountCents = payment.amountCents, payment = payment))
            scope.launch { runCheck(done, payment) }
            return await(done)
        }

        /** The "Verifier" button of the overlay: the payment it shows is unknown. */
        fun check() {
            val done = finish ?: return
            val payment = mutableOverlay.value.payment ?: return
            mutableOverlay.update { it.copy(phase = EftPhase.CHECKING, failure = null) }
            scope.launch { runCheck(done, payment) }
        }

        /**
         * A request to the terminal, not a guarantee: the pending [charge] then ends cancelled, or approved when the
         * abort came too late.
         */
        @Suppress("TooGenericExceptionCaught")
        fun abort() {
            if (mutableOverlay.value.abortRequested) return
            mutableOverlay.update { it.copy(abortRequested = true) }
            scope.launch {
                try {
                    gateway.call { engine -> engine.eftAbort() }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    mutableOverlay.update { it.copy(abortRequested = false) }
                    errors.show(e, settings.language.value)
                }
            }
        }

        /** The "OK" or "Fermer" button of the overlay. */
        fun dismiss() = close(null)

        @Suppress("TooGenericExceptionCaught")
        private suspend fun runCharge(
            done: CompletableDeferred<EftPayment?>,
            orderId: Long,
            cents: Long,
        ) {
            try {
                val payment = gateway.call { engine -> engine.eftPay(orderId, cents) }
                if (finish === done) answered(payment) else record(payment)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // Nothing was charged, or the request was refused first.
                Timber.w(e, "Terminal payment refused")
                if (finish === done) failed(e)
            }
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun runCheck(
            done: CompletableDeferred<EftPayment?>,
            payment: EftPayment,
        ) {
            try {
                val answer = gateway.call { engine -> engine.eftCheck(payment.id) }
                if (finish === done) answered(answer) else record(answer)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.w(e, "Terminal payment check failed")
                if (finish === done) failed(e)
            }
        }

        private fun failed(error: Throwable) {
            val text = errors.text(error, settings.language.value)
            mutableOverlay.update { it.copy(phase = EftPhase.RESULT, failure = text) }
        }
    }
