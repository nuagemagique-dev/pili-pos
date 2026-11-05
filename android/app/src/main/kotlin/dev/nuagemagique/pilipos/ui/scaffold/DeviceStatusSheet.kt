package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.fdm.rememberFdmAlertText
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.launch
import java.time.Instant

// ── The status sheet (UX audit L2) ──────────────────────────────────

/** Which device the status sheet is open for (`fdm`, `ticket`, `kitchen`, `terminal`, `scale`, `hubrise`), or null. */
internal class DeviceSheetState {
    var openKey: String? by mutableStateOf(null)
}

/** What the sheet shows about one device (`DeviceStatus` of `utils/deviceStatus.ts`). */
internal data class DeviceStatus(
    val key: String,
    val name: String,
    val state: LightState,
    /** Why the device is in that state, in words. */
    val reason: String,
    /** When it was last checked; null before the first check. */
    val checkedAt: Instant?,
)

/** Vue key of the state in words ("Fonctionne", "Ne répond pas"...), `stateWordKey` of `utils/deviceStatus.ts`. */
internal fun stateWordKey(state: LightState): String = "status_state_" + state.name.lowercase()

/** The reason of a device that only answers yes or no: ok, unknown, or [errKey] (`plainReason` of Vue). */
@Composable
private fun plainReason(
    state: LightState,
    errKey: String,
): String =
    when (state) {
        LightState.OK -> vueText("status_reason_ok")
        LightState.NONE -> vueText("status_reason_none")
        else -> vueText(errKey)
    }

/**
 * The sheet a status light opens (UX audit L2, `DeviceStatusSheet.vue`): the device, its state in words, the reason, the
 * time of the last check and "Réessayer maintenant", which runs that device's check again ([onRetry]) and waits while
 * [busy]. Test tags: `device_sheet_state`, `device_sheet_reason`, `device_sheet_checked`, `device_sheet_retry`,
 * `device_sheet_close`.
 */
@Composable
internal fun DeviceStatusSheet(
    device: DeviceStatus,
    busy: Boolean,
    onRetry: () -> Unit,
    onClose: () -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    PiliDialog(
        title = device.name,
        onDismissRequest = onClose,
        maxWidth = SheetMaxWidth,
        actions = {
            PiliButton(
                text = vueText("status_sheet_close"),
                onClick = onClose,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("device_sheet_close"),
            )
            PiliButton(
                text = vueText("fdm_retry_now"),
                onClick = onRetry,
                variant = PiliButtonVariant.Secondary,
                enabled = !busy,
                modifier = Modifier.testTag("device_sheet_retry"),
            )
        },
    ) {
        Text(
            text = vueText(stateWordKey(device.state)),
            style = t.title.copy(fontSize = t.sizeXl),
            color = if (device.state == LightState.NONE) c.textSecondary else lightColor(c, device.state),
            modifier = Modifier.testTag("device_sheet_state"),
        )
        Text(text = device.reason, style = t.body, color = c.textPrimary, modifier = Modifier.testTag("device_sheet_reason"))
        val checked =
            device.checkedAt?.let { vueText("status_last_check", "time" to PosDateTime.formatTimeSeconds(it)) }
                ?: vueText("status_never_checked")
        Text(
            text = checked,
            style = t.body.copy(fontSize = t.sizeSm, fontFamily = t.familyFigures),
            color = c.textSecondary,
            modifier = Modifier.testTag("device_sheet_checked"),
        )
    }
}

private val SheetMaxWidth = 560.dp

/**
 * Draws the [DeviceStatusSheet] of the device [sheet] is open for, over the whole screen (put it in the scaffold's
 * full-size box). It reads the same lights as [StatusLights]; "Réessayer maintenant" calls [onCheckFdm] for the FDM and
 * [DeviceLights.retry] for the others. Nothing is drawn while no device is open or when it is gone (a terminal that
 * was unlinked).
 */
@Composable
internal fun DeviceStatusSheetHost(
    sheet: DeviceSheetState,
    fdm: FdmUiState,
    fdmCheckedAt: Instant?,
    lights: DeviceLights,
    hubriseVisible: Boolean,
    onCheckFdm: () -> Unit,
) {
    val key = sheet.openKey ?: return
    var retrying by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val device = deviceStatusOf(key, fdm, fdmCheckedAt, lights, hubriseVisible) ?: return
    DeviceStatusSheet(
        device = device,
        busy = retrying || (key == "fdm" && fdm.checking),
        onRetry = {
            if (key == "fdm") {
                onCheckFdm()
            } else {
                scope.launch {
                    retrying = true
                    try {
                        lights.retry(key)
                    } finally {
                        retrying = false
                    }
                }
            }
        },
        onClose = { sheet.openKey = null },
    )
}

/** What the sheet of the device [key] shows, read from the same lights as the header; null when the device is gone. */
@Composable
private fun deviceStatusOf(
    key: String,
    fdm: FdmUiState,
    fdmCheckedAt: Instant?,
    lights: DeviceLights,
    hubriseVisible: Boolean,
): DeviceStatus? {
    val checkedAt by lights.checkedAt.collectAsState()
    return when (key) {
        "fdm" -> fdmStatus(fdm, fdmCheckedAt)
        "ticket" -> printerStatus(key, "check_ticket_printer", lights.ticket.collectAsState().value, checkedAt[key])
        "kitchen" -> printerStatus(key, "check_kitchen_printer", lights.kitchen.collectAsState().value, checkedAt[key])
        "terminal" ->
            lights.terminal.collectAsState().value?.let {
                DeviceStatus(key, vueText("check_payment_terminal"), it.state, terminalDescription(it), checkedAt[key])
            }
        "scale" ->
            lights.scale.collectAsState().value?.let {
                DeviceStatus(key, vueText("status_scale"), it.state, vueText(it.descriptionKey), checkedAt[key])
            }
        "hubrise" -> if (hubriseVisible) hubriseStatus(lights.hubrise.collectAsState().value, checkedAt[key]) else null
        else -> null
    }
}

@Composable
private fun fdmStatus(
    fdm: FdmUiState,
    checkedAt: Instant?,
): DeviceStatus {
    val said = rememberFdmAlertText(fdm.code).message?.french
    val reason = said ?: vueText(if (fdm.connected) "status_reason_ok" else "status_reason_fdm_offline")
    return DeviceStatus("fdm", uiText(R.string.ui_common_light_fdm), fdmLight(fdm), reason, checkedAt)
}

@Composable
private fun printerStatus(
    key: String,
    nameKey: String,
    state: LightState,
    checkedAt: Instant?,
): DeviceStatus = DeviceStatus(key, vueText(nameKey), state, plainReason(state, "status_reason_printer_err"), checkedAt)

@Composable
private fun hubriseStatus(
    state: LightState,
    checkedAt: Instant?,
): DeviceStatus {
    val reason = if (state == LightState.WARN) vueText("status_reason_hubrise_warn") else plainReason(state, "status_reason_hubrise_err")
    return DeviceStatus("hubrise", uiText(R.string.ui_common_light_hubrise), state, reason, checkedAt)
}
