package dev.nuagemagique.pilipos.platform.policy

import android.app.Application
import android.app.admin.DevicePolicyManager
import android.content.Context
import android.os.UserManager
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.provisioning.DeviceOwner
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

@RunWith(RobolectricTestRunner::class)
class AndroidDevicePoliciesTest {
    // Robolectric's manager reports no owner until setDeviceOwner is called; the not-owner case is the
    // second class of this file.
    private val context = ApplicationProvider.getApplicationContext<Application>()
    private val owner = DeviceOwner(context)
    private val policies = AndroidDevicePolicies(context, owner)
    private val shadow = shadowOf(context.getSystemService(Context.DEVICE_POLICY_SERVICE) as DevicePolicyManager)

    @Before
    fun becomeOwner() {
        shadow.setDeviceOwner(owner.adminComponent)
    }

    @Test
    fun theOwnerRoleComesFromTheFoundationHelper() {
        assertTrue(policies.isDeviceOwner())
    }

    @Test
    fun restrictionsAreAddedThroughTheAdmin() {
        // Robolectric does not shadow getUserRestrictions(admin): the read is checked on a device run (P6-T11).
        assertEquals(emptySet<String>(), policies.getUserRestrictions())
        policies.addUserRestriction("no_factory_reset")
        val users = context.getSystemService(UserManager::class.java)
        // Other tests of the run may leave restrictions in the shadow, so only the added key is checked.
        assertTrue(users.hasUserRestriction("no_factory_reset"))
    }

    @Test
    fun aRestrictionIsClearedThroughTheAdmin() {
        val users = context.getSystemService(UserManager::class.java)
        policies.addUserRestriction(PolicySet.INSTALL_APPS)
        assertTrue(users.hasUserRestriction(PolicySet.INSTALL_APPS))
        policies.clearUserRestriction(PolicySet.INSTALL_APPS)
        assertFalse(users.hasUserRestriction(PolicySet.INSTALL_APPS))
    }

    @Test
    fun theHideCallsReachTheManagerWithoutError() {
        // Robolectric's shadow does not model packages: what the framework answers for a missing or a present
        // package, and showing again, are checked on a device run (EV-13).
        policies.setApplicationHidden("com.android.vending", true)
        policies.isApplicationHidden("com.android.vending")
        policies.setApplicationHidden("com.android.vending", false)
    }

    @Test
    fun lockTaskFeaturesAreWrittenThroughTheAdmin() {
        policies.setLockTaskFeatures(DevicePolicyManager.LOCK_TASK_FEATURE_GLOBAL_ACTIONS)
        policies.setLockTaskFeatures(PolicySet.LOCK_TASK_FEATURES_NONE)
        val manager = context.getSystemService(DevicePolicyManager::class.java)
        assertEquals(0, manager.getLockTaskFeatures(owner.adminComponent))
    }

    @Test
    fun theOtherCallsReachTheManagerWithoutError() {
        policies.setUninstallBlocked(context.packageName, true)
        policies.setAutoTimeZoneEnabled(false)
        policies.setAutoTimeEnabled(true)
        policies.setSystemUpdatePolicyWindowed(PolicySet.UPDATE_WINDOW_START_MINUTE, PolicySet.UPDATE_WINDOW_END_MINUTE)
        policies.setGlobalSetting(PolicySet.STAY_ON_SETTING, PolicySet.STAY_ON_VALUE)
        val manager = context.getSystemService(DevicePolicyManager::class.java)
        assertTrue(manager.isUninstallBlocked(owner.adminComponent, context.packageName))
        assertTrue(manager.getAutoTimeEnabled(owner.adminComponent))
        assertFalse(manager.getAutoTimeZoneEnabled(owner.adminComponent))
        val update = manager.getSystemUpdatePolicy()
        assertEquals(android.app.admin.SystemUpdatePolicy.TYPE_INSTALL_WINDOWED, update?.policyType)
        assertEquals(180, update?.installWindowStart)
        assertEquals(300, update?.installWindowEnd)
        policies.setTimeZone(PolicySet.TIME_ZONE)
        policies.setKeyguardDisabled(true)
        policies.setStatusBarDisabled(true)
    }
}

@RunWith(RobolectricTestRunner::class)
class AndroidDevicePoliciesNotOwnerTest {
    @Test
    fun anAppThatIsNotOwnerIsReportedAsSuch() {
        val context = ApplicationProvider.getApplicationContext<Application>()
        assertFalse(AndroidDevicePolicies(context, DeviceOwner(context)).isDeviceOwner())
    }
}
