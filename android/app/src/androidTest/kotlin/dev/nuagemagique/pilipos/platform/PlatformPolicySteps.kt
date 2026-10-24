package dev.nuagemagique.pilipos.platform

import android.app.KeyguardManager
import android.app.admin.DevicePolicyManager
import android.app.admin.SystemUpdatePolicy
import android.content.pm.PackageManager
import android.os.UserManager
import android.provider.Settings
import dev.nuagemagique.pilipos.platform.policy.PolicySet
import dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import java.util.TimeZone

/** The `provision_and_policies` step: the device-owner policies of PD206, read back through the system. */
internal object PlatformPolicySteps {
    private const val POLICY_TIMEOUT_MS = 30_000L

    /** Asserts the PD206 set after the import (the policies are applied in the application scope, so it polls). */
    fun assertPolicies(env: PlatformE2eEnv) {
        val context = env.context
        val dpm = context.getSystemService(DevicePolicyManager::class.java)
        val users = context.getSystemService(UserManager::class.java)
        val admin = PiliAdminReceiver.component(context)

        assertTrue("the app must be the device owner", dpm.isDeviceOwnerApp(context.packageName))
        env.waitFor("the PD206 restrictions", POLICY_TIMEOUT_MS) { PolicySet.RESTRICTIONS.all { users.hasUserRestriction(it) } }
        val missing = PolicySet.RESTRICTIONS.filterNot { users.hasUserRestriction(it) }
        assertTrue("restrictions missing: $missing", missing.isEmpty())
        assertFalse("no_debugging_features must stay off in a debug build", users.hasUserRestriction(PolicySet.DEBUGGING_FEATURES))
        PolicySet.NEVER_APPLIED.forEach { assertFalse("$it must never be applied", users.hasUserRestriction(it)) }

        env.waitFor("lock task features 0", POLICY_TIMEOUT_MS) { dpm.getLockTaskFeatures(admin) == PolicySet.LOCK_TASK_FEATURES_NONE }
        assertEquals("lock task features", PolicySet.LOCK_TASK_FEATURES_NONE, dpm.getLockTaskFeatures(admin))
        assertTrue("auto time must be on", dpm.getAutoTimeEnabled(admin))
        assertFalse("auto time zone must be off", dpm.getAutoTimeZoneEnabled(admin))
        env.waitFor("time zone ${PolicySet.TIME_ZONE}", POLICY_TIMEOUT_MS) { systemZone() == PolicySet.TIME_ZONE }
        assertEquals(PolicySet.TIME_ZONE, systemZone())
        assertTrue("uninstall must be blocked", dpm.isUninstallBlocked(admin, context.packageName))

        val update = dpm.getSystemUpdatePolicy()
        assertEquals("system update policy", SystemUpdatePolicy.TYPE_INSTALL_WINDOWED, update?.policyType)
        assertEquals("update window start", PolicySet.UPDATE_WINDOW_START_MINUTE, update?.installWindowStart)
        assertEquals("update window end", PolicySet.UPDATE_WINDOW_END_MINUTE, update?.installWindowEnd)
        val stillVisible =
            PolicySet.HIDDEN_PACKAGES.filter { isInstalled(context.packageManager, it) && !dpm.isApplicationHidden(admin, it) }
        assertTrue("Google apps still visible: $stillVisible", stillVisible.isEmpty())

        val stayOn = Settings.Global.getString(context.contentResolver, PolicySet.STAY_ON_SETTING)
        assertEquals("stay awake while plugged in", PolicySet.STAY_ON_VALUE, stayOn)
        val keyguard = context.getSystemService(KeyguardManager::class.java)
        assertFalse("the keyguard must be disabled", keyguard.isKeyguardLocked)
        PlatformE2eEnv.line("PLATE2E_POLICIES ok restrictions=${PolicySet.RESTRICTIONS.size} zone=${systemZone()}")
    }

    /** True when [pkg] is on the device and not hidden yet (a hidden package no longer resolves). */
    private fun isInstalled(
        packageManager: PackageManager,
        pkg: String,
    ): Boolean =
        try {
            packageManager.getPackageInfo(pkg, 0)
            true
        } catch (_: PackageManager.NameNotFoundException) {
            false
        }

    /** The zone the system holds now: the cached default of the process is dropped first. */
    private fun systemZone(): String {
        TimeZone.setDefault(null)
        return TimeZone.getDefault().id
    }
}
