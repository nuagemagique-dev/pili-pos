package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest

private val OptionHeight = 64.dp

/** The captions of a [TicketMediumPicker], resolved by the caller. */
@Immutable
data class TicketMediumPickerTexts(
    val paper: String,
    val email: String,
    val both: String,
    val unreachableNote: String,
)

/** The Vue captions (`delivery_paper`, `delivery_email`, `delivery_both`, `delivery_unreachable_note`). Needs `ProvideUiText` above. */
@Composable
fun defaultTicketMediumPickerTexts(): TicketMediumPickerTexts =
    TicketMediumPickerTexts(
        paper = vueText("delivery_paper"),
        email = vueText("delivery_email"),
        both = vueText("delivery_both"),
        unreachableNote = vueText("delivery_unreachable_note"),
    )

/**
 * Which e-mail medium the address keypad is open for, shared by the [TicketMediumPicker] (which opens it) and
 * the [TicketMediumKeypad] (which draws it). The keypad is a dialog and a dialog is drawn in the screen's own
 * overlay, not inside the picker's small box (PD214), so the two halves meet in this holder.
 */
@Stable
class TicketMediumPickerState(
    initial: RequestedMedium? = null,
) {
    /** The e-mail medium the keypad is open for; null while it is closed. */
    var pendingMedium: RequestedMedium? by mutableStateOf(initial)
        private set

    /** Opens the keypad for [medium], [RequestedMedium.DIGITAL] or [RequestedMedium.PAPER_DIGITAL]. */
    fun open(medium: RequestedMedium) {
        require(medium != RequestedMedium.PAPER) { "the keypad is for an e-mail medium" }
        pendingMedium = medium
    }

    /** Closes the keypad. */
    fun close() {
        pendingMedium = null
    }

    internal companion object {
        val Saver: Saver<TicketMediumPickerState, String> =
            Saver(
                save = { it.pendingMedium?.name ?: "" },
                restore = { name -> TicketMediumPickerState(RequestedMedium.entries.firstOrNull { it.name == name }) },
            )
    }
}

/** A [TicketMediumPickerState] that survives a configuration change. */
@Composable
fun rememberTicketMediumPickerState(): TicketMediumPickerState =
    rememberSaveable(saver = TicketMediumPickerState.Saver) { TicketMediumPickerState() }

/**
 * True when e-mail only is offered: the till allows it and the mail service answered the last probe (else the
 * engine would downgrade it to paper plus e-mail anyway).
 */
fun MailStatus.offersDigitalOnly(): Boolean = allowDigitalOnly && reachable

/** True when the note "mail unreachable" is shown: e-mail is set up but the last probe failed. */
fun MailStatus.showsUnreachableNote(): Boolean = configured && !reachable

/**
 * `TicketMediumPicker.vue`: the touch options for how the customer gets the ticket: paper, paper plus e-mail
 * ("both") and, when [offersDigitalOnly], e-mail only. Hidden entirely (nothing drawn) while [mailStatus] is
 * null or says the e-mail setup is not configured: paper stays the silent default.
 *
 * The component holds no delivery state: [request] says which option is active, [onPaper] chooses paper. The
 * two e-mail options only open the address keypad through [state]; the keypad itself is [TicketMediumKeypad],
 * which the screen draws in its overlay, and which calls back with the address.
 *
 * Test tags: `ticket_medium_picker` (root), `medium_paper`, `medium_digital`, `medium_both`, `medium_note`.
 */
@Composable
fun TicketMediumPicker(
    mailStatus: MailStatus?,
    request: TicketDeliveryRequest,
    state: TicketMediumPickerState,
    onPaper: () -> Unit,
    texts: TicketMediumPickerTexts,
    modifier: Modifier = Modifier,
) {
    if (mailStatus == null || !mailStatus.configured) return
    Column(
        modifier = modifier.fillMaxWidth().testTag("ticket_medium_picker"),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            MediumOption(texts.paper, request.medium == RequestedMedium.PAPER, "medium_paper", onPaper)
            if (mailStatus.offersDigitalOnly()) {
                MediumOption(texts.email, request.medium == RequestedMedium.DIGITAL, "medium_digital") {
                    state.open(RequestedMedium.DIGITAL)
                }
            }
            MediumOption(texts.both, request.medium == RequestedMedium.PAPER_DIGITAL, "medium_both") {
                state.open(RequestedMedium.PAPER_DIGITAL)
            }
        }
        if (mailStatus.showsUnreachableNote()) {
            Text(
                text = texts.unreachableNote,
                style = PiliTheme.typography.label,
                color = PiliTheme.colors.accentWarning,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth().testTag("medium_note"),
            )
        }
    }
}

@Composable
private fun RowScope.MediumOption(
    text: String,
    active: Boolean,
    tag: String,
    onClick: () -> Unit,
) {
    PiliButton(
        text = text,
        onClick = onClick,
        variant = if (active) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
        modifier = Modifier.weight(1f).height(OptionHeight).testTag(tag),
    )
}

/**
 * The address keypad of a [TicketMediumPicker], as a dialog: nothing while [state] is closed, else an
 * [EmailKeypad] over the whole box. Confirming gives [onEmail] the medium that was asked for and the address,
 * then closes. Cancelling returns to paper through [onPaper] (cert UX: never a half-picked e-mail medium), then
 * closes. Put it last in the screen's overlay `Box`, like every dialog.
 */
@Composable
fun TicketMediumKeypad(
    state: TicketMediumPickerState,
    texts: EmailKeypadTexts,
    onEmail: (medium: RequestedMedium, email: String) -> Unit,
    onPaper: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val medium = state.pendingMedium ?: return
    Box(modifier = modifier.fillMaxSize()) {
        EmailKeypad(
            texts = texts,
            onConfirm = { email ->
                onEmail(medium, email)
                state.close()
            },
            onCancel = {
                onPaper()
                state.close()
            },
        )
    }
}
