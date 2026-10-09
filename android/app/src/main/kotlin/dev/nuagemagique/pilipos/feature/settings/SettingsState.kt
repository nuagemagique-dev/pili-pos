package dev.nuagemagique.pilipos.feature.settings

import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.nav.SettingsEntry
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import uniffi.pos_ffi.DeliveryView
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PosSettings
import uniffi.pos_ffi.SwitchChange
import uniffi.pos_ffi.User

/** The role of a new employee, in the order of the Vue role buttons (`roles` of `SettingsView.vue`). */
internal enum class NewRole(
    /** The engine role string of `CreateUserInput.role`. */
    val code: String,
    /** The Vue key of the button label (`role_<code>`). */
    val labelKey: String,
) {
    Admin("admin", "role_admin"),
    Cashier("cashier", "role_cashier"),
    Waiter("waiter", "role_waiter"),
}

/** Whose PIN the manager is setting. */
internal enum class PinPurpose {
    /** The PIN of the logged-in operator himself: typed twice, then saved. */
    Own,

    /** The PIN of another employee: chosen, then confirmed with the manager's own PIN. */
    Other,

    /** The first PIN of a new manager: chosen, then confirmed with the creating manager's own PIN. */
    NewManager,
}

/** Where the PIN dialogs are (the three `PinDialog`s of `SettingsView.vue`). */
internal enum class PinStage {
    /** The new PIN is typed twice (`PinMode.Choose`). */
    Choose,

    /** The new PIN is chosen; the manager's own PIN confirms it (`PinMode.Verify`, hint `pin_confirm_manager`). */
    ConfirmWithManager,
}

/**
 * The PIN dialog being shown. [insz] and [name] are the person the PIN is for; [chosen] is the PIN picked in
 * [PinStage.Choose] of an [PinPurpose.Other] or [PinPurpose.NewManager] flow.
 */
internal data class SettingsPinFlow(
    val purpose: PinPurpose,
    val insz: String,
    val name: String,
    val stage: PinStage = PinStage.Choose,
    val chosen: String? = null,
)

/** The "add an employee" form (`newName`, `newInsz`, `newRole`, `addError`; a failed deactivation shows in [error] too). */
internal data class NewEmployeeForm(
    val name: String = "",
    val insz: String = "",
    val role: NewRole = NewRole.Cashier,
    val error: String? = null,
)

/** The HubRise switch panel (`IntegrationSwitch.vue`). */
internal data class HubriseSwitchState(
    /** True when the integration is switched on. */
    val enabled: Boolean = false,
    /** True when the HubRise screens are shown: on, or online orders are still to settle. */
    val visible: Boolean = false,
    /** Online orders still to settle. */
    val onlineOrdersOpen: Long = 0L,
    /** The latest changes first; the times are engine strings (UTC, the screen formats them in Brussels). */
    val log: List<SwitchChange> = emptyList(),
    /** True while a switch is being sent: a second tap is ignored. */
    val busy: Boolean = false,
)

/** Everything the settings screen draws, except the HubRise panel ([HubriseUiState]). */
internal data class SettingsUiState(
    /** The settings rows the logged-in role may see, in order (UI-PD308). */
    val rows: List<SettingsEntry> = emptyList(),
    /** True for a manager. */
    val isManager: Boolean = false,
    /** The INSZ of the logged-in operator, or null. */
    val currentInsz: String? = null,
    val training: Boolean = false,
    val language: AppLanguage = AppLanguage.FR,
    val productLabel: ProductLabel = ProductLabel.NAME,
    /** The info panel; null until the engine settings are loaded. */
    val info: PosSettings? = null,
    /** The employees, as the engine listed them (active and inactive). */
    val users: List<User> = emptyList(),
    /** The INSZ whose deactivate button is armed (second tap needed), or null. */
    val confirmDeactivate: String? = null,
    val form: NewEmployeeForm = NewEmployeeForm(),
    val pin: SettingsPinFlow? = null,
    /** The last ticket deliveries, newest first as the engine answers; the times are engine strings. */
    val deliveries: List<DeliveryView> = emptyList(),
    val deliveriesLoading: Boolean = false,
    /** The mail service state; null until the first answer. */
    val mailStatus: MailStatus? = null,
    val hubrise: HubriseSwitchState = HubriseSwitchState(),
) {
    /** True when the user [insz] is the operator himself (no deactivate button, own PIN flow). */
    fun isSelf(insz: String): Boolean = insz == currentInsz
}

/** The first letter of a name for the avatar (`initial` of `SettingsView.vue`, surrogate pairs kept whole). */
internal fun avatarInitial(name: String): String {
    val first = name.trim().codePoints().findFirst()
    return if (first.isPresent) String(Character.toChars(first.asInt)).uppercase() else "?"
}

/**
 * The rows of the settings screen for a role: [all] in their order, the manager-only rows only for a manager
 * (UI-PD308). The caller passes `NavRegistry.settingsEntries`, which is already sorted by order, then id.
 */
internal fun visibleRows(
    all: List<SettingsEntry>,
    isManager: Boolean,
): List<SettingsEntry> = all.filter { isManager || !it.managerOnly }
