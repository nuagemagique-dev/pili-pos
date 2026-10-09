package dev.nuagemagique.pilipos.platform.policy

/**
 * The constant device-owner policy set (PD206, DECISIONS Q16, S4 section 8).
 *
 * Restriction keys are written out as strings (they are the values of the `UserManager.DISALLOW_*`
 * constants) so that the set can be read, and tested, without the framework.
 */
object PolicySet {
    /**
     * The user restrictions applied in every build, in the order they are applied: the 12 measured in S4
     * plus `no_system_error_dialogs` (a second crash must not freeze the till behind a dialog).
     */
    val RESTRICTIONS: List<String> =
        listOf(
            "no_factory_reset",
            "no_safe_boot",
            "no_uninstall_apps",
            "no_install_unknown_sources",
            "no_install_unknown_sources_globally",
            "no_config_date_time",
            "no_add_user",
            "no_user_switch",
            "no_remove_user",
            "no_modify_accounts",
            "no_usb_file_transfer",
            "no_network_reset",
            "no_system_error_dialogs",
        )

    /** Adb off: applied last, in a release build, once a provisioning record exists. Debug builds need adb. */
    const val DEBUGGING_FEATURES = "no_debugging_features"

    /**
     * Restrictions that must never be applied: the export needs a mounted USB stick or SD card, and the
     * manager must be able to fix the Wi-Fi on site (Q16). The framework key of `DISALLOW_MOUNT_PHYSICAL_MEDIA`
     * is `no_physical_media`; the name used in the decisions (`no_mount_physical_media`) is barred as well.
     */
    val NEVER_APPLIED: Set<String> = setOf("no_mount_physical_media", "no_physical_media", "no_config_wifi")

    /** Name of the global setting that keeps the screen on while plugged in (`Settings.Global`). */
    const val STAY_ON_SETTING = "stay_on_while_plugged_in"

    /** Value 7: stay on for AC, USB and wireless power. */
    const val STAY_ON_VALUE = "7"

    /** The only time zone of the till (hard-coded, never the runtime zone). */
    const val TIME_ZONE = "Europe/Brussels"

    /** Lock Task features NONE: no power menu, no home, no overview for the staff. It reverts at every boot. */
    const val LOCK_TASK_FEATURES_NONE = 0
}
