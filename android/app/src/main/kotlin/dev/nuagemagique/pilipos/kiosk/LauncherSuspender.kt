package dev.nuagemagique.pilipos.kiosk

import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.content.pm.PackageManager
import timber.log.Timber

/**
 * Suspends every other home (launcher) package and the system Settings, and remembers which ones it
 * suspended so that [unsuspendStored] can undo it (S4 exit notes: the escape window after an update).
 */
internal class LauncherSuspender(
    private val context: Context,
    private val preferences: SharedPreferences,
) {
    /**
     * Suspends the packages that resolve the HOME intent, except ours, plus Settings when installed. The
     * packages the OS refuses to suspend are logged; the suspended ones join the stored list.
     */
    fun suspendOthers(
        manager: DevicePolicyManager,
        admin: ComponentName,
    ) {
        val wanted = (homePackages() + settingsPackage()).filter { it != context.packageName }.distinct()
        if (wanted.isEmpty()) {
            Timber.i("kiosk: no other launcher to suspend")
            return
        }
        val failed = manager.setPackagesSuspended(admin, wanted.toTypedArray(), true).toSet()
        Timber.i("kiosk: suspended %s, could not suspend %s", wanted - failed, failed)
        // Union with the earlier list: a suspended launcher may stop resolving, release must still find it.
        val stored = preferences.getStringSet(KEY_SUSPENDED, emptySet()).orEmpty()
        preferences.edit().putStringSet(KEY_SUSPENDED, stored + (wanted - failed)).commit()
    }

    /** Lifts the suspension of the stored packages. */
    fun unsuspendStored(
        manager: DevicePolicyManager,
        admin: ComponentName,
    ) {
        val stored = preferences.getStringSet(KEY_SUSPENDED, emptySet()).orEmpty().filter { it != context.packageName }
        if (stored.isEmpty()) return
        val failed = manager.setPackagesSuspended(admin, stored.toTypedArray(), false).toSet()
        Timber.i("kiosk: unsuspended %s, could not unsuspend %s", stored - failed, failed)
    }

    private fun homePackages(): List<String> {
        val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME)
        val flags = PackageManager.ResolveInfoFlags.of(PackageManager.MATCH_ALL.toLong())
        return context.packageManager
            .queryIntentActivities(intent, flags)
            .mapNotNull { it.activityInfo?.packageName }
    }

    private fun settingsPackage(): List<String> =
        try {
            context.packageManager.getPackageInfo(SETTINGS_PACKAGE, PackageManager.PackageInfoFlags.of(0L))
            listOf(SETTINGS_PACKAGE)
        } catch (e: PackageManager.NameNotFoundException) {
            Timber.i(e, "kiosk: %s is not installed", SETTINGS_PACKAGE)
            emptyList()
        }

    companion object {
        /** SharedPreferences key (file 'kiosk') of the set of packages this app suspended. */
        const val KEY_SUSPENDED = "suspended"
        private const val SETTINGS_PACKAGE = "com.android.settings"
    }
}
