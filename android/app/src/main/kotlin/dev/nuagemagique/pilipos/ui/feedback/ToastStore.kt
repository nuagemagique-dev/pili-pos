package dev.nuagemagique.pilipos.ui.feedback

import dev.nuagemagique.pilipos.di.ApplicationScope
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicLong
import javax.inject.Inject
import javax.inject.Singleton

/** The kind of a [Toast]; it picks the accent colour (`toast-info`, `toast-success`, ... of ToastStack.vue). */
enum class ToastVariant {
    Info,
    Success,
    Warning,
    Error,
}

/** The one button of a toast (UX audit C8: "Annuler" after a table move); [run] is started once, by [ToastStore.runAction]. */
data class ToastAction(
    val label: String,
    val run: suspend () -> Unit,
)

/** One non-blocking message of the operator feedback stack. */
data class Toast(
    val id: Long,
    val message: String,
    val variant: ToastVariant,
    val action: ToastAction? = null,
)

/** Default lifetime of a toast, in milliseconds (`DEFAULT_TTL_MS` of the Vue toast store). */
const val DEFAULT_TOAST_TTL_MS: Long = 3000L

/** At most this many toasts are drawn at once, the newest ones (`MAX_VISIBLE_TOASTS` of the Vue toast store). */
const val MAX_VISIBLE_TOASTS: Int = 3

/**
 * `pickVisible` of the Vue toast store: the toasts to draw, oldest first, at most [max]. Error toasts come first in
 * the choice (an error waits for a tap, so newer messages must not push it out of view and let it come back
 * unprompted later), then the newest of the others fill the remaining places.
 */
fun pickVisibleToasts(
    toasts: List<Toast>,
    max: Int = MAX_VISIBLE_TOASTS,
): List<Toast> {
    val errors = toasts.filter { it.variant == ToastVariant.Error }.takeLast(max)
    val room = max - errors.size
    val others = if (room > 0) toasts.filter { it.variant != ToastVariant.Error }.takeLast(room) else emptyList()
    val chosen = (errors + others).map { it.id }.toSet()
    return toasts.filter { it.id in chosen }
}

/**
 * `stores/toast.ts`: the queue of non-blocking operator feedback (UX guide 8.2, auto-dismiss).
 *
 * Info, success and warning toasts remove themselves after their own time to live. An [ToastVariant.Error] toast
 * never does, whatever time to live it was given: it stays until [dismiss] (a tap on it), so a failure cannot
 * vanish while the operator looks elsewhere (UX audit C5). Safe to call from any thread.
 */
@Singleton
class ToastStore
    @Inject
    constructor(
        @ApplicationScope private val scope: CoroutineScope,
    ) {
        private val nextId = AtomicLong(1L)
        private val state = MutableStateFlow<List<Toast>>(emptyList())

        /** The visible toasts, oldest first. */
        val toasts: StateFlow<List<Toast>> = state.asStateFlow()

        /** Appends a toast, schedules its removal after [ttlMs] (never for an error) and returns its id. */
        fun show(
            message: String,
            variant: ToastVariant = ToastVariant.Info,
            ttlMs: Long = DEFAULT_TOAST_TTL_MS,
        ): Long = push(Toast(nextId.getAndIncrement(), message, variant), ttlMs)

        /** A success toast with one button ([action]); it goes away after [ttlMs] without running the action. */
        fun showWithAction(
            message: String,
            action: ToastAction,
            ttlMs: Long = DEFAULT_TOAST_TTL_MS,
        ): Long = push(Toast(nextId.getAndIncrement(), message, ToastVariant.Success, action), ttlMs)

        /** The tap on the button of the toast [id]: the toast goes first, then its action runs once, on the app scope. */
        fun runAction(id: Long) {
            val action = state.value.firstOrNull { it.id == id }?.action
            dismiss(id)
            if (action != null) scope.launch { action.run() }
        }

        private fun push(
            toast: Toast,
            ttlMs: Long,
        ): Long {
            val id = toast.id
            val variant = toast.variant
            state.update { it + toast }
            if (variant != ToastVariant.Error) {
                scope.launch {
                    delay(ttlMs)
                    dismiss(id)
                }
            }
            return id
        }

        /** Removes the toast [id]; nothing happens when it is already gone. */
        fun dismiss(id: Long) {
            state.update { list -> list.filterNot { it.id == id } }
        }
    }
