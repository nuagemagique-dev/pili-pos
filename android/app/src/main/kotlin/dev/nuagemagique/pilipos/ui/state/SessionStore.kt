package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.LockOutcome
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PresenceFailed
import uniffi.pos_ffi.SessionLocked
import uniffi.pos_ffi.SocialResult
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole
import java.util.concurrent.atomic.AtomicBoolean
import javax.inject.Inject
import javax.inject.Singleton

/** The answer of `end_shift`: whether the clock-out (S event) was registered. */
typealias EndShiftResult = SocialResult

/**
 * Who is logged in (port of `ui/src/stores/auth.ts`).
 *
 * The engine owns the session; this keeps the [User] it answered so the screens can show the operator and hide
 * what the role may not do (the backend enforces it). No toast is shown here: the screens collect
 * [presenceFailures] and [sessionLocked] and show them.
 */
@Singleton
class SessionStore
    @Inject
    constructor(
        private val gateway: EngineGateway,
        events: EngineEvents,
        @ApplicationScope scope: CoroutineScope,
    ) {
        private val mutableUser = MutableStateFlow<User?>(null)
        private val mutableManager = MutableStateFlow(false)
        private val mutablePayment = MutableStateFlow(false)
        private val mutableLocked =
            MutableSharedFlow<SessionLocked>(extraBufferCapacity = EVENT_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)
        private val mutablePresence =
            MutableSharedFlow<PresenceFailed>(extraBufferCapacity = EVENT_BUFFER, onBufferOverflow = BufferOverflow.DROP_OLDEST)

        // True while lock() runs: the session-locked event it causes is handled by the caller, not re-emitted.
        private val locking = AtomicBoolean(false)

        /** The logged-in operator; null on the login screen. */
        val currentUser: StateFlow<User?> = mutableUser.asStateFlow()

        /** True for an administrator. */
        val isManager: StateFlow<Boolean> = mutableManager.asStateFlow()

        /** True for the roles that may take a payment (administrator, cashier). */
        val canTakePayment: StateFlow<Boolean> = mutablePayment.asStateFlow()

        /** The clock-in queued by a login failed; never blocks anything. */
        val presenceFailures: SharedFlow<PresenceFailed> = mutablePresence.asSharedFlow()

        /** The engine locked the session by itself (idle): the user is already cleared, the caller leaves the screen. */
        val sessionLocked: SharedFlow<SessionLocked> = mutableLocked.asSharedFlow()

        init {
            // Subscribed before any call (no replay): UNDISPATCHED registers the collector right here.
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event ->
                    when (event) {
                        is PosEvent.PresenceFailed -> mutablePresence.tryEmit(event.failure)
                        is PosEvent.SessionLocked -> onSessionLocked(event.locked)
                        else -> Unit
                    }
                }
            }
        }

        /** Logs in with [insz] and [pin] (null for an operator without a PIN). A refusal propagates for the PIN pad. */
        suspend fun login(
            insz: String,
            pin: String?,
        ): User = remember(gateway.call { engine -> engine.login(insz, pin) })

        /** First login with a PIN chosen now (a manager must have one). */
        suspend fun loginWithNewPin(
            insz: String,
            pin: String,
        ): User = remember(gateway.call { engine -> engine.loginWithNewPin(insz, pin) })

        /**
         * Locks the screen through the engine: it registers the open order (it needs the open session to sign the
         * P), ends the session and keeps the operator on shift. A failed order registration comes back in the
         * outcome and never blocks the lock; any other failure clears the user and tries a logout.
         */
        @Suppress("TooGenericExceptionCaught")
        suspend fun lock(): LockOutcome {
            locking.set(true)
            try {
                val outcome = gateway.call { engine -> engine.lockSession() }
                forget()
                return outcome
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.w(e, "Lock failed, logging out")
                forget()
                try {
                    gateway.call { engine -> engine.logout() }
                } catch (cause: CancellationException) {
                    throw cause
                } catch (cause: Exception) {
                    // The next login replaces the session anyway.
                    Timber.d(cause, "Logout after a failed lock failed")
                }
                return LockOutcome(orderError = null)
            } finally {
                locking.set(false)
            }
        }

        /**
         * Ends the shift of [insz] from the login screen: the engine checks the PIN (a refusal is thrown for the PIN
         * pad), signs the clock-out and always ends the session. The user is cleared whatever the result says.
         */
        suspend fun endShift(
            insz: String,
            pin: String?,
        ): EndShiftResult {
            val result = gateway.call { engine -> engine.endShift(insz, pin) }
            forget()
            return result
        }

        /** Ends the session; the user is cleared even when the engine refuses. */
        suspend fun logout() {
            try {
                gateway.call { engine -> engine.logout() }
            } finally {
                forget()
            }
        }

        private fun onSessionLocked(locked: SessionLocked) {
            if (locking.get() || mutableUser.value == null) return
            forget()
            mutableLocked.tryEmit(locked)
        }

        private fun remember(user: User): User {
            mutableUser.value = user
            mutableManager.value = user.role == UserRole.ADMIN
            mutablePayment.value = user.role == UserRole.ADMIN || user.role == UserRole.CASHIER
            return user
        }

        private fun forget() {
            mutableUser.value = null
            mutableManager.value = false
            mutablePayment.value = false
        }

        private companion object {
            const val EVENT_BUFFER = 8
        }
    }
