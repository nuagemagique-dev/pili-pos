package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier

/** A keypad a field asks the [KeypadHost] to draw, with what to do with the text typed. */
sealed interface KeypadRequest {
    /** A text on the [TextKeypad]. */
    data class Text(
        val spec: TextKeypadSpec,
        val onConfirm: (String) -> Unit,
    ) : KeypadRequest

    /** A text on the AZERTY [TextKeyboard]. */
    data class Keyboard(
        val spec: TextKeyboardSpec,
        val onConfirm: (String) -> Unit,
    ) : KeypadRequest

    /** A whole number on the [NumPad]. */
    data class Number(
        val spec: NumberKeypadSpec,
        val onConfirm: (String) -> Unit,
    ) : KeypadRequest
}

/** The keypad being shown, if any; one at a time, like the Vue dialogs. */
@Stable
class KeypadHostState {
    /** The request on screen; null when no keypad is open. */
    var request: KeypadRequest? by mutableStateOf(null)
        private set

    /** Shows [request]. */
    fun open(request: KeypadRequest) {
        this.request = request
    }

    /** Hides the keypad. */
    fun close() {
        request = null
    }
}

/** The host of the keypads below this point. */
val LocalKeypadHost =
    staticCompositionLocalOf<KeypadHostState> {
        error("No KeypadHost: draw the screen inside PosUiRoot or ProvideUiText")
    }

/**
 * Hosts the keypads of the fields below it (C13 / R6: the till never raises the system keyboard). A field sits deep in a
 * scrolling column and a dialog is drawn in the window's layout (PD214), so the field only asks ([KeypadHostState.open])
 * and the host draws the dialog above [content]. The layout of [content] is untouched: the box passes its constraints on.
 */
@Composable
fun ProvideKeypadHost(content: @Composable () -> Unit) {
    val host = remember { KeypadHostState() }
    CompositionLocalProvider(LocalKeypadHost provides host) {
        Box(propagateMinConstraints = true) {
            content()
            val request = host.request
            if (request != null) KeypadLayer(request, host)
        }
    }
}

@Composable
private fun KeypadLayer(
    request: KeypadRequest,
    host: KeypadHostState,
) {
    val done: (String, (String) -> Unit) -> Unit = { text, onConfirm ->
        host.close()
        onConfirm(text)
    }
    when (request) {
        is KeypadRequest.Text -> TextKeypad(request.spec, { done(it, request.onConfirm) }, host::close, Modifier.fillMaxSize())
        is KeypadRequest.Keyboard -> TextKeyboardDialog(request.spec, { done(it, request.onConfirm) }, host::close, Modifier.fillMaxSize())
        is KeypadRequest.Number -> NumberKeypad(request.spec, { done(it, request.onConfirm) }, host::close, Modifier.fillMaxSize())
    }
}
