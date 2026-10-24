package dev.nuagemagique.pilipos.ui.components

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/**
 * The PIN dialog of `PinDialog.vue`: a [PiliDialog] around a [PinPad] that holds its own [PinPadState].
 *
 * In [PinMode.Verify] [onSubmit] runs on the 4th digit; in [PinMode.Choose] it runs after a matching
 * confirmation (a mismatch shows `texts.mismatch` and starts over at Enter). It runs in a coroutine scope of the
 * composition with the pad busy (keys ignored, never submitted twice). On success [onDone] is called; on a
 * throwable the error line shows `describeError(it)` and the pad is cleared (Verify) or restarts at Enter
 * (Choose). A tap on the scrim or the cancel button calls [onCancel].
 *
 * What the caller wires (all texts and the engine call are the caller's, the dialog never looks a key up).
 * Vue keys of `PinDialog.vue`: `pin_enter` (title in Verify and on a normal change, prompt), `pin_choose`
 * (title and prompt of a first login), `pin_confirm` (prompt of the second entry), `pin_mismatch`,
 * `pin_first_login_hint` (hint of a first login, or the caller's own hint), `pin_change` (accessible name),
 * `pin_chosen` and `pin_changed` (toasts the caller raises after [onDone]), `back` (cancel); the error text is
 * produced by `ErrorTexts.describe` through [describeError].
 */
@Composable
fun PinDialog(
    mode: PinMode,
    texts: PinPadTexts,
    onSubmit: suspend (pin: String) -> Unit,
    describeError: (Throwable) -> String,
    onDone: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val state = remember(mode, texts.mismatch) { PinPadState(mode, texts.mismatch) }
    val scope = rememberCoroutineScope()
    val currentSubmit by rememberUpdatedState(onSubmit)
    val currentDescribe by rememberUpdatedState(describeError)
    val currentDone by rememberUpdatedState(onDone)
    PiliDialog(title = texts.title, onDismissRequest = onCancel, modifier = modifier) {
        PinPad(
            state = state,
            texts = texts,
            onKey = { key ->
                val pin = state.apply(key)
                if (pin != null) {
                    scope.launch {
                        val failure = runSubmit { currentSubmit(pin) }
                        if (failure == null) {
                            state.succeeded()
                            currentDone()
                        } else {
                            state.failed(currentDescribe(failure))
                        }
                    }
                }
            },
            onCancel = onCancel,
            showTitle = false,
        )
    }
}

/** Runs [block]; returns what it threw (cancellation is rethrown), or null on success. */
@Suppress("TooGenericExceptionCaught")
private suspend fun runSubmit(block: suspend () -> Unit): Throwable? =
    try {
        block()
        null
    } catch (e: CancellationException) {
        throw e
    } catch (e: Throwable) {
        e
    }
