package dev.nuagemagique.pilipos.platform.policy

import android.app.admin.DevicePolicyManager
import android.app.admin.SystemUpdatePolicy
import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.provisioning.DeviceOwner
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The calls of [DevicePolicyManager] that the policy layer needs, and no others: a seam so that the order and
 * the isolation of the steps are tested without a device. Every method may throw (a `SecurityException` when
 * the app lost the owner role, an `IllegalArgumentException` for a refused value); [PolicyApplier] isolates
 * each call.
 */
@Suppress("TooManyFunctions")
interface DevicePolicies {
    /** True when this app is the device owner. */
    fun isDeviceOwner(): Boolean

    /** Adds a user restriction. */
    fun addUserRestriction(key: String)

    /** The restrictions this admin has set at the moment. */
    fun getUserRestrictions(): Set<String>

    /** Blocks or allows the uninstall of [packageName]. */
    fun setUninstallBlocked(
        packageName: String,
        blocked: Boolean,
    )

    /** Turns the automatic time on or off. */
    fun setAutoTimeEnabled(enabled: Boolean)

    /** Turns the automatic time zone on or off. */
    fun setAutoTimeZoneEnabled(enabled: Boolean)

    /** Sets the time zone; false when the system refused it (the automatic zone is still on). */
    fun setTimeZone(timeZoneId: String): Boolean

    /** Writes a global setting. */
    fun setGlobalSetting(
        name: String,
        value: String,
    )

    /** Disables or enables the keyguard; false when the system refused it. */
    fun setKeyguardDisabled(disabled: Boolean): Boolean

    /** Disables or enables the status bar; false when the system refused it. */
    fun setStatusBarDisabled(disabled: Boolean): Boolean

    /** Sets the system update policy to "postpone": the till never updates its OS on its own. */
    fun setSystemUpdatePolicyPostpone()

    /** Sets the Lock Task features (a `LOCK_TASK_FEATURE_*` bit mask; 0 is none). */
    fun setLockTaskFeatures(features: Int)
}

/** The production [DevicePolicies]: a [DevicePolicyManager] used through the admin of [DeviceOwner]. */
@Singleton
@Suppress("TooManyFunctions")
class AndroidDevicePolicies
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
        private val deviceOwner: DeviceOwner,
    ) : DevicePolicies {
        private val manager: DevicePolicyManager
            get() = context.getSystemService(DevicePolicyManager::class.java)

        private val admin get() = deviceOwner.adminComponent

        override fun isDeviceOwner(): Boolean = deviceOwner.isDeviceOwner()

        override fun addUserRestriction(key: String) = manager.addUserRestriction(admin, key)

        override fun getUserRestrictions(): Set<String> {
            val bundle = manager.getUserRestrictions(admin)
            return bundle.keySet().filterTo(HashSet()) { bundle.getBoolean(it) }
        }

        override fun setUninstallBlocked(
            packageName: String,
            blocked: Boolean,
        ) = manager.setUninstallBlocked(admin, packageName, blocked)

        override fun setAutoTimeEnabled(enabled: Boolean) = manager.setAutoTimeEnabled(admin, enabled)

        override fun setAutoTimeZoneEnabled(enabled: Boolean) = manager.setAutoTimeZoneEnabled(admin, enabled)

        override fun setTimeZone(timeZoneId: String): Boolean = manager.setTimeZone(admin, timeZoneId)

        override fun setGlobalSetting(
            name: String,
            value: String,
        ) = manager.setGlobalSetting(admin, name, value)

        override fun setKeyguardDisabled(disabled: Boolean): Boolean = manager.setKeyguardDisabled(admin, disabled)

        override fun setStatusBarDisabled(disabled: Boolean): Boolean = manager.setStatusBarDisabled(admin, disabled)

        override fun setSystemUpdatePolicyPostpone() =
            manager.setSystemUpdatePolicy(admin, SystemUpdatePolicy.createPostponeInstallPolicy())

        override fun setLockTaskFeatures(features: Int) = manager.setLockTaskFeatures(admin, features)
    }
