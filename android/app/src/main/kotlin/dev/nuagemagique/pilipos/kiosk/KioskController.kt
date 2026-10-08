package dev.nuagemagique.pilipos.kiosk

import android.app.Activity
import android.app.ActivityManager
import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.provisioning.DeviceOwner
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/** Whether the till is held in the kiosk, as far as this process knows. */
sealed interface KioskState {
    /** The app is not the device owner: no policy is applied and the shell says so (P4-T11). */
    data object NotOwner : KioskState

    /** The owner policies are applied; the activity is not (yet) in Lock Task mode. */
    data object Prepared : KioskState

    /** The activity is pinned: the system reports Lock Task mode LOCKED. */
    data object Locked : KioskState
}

/**
 * The kiosk of the till (PD109, plan P4.5): Lock Task entry, the app as the persistent home, and the
 * stock launcher and Settings suspended so that nothing else can be reached after an update (S4 exit
 * notes, Q18).
 *
 * Only the device owner may do any of it; without the role every call is a no-op and the state is
 * [KioskState.NotOwner]. The restrictions (`DISALLOW_*`), the time policy and adb stay in P6.
 */
@Singleton
class KioskController
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
        private val deviceOwner: DeviceOwner,
    ) {
        private val mutableState = MutableStateFlow<KioskState>(KioskState.NotOwner)

        /** The state after the last [prepare], [relock], [refreshState] or [release]. */
        val state: StateFlow<KioskState> = mutableState.asStateFlow()

        private val policyManager: DevicePolicyManager
            get() = context.getSystemService(DevicePolicyManager::class.java)

        private val preferences by lazy { context.getSharedPreferences(PREFS, Context.MODE_PRIVATE) }

        private val suspender by lazy { LauncherSuspender(context, preferences) }

        /**
         * Applies the owner policies; call it at every start (S4: the lock task features revert to the
         * default at every boot). Order: own permissions, lock task packages, lock task features,
         * persistent HOME, suspension of the other launchers and Settings, kiosk flag.
         *
         * Each policy call is isolated: one that the OS refuses is logged and the rest still run.
         */
        fun prepare() {
            if (!deviceOwner.isDeviceOwner()) {
                Timber.i("kiosk: not the device owner, no policy applied")
                mutableState.value = KioskState.NotOwner
                return
            }
            val admin = deviceOwner.adminComponent
            val manager = policyManager
            deviceOwner.grantOwnPermissions()
            step("lock task packages") { manager.setLockTaskPackages(admin, arrayOf(context.packageName)) }
            step("lock task features") {
                manager.setLockTaskFeatures(admin, DevicePolicyManager.LOCK_TASK_FEATURE_GLOBAL_ACTIONS)
            }
            step("persistent home") {
                manager.addPersistentPreferredActivity(admin, homeFilter(), ComponentName(context, MainActivity::class.java))
            }
            step("suspend launchers") { suspender.suspendOthers(manager, admin) }
            preferences.edit().putBoolean(KEY_ENABLED, true).commit()
            mutableState.value = currentState()
            Timber.i("kiosk: prepared, state=%s", mutableState.value)
        }

        /**
         * Enters Lock Task mode when the app is the device owner, the kiosk flag is set, the app is
         * permitted and the mode is NONE. Updates [state] from the mode the system reports; entering
         * the mode is asynchronous, so [refreshState] reads the result.
         */
        fun relock(activity: Activity) {
            if (!deviceOwner.isDeviceOwner()) {
                mutableState.value = KioskState.NotOwner
                return
            }
            val manager = policyManager
            val mode = lockTaskMode(activity)
            if (preferences.getBoolean(KEY_ENABLED, false) &&
                manager.isLockTaskPermitted(context.packageName) &&
                mode == ActivityManager.LOCK_TASK_MODE_NONE
            ) {
                Timber.i("kiosk: entering lock task")
                activity.startLockTask()
            }
            refreshState(activity)
        }

        /** Recomputes [state] from the owner role and the Lock Task mode of [activity]. */
        fun refreshState(activity: Activity) {
            mutableState.value =
                when {
                    !deviceOwner.isDeviceOwner() -> KioskState.NotOwner
                    lockTaskMode(activity) == ActivityManager.LOCK_TASK_MODE_LOCKED -> KioskState.Locked
                    else -> KioskState.Prepared
                }
        }

        /**
         * Undoes [prepare]: ends Lock Task (empty package list), unsuspends the stored packages,
         * removes the persistent HOME and clears the flag. Without the role only the flag is cleared.
         */
        fun release() {
            if (deviceOwner.isDeviceOwner()) {
                val admin = deviceOwner.adminComponent
                val manager = policyManager
                step("release lock task packages") { manager.setLockTaskPackages(admin, emptyArray()) }
                step("unsuspend launchers") { suspender.unsuspendStored(manager, admin) }
                step("clear persistent home") { manager.clearPackagePersistentPreferredActivities(admin, context.packageName) }
            } else {
                Timber.i("kiosk: release without the device owner role, only the flag is cleared")
            }
            preferences.edit().putBoolean(KEY_ENABLED, false).remove(LauncherSuspender.KEY_SUSPENDED).commit()
            mutableState.value = KioskState.NotOwner
        }

        private fun currentState(): KioskState =
            if (activityManager().lockTaskModeState == ActivityManager.LOCK_TASK_MODE_LOCKED) KioskState.Locked else KioskState.Prepared

        private fun activityManager(): ActivityManager = context.getSystemService(ActivityManager::class.java)

        private fun lockTaskMode(activity: Activity): Int = activity.getSystemService(ActivityManager::class.java).lockTaskModeState

        @Suppress("TooGenericExceptionCaught")
        private fun step(
            name: String,
            block: () -> Unit,
        ) {
            try {
                block()
            } catch (e: Exception) {
                Timber.w(e, "kiosk: %s failed (%s)", name, e.javaClass.simpleName)
            }
        }

        private fun homeFilter(): IntentFilter =
            IntentFilter(Intent.ACTION_MAIN).apply {
                addCategory(Intent.CATEGORY_HOME)
                addCategory(Intent.CATEGORY_DEFAULT)
            }

        private companion object {
            const val PREFS = "kiosk"
            const val KEY_ENABLED = "enabled"
        }
    }
