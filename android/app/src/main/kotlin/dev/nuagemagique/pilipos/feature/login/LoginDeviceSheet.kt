package dev.nuagemagique.pilipos.feature.login

import androidx.compose.runtime.Composable
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.fdm.rememberFdmAlertText
import dev.nuagemagique.pilipos.ui.scaffold.DeviceStatus
import dev.nuagemagique.pilipos.ui.scaffold.DeviceStatusSheet
import dev.nuagemagique.pilipos.ui.scaffold.LightState
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText

/** The colour of a start-up check as a header light (the two share the status sheet). */
internal fun CheckState.light(): LightState =
    when (this) {
        CheckState.Ok -> LightState.OK
        CheckState.Warn -> LightState.WARN
        CheckState.Err -> LightState.ERR
        CheckState.None -> LightState.NONE
    }

/**
 * The status sheet of the start-up check the operator tapped (UX audit L2, the same sheet as the header lights): the
 * device, its state in words, the reason, the time of the last check and "Réessayer maintenant", which runs that
 * check again. Nothing is drawn while no check is open.
 */
@Composable
internal fun LoginDeviceSheet(
    state: LoginUiState,
    callbacks: DeviceCallbacks,
) {
    val id = state.deviceSheet ?: return
    val check = state.checks.firstOrNull { it.id == id } ?: return
    val name =
        when (id) {
            CheckId.Fdm -> uiText(R.string.ui_login_check_fdm)
            CheckId.Hubrise -> uiText(R.string.ui_login_check_hubrise)
            else -> vueText(requireNotNull(id.labelKey))
        }
    DeviceStatusSheet(
        device = DeviceStatus(id.name.lowercase(), name, check.state.light(), checkReason(check, state.fdmCode), state.checkedAt[id]),
        busy = state.deviceBusy,
        onRetry = callbacks.onRetry,
        onClose = callbacks.onClose,
    )
}

/** Why the device of [check] is in its state, in words (the reasons of the Vue start-up rows). */
@Composable
private fun checkReason(
    check: StartupCheck,
    fdmCode: String?,
): String {
    val fdmMessage = if (check.id == CheckId.Fdm) rememberFdmAlertText(fdmCode).message?.french else null
    return when {
        check.state == CheckState.Ok -> vueText("status_reason_ok")
        fdmMessage != null && fdmCode != null -> fdmMessage
        check.state == CheckState.None -> vueText("status_reason_none")
        check.id == CheckId.Fdm -> vueText("status_reason_fdm_offline")
        check.id == CheckId.Terminal -> listOfNotNull(vueText(check.textKey), check.detail).joinToString(": ")
        check.id == CheckId.Hubrise -> vueText("status_reason_hubrise_warn")
        check.id == CheckId.Mail -> vueText("status_reason_mail_warn")
        else -> vueText("status_reason_printer_err")
    }
}
