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
     * plus `no_system_error_dialogs` (a second crash must not freeze the till behind a dialog) and
     * `no_install_apps` (PD230, the GMS variant ships the Play Store). `no_install_apps` is the only one
     * the updater lifts for its own session ([INSTALL_APPS]) and sets again in a `finally`; a crash that
     * leaves it cleared is repaired by the next [PolicyApplier.apply].
     */
    val RESTRICTIONS: List<String> =
        listOf(
            "no_factory_reset",
            "no_safe_boot",
            "no_uninstall_apps",
            "no_install_apps",
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

    /** `DISALLOW_INSTALL_APPS`: the one restriction the updater clears for its own PackageInstaller session. */
    const val INSTALL_APPS = "no_install_apps"

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

    /**
     * Start of the system update window, in minutes after local midnight (03:00). The till is closed then
     * (service hours end at night and start late morning), and the zone is locked to [TIME_ZONE], so the
     * window is Brussels time. A postpone policy only delays an update by 30 days and the OS then updates
     * whenever it wants, possibly during service; a windowed policy keeps every install at night.
     */
    const val UPDATE_WINDOW_START_MINUTE = 3 * 60

    /** End of the system update window, in minutes after local midnight (05:00). */
    const val UPDATE_WINDOW_END_MINUTE = 5 * 60

    /**
     * The user-facing Google and GMS apps hidden with `setApplicationHidden` (PD230). The D3 Pro GMS image
     * ships them launcher-visible; with Lock Task they cannot be reached from the till, but a hidden app
     * can neither be started by an intent nor install software, and the Play Store is the way to install
     * anything. A package that is not on the device is ignored. Packages are hidden, not disabled: the
     * state is per user, survives a reboot, and is lifted by `setApplicationHidden(false)`.
     *
     * - `com.android.vending`: Google Play Store, installs and updates apps.
     * - `com.android.chrome`: Chrome, free browsing from a link or a share sheet.
     * - `com.google.android.googlequicksearchbox`: the Google app and the Assistant.
     * - `com.google.android.youtube`, `com.google.android.apps.youtube.music`, `com.google.android.videos`:
     *   media apps.
     * - `com.google.android.gm`: Gmail, a mail account on the till.
     * - `com.google.android.apps.maps`: Maps.
     * - `com.google.android.apps.photos`: Photos, cloud upload of the screen captures.
     * - `com.google.android.apps.docs`: Drive, file sharing out of the till.
     * - `com.google.android.apps.nbu.files`: Files by Google, can install an APK from storage.
     * - `com.google.android.apps.tachyon`: Meet, calls.
     * - `com.google.android.apps.messaging`: Messages.
     * - `com.google.android.calendar`: Calendar.
     */
    val HIDDEN_PACKAGES: List<String> =
        listOf(
            "com.android.vending",
            "com.android.chrome",
            "com.google.android.googlequicksearchbox",
            "com.google.android.youtube",
            "com.google.android.apps.youtube.music",
            "com.google.android.videos",
            "com.google.android.gm",
            "com.google.android.apps.maps",
            "com.google.android.apps.photos",
            "com.google.android.apps.docs",
            "com.google.android.apps.nbu.files",
            "com.google.android.apps.tachyon",
            "com.google.android.apps.messaging",
            "com.google.android.calendar",
        )

    /**
     * Packages that must never be hidden, whatever [HIDDEN_PACKAGES] holds: Google Play services and the
     * services framework (push, Play Integrity), the system UI and settings (the manager's Wi-Fi screen,
     * Q16), the package installer and permission controller (the updater session), the WebView, the
     * keyboard and the text-to-speech engine. Our own package is added by [PolicyApplier].
     */
    val NEVER_HIDDEN: Set<String> =
        setOf(
            "com.google.android.gms",
            "com.google.android.gsf",
            "com.android.systemui",
            "com.android.settings",
            "com.android.packageinstaller",
            "com.google.android.packageinstaller",
            "com.google.android.permissioncontroller",
            "com.google.android.webview",
            "com.google.android.inputmethod.latin",
            "com.google.android.tts",
        )
}
