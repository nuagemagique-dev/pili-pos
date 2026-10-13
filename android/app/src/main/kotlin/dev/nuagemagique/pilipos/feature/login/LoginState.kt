package dev.nuagemagique.pilipos.feature.login

import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.PinMode
import dev.nuagemagique.pilipos.ui.components.PinPadState
import uniffi.pos_ffi.OpenShift
import uniffi.pos_ffi.User

/** Result of one start-up check, the `state` of a `Check` in `LoginView.vue`. */
internal enum class CheckState {
    /** Working. */
    Ok,

    /** Reachable with a warning, or configured but not reachable. */
    Warn,

    /** Not working. */
    Err,

    /** Not configured, or the engine did not answer. */
    None,
}

/** The device a start-up check is about, in the order the Vue screen lists them. */
internal enum class CheckId(
    /** Vue key of the label, or null when the label is a fixed name (FDM, HubRise). */
    val labelKey: String?,
) {
    Fdm(null),
    Ticket("check_ticket_printer"),
    Kitchen("check_kitchen_printer"),
    Terminal("check_payment_terminal"),
    Hubrise(null),
    Mail("check_email"),
}

/**
 * One line of the checks panel: the device, its [state] and the Vue key of the [textKey] shown on the right. [detail]
 * is why a device does not answer (the tooltip of the Vue row), when the engine said.
 */
internal data class StartupCheck(
    val id: CheckId,
    val state: CheckState,
    val textKey: String,
    val detail: String? = null,
)

/**
 * The PIN panel next to the tiles (`pinMode` of `LoginView.vue`).
 *
 * [pad] collects the 4 digits of an operator who has a PIN. [endShift] true makes the PIN end the shift instead
 * of opening the session. [error] is the refusal of an action that has no pad (an operator without a PIN).
 */
internal data class PinPanel(
    val user: User,
    val endShift: Boolean = false,
    val pad: PinPadState = PinPadState(PinMode.Verify),
    val error: String? = null,
)

/** The manager who must choose a first PIN (`firstPinUser` of `LoginView.vue`). */
internal data class FirstPinTarget(
    val insz: String,
    val name: String,
)

/** Everything the login screen draws. */
internal data class LoginUiState(
    /** The active users, one tile each. */
    val users: List<User> = emptyList(),
    /** The open shifts: who is clocked in since when. */
    val shifts: List<OpenShift> = emptyList(),
    /** True when the INSZ entry replaces the tiles (no user, the list failed, or the operator asked for it). */
    val numpadMode: Boolean = false,
    /** The INSZ typed so far, at most [INSZ_LENGTH] digits. */
    val insz: String = "",
    /** The refusal shown under the title. */
    val error: String? = null,
    /** The PIN panel, or null. */
    val pin: PinPanel? = null,
    /** The first-PIN dialog, or null. */
    val firstPin: FirstPinTarget? = null,
    /** The start-up checks; empty until they all ran. */
    val checks: List<StartupCheck> = emptyList(),
    /** Restaurant name and terminal id. */
    val footer: String = "",
    /** The software version (art. 47), empty when the settings are not loaded. */
    val version: String = "",
    /** The language of the texts. */
    val language: AppLanguage = AppLanguage.FR,
    /** True while the till is in training mode. */
    val training: Boolean = false,
) {
    /** When the open shift of [insz] started, or null when that operator is off shift. */
    fun shiftStart(insz: String): String? = shifts.firstOrNull { it.userInsz == insz }?.startedAt

    /** The name of the known user [insz], or the INSZ itself. */
    fun nameOf(insz: String): String = users.firstOrNull { it.insz == insz }?.name ?: insz

    companion object {
        /** An INSZ has 11 digits. */
        const val INSZ_LENGTH: Int = 11
    }
}

/** One-shot effects of the login screen, each delivered once and only after a user action. */
internal sealed interface LoginEffect {
    /** A session was opened: show the Tables screen (PD211). */
    data object GoToTables : LoginEffect
}
