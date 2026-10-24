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

/** One non-blocking message of the operator feedback stack. */
data class Toast(
    val id: Long,
    val message: String,
    val variant: ToastVariant,
)

/** Default lifetime of a toast, in milliseconds (`DEFAULT_TTL_MS` of the Vue toast store). */
const val DEFAULT_TOAST_TTL_MS: Long = 3000L

/**
 * `stores/toast.ts`: the queue of non-blocking operator feedback (UX guide 8.2, auto-dismiss).
 *
 * Each toast removes itself after its own time to live; [dismiss] removes one earlier (a tap). Safe to call from
 * any thread.
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

        /** Appends a toast, schedules its removal after [ttlMs] and returns its id. */
        fun show(
            message: String,
            variant: ToastVariant = ToastVariant.Info,
            ttlMs: Long = DEFAULT_TOAST_TTL_MS,
        ): Long {
            val id = nextId.getAndIncrement()
            state.update { it + Toast(id, message, variant) }
            scope.launch {
                delay(ttlMs)
                dismiss(id)
            }
            return id
        }

        /** Removes the toast [id]; nothing happens when it is already gone. */
        fun dismiss(id: Long) {
            state.update { list -> list.filterNot { it.id == id } }
        }
    }
