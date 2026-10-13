package dev.nuagemagique.pilipos.platform.policy

import android.os.UserManager
import android.provider.Settings
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PolicySetTest {
    @Test
    fun theSetHoldsTheTwelveMeasuredRestrictionsTheErrorDialogOneAndInstallApps() {
        assertEquals(
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
            ),
            PolicySet.RESTRICTIONS,
        )
        assertEquals(14, PolicySet.RESTRICTIONS.toSet().size)
    }

    @Test
    fun theKeysAreTheFrameworkConstants() {
        assertEquals(
            listOf(
                UserManager.DISALLOW_FACTORY_RESET,
                UserManager.DISALLOW_SAFE_BOOT,
                UserManager.DISALLOW_UNINSTALL_APPS,
                UserManager.DISALLOW_INSTALL_APPS,
                UserManager.DISALLOW_INSTALL_UNKNOWN_SOURCES,
                UserManager.DISALLOW_INSTALL_UNKNOWN_SOURCES_GLOBALLY,
                UserManager.DISALLOW_CONFIG_DATE_TIME,
                UserManager.DISALLOW_ADD_USER,
                UserManager.DISALLOW_USER_SWITCH,
                UserManager.DISALLOW_REMOVE_USER,
                UserManager.DISALLOW_MODIFY_ACCOUNTS,
                UserManager.DISALLOW_USB_FILE_TRANSFER,
                UserManager.DISALLOW_NETWORK_RESET,
                UserManager.DISALLOW_SYSTEM_ERROR_DIALOGS,
            ),
            PolicySet.RESTRICTIONS,
        )
        assertEquals(UserManager.DISALLOW_DEBUGGING_FEATURES, PolicySet.DEBUGGING_FEATURES)
        assertEquals(UserManager.DISALLOW_INSTALL_APPS, PolicySet.INSTALL_APPS)
        // The framework key of "mount physical media" is no_physical_media: both spellings are barred.
        assertEquals("no_physical_media", UserManager.DISALLOW_MOUNT_PHYSICAL_MEDIA)
        assertTrue(PolicySet.NEVER_APPLIED.containsAll(listOf(UserManager.DISALLOW_MOUNT_PHYSICAL_MEDIA, UserManager.DISALLOW_CONFIG_WIFI)))
        assertEquals(Settings.Global.STAY_ON_WHILE_PLUGGED_IN, PolicySet.STAY_ON_SETTING)
    }

    @Test
    fun mountPhysicalMediaAndConfigWifiAreAbsentAndAdbOffIsNotInTheEverywhereSet() {
        assertFalse("no_mount_physical_media" in PolicySet.RESTRICTIONS)
        assertFalse("no_physical_media" in PolicySet.RESTRICTIONS)
        assertFalse("no_config_wifi" in PolicySet.RESTRICTIONS)
        assertFalse(PolicySet.DEBUGGING_FEATURES in PolicySet.RESTRICTIONS)
        assertTrue(PolicySet.NEVER_APPLIED.none { it in PolicySet.RESTRICTIONS })
        assertEquals("no_debugging_features", PolicySet.DEBUGGING_FEATURES)
    }

    @Test
    fun theOtherConstants() {
        assertEquals("7", PolicySet.STAY_ON_VALUE)
        assertEquals("Europe/Brussels", PolicySet.TIME_ZONE)
        assertEquals(0, PolicySet.LOCK_TASK_FEATURES_NONE)
    }

    @Test
    fun theHiddenPackagesAreTheGoogleAppsAndNeverPlayServicesOrTheSystem() {
        assertTrue("com.android.vending" in PolicySet.HIDDEN_PACKAGES)
        assertTrue("com.android.chrome" in PolicySet.HIDDEN_PACKAGES)
        assertEquals(PolicySet.HIDDEN_PACKAGES.size, PolicySet.HIDDEN_PACKAGES.toSet().size)
        assertTrue(PolicySet.HIDDEN_PACKAGES.none { it in PolicySet.NEVER_HIDDEN })
        assertTrue(PolicySet.HIDDEN_PACKAGES.none { it.startsWith("com.android.settings") || it.startsWith("com.android.systemui") })
        assertTrue(
            listOf("com.google.android.gms", "com.google.android.gsf", "com.android.settings", "com.android.systemui").all {
                it in PolicySet.NEVER_HIDDEN
            },
        )
        assertFalse("dev.nuagemagique.pilipos" in PolicySet.HIDDEN_PACKAGES)
    }

    @Test
    fun theUpdateWindowIsThreeToFiveAtNight() {
        assertEquals(180, PolicySet.UPDATE_WINDOW_START_MINUTE)
        assertEquals(300, PolicySet.UPDATE_WINDOW_END_MINUTE)
    }
}
