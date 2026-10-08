package dev.nuagemagique.pilipos.kiosk

import android.app.Activity
import android.app.ActivityManager
import android.app.admin.DevicePolicyManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ActivityInfo
import android.content.pm.ApplicationInfo
import android.content.pm.PackageInfo
import android.content.pm.ResolveInfo
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.provisioning.DeviceOwner
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

@RunWith(RobolectricTestRunner::class)
class KioskControllerTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private val manager: DevicePolicyManager = context.getSystemService(DevicePolicyManager::class.java)
    private val deviceOwner = DeviceOwner(context)
    private val admin = deviceOwner.adminComponent
    private val controller = KioskController(context, deviceOwner)
    private val prefs = context.getSharedPreferences("kiosk", Context.MODE_PRIVATE)

    @Before
    fun installSystemApps() {
        val packageManager = shadowOf(context.packageManager)
        packageManager.installPackage(
            PackageInfo().apply {
                packageName = FAKE_LAUNCHER
                applicationInfo = ApplicationInfo().apply { packageName = FAKE_LAUNCHER }
            },
        )
        packageManager.installPackage(
            PackageInfo().apply {
                packageName = SETTINGS
                applicationInfo = ApplicationInfo().apply { packageName = SETTINGS }
            },
        )
        val launcher =
            ResolveInfo().apply {
                activityInfo =
                    ActivityInfo().apply {
                        packageName = FAKE_LAUNCHER
                        name = "$FAKE_LAUNCHER.Home"
                        applicationInfo = ApplicationInfo().apply { packageName = FAKE_LAUNCHER }
                    }
            }
        packageManager.addResolveInfoForIntent(homeIntent(), launcher)
    }

    @Test
    fun preparingAsOwnerSetsTheLockTaskPackagesToOurPackageOnly() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        assertArrayEquals(arrayOf(context.packageName), manager.getLockTaskPackages(admin))
        assertEquals(KioskState.Prepared, controller.state.value)
    }

    @Test
    fun preparingAsOwnerKeepsOnlyTheGlobalActionsLockTaskFeature() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        assertEquals(DevicePolicyManager.LOCK_TASK_FEATURE_GLOBAL_ACTIONS, manager.getLockTaskFeatures(admin))
    }

    @Test
    fun preparingAsOwnerGrantsTheOwnPermissions() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        assertEquals(
            DevicePolicyManager.PERMISSION_GRANT_STATE_GRANTED,
            manager.getPermissionGrantState(admin, context.packageName, android.Manifest.permission.READ_PHONE_STATE),
        )
    }

    @Test
    fun preparingAsOwnerMakesTheMainActivityThePersistentHome() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        val filters = mutableListOf<IntentFilter>()
        val components = mutableListOf<android.content.ComponentName>()
        shadowOf(context.packageManager).getPersistentPreferredActivities(filters, components, null)
        assertEquals(1, components.size)
        assertEquals(MainActivity::class.java.name, components.single().className)
        assertTrue(filters.single().hasCategory(Intent.CATEGORY_HOME))
        assertTrue(filters.single().hasCategory(Intent.CATEGORY_DEFAULT))
        assertTrue(filters.single().hasAction(Intent.ACTION_MAIN))
    }

    @Test
    fun preparingSuspendsTheOtherLauncherAndSettingsButNotOurPackage() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        assertTrue(manager.isPackageSuspended(admin, FAKE_LAUNCHER))
        assertTrue(manager.isPackageSuspended(admin, SETTINGS))
        assertFalse(manager.isPackageSuspended(admin, context.packageName))
        assertEquals(setOf(FAKE_LAUNCHER, SETTINGS), prefs.getStringSet("suspended", emptySet()))
    }

    @Test
    fun preparingPersistsTheKioskFlag() {
        shadowOf(manager).setDeviceOwner(admin)
        assertFalse(prefs.getBoolean("enabled", false))
        controller.prepare()
        assertTrue(prefs.getBoolean("enabled", false))
    }

    @Test
    fun preparingAgainKeepsTheStoredSuspensionWhenTheLauncherNoLongerResolves() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        shadowOf(context.packageManager).removePackage(SETTINGS)
        shadowOf(context.packageManager).removeResolveInfosForIntent(homeIntent(), FAKE_LAUNCHER)
        controller.prepare()
        assertEquals(setOf(FAKE_LAUNCHER, SETTINGS), prefs.getStringSet("suspended", emptySet()))
    }

    @Test
    fun withoutTheOwnerRoleNoPolicyIsAppliedAndTheStateIsNotOwner() {
        controller.prepare()
        assertEquals(KioskState.NotOwner, controller.state.value)
        assertFalse(prefs.getBoolean("enabled", false))
        assertTrue(prefs.getStringSet("suspended", null).isNullOrEmpty())
        // Becoming the owner afterwards shows that nothing was recorded before.
        shadowOf(manager).setDeviceOwner(admin)
        assertEquals(0, manager.getLockTaskPackages(admin).size)
        assertFalse(manager.isPackageSuspended(admin, SETTINGS))
    }

    @Test
    fun relockStartsLockTaskForTheOwnerWithTheFlagWhenTheModeIsNone() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        val activity = Robolectric.buildActivity(CountingActivity::class.java).setup().get()
        controller.relock(activity)
        assertEquals(1, activity.lockTaskStarts)
    }

    @Test
    fun relockDoesNothingWithoutTheKioskFlag() {
        shadowOf(manager).setDeviceOwner(admin)
        manager.setLockTaskPackages(admin, arrayOf(context.packageName))
        val activity = Robolectric.buildActivity(CountingActivity::class.java).setup().get()
        controller.relock(activity)
        assertEquals(0, activity.lockTaskStarts)
        assertEquals(KioskState.Prepared, controller.state.value)
    }

    @Test
    fun relockDoesNothingWithoutTheOwnerRole() {
        prefs.edit().putBoolean("enabled", true).commit()
        val activity = Robolectric.buildActivity(CountingActivity::class.java).setup().get()
        controller.relock(activity)
        assertEquals(0, activity.lockTaskStarts)
        assertEquals(KioskState.NotOwner, controller.state.value)
    }

    @Test
    fun relockDoesNotStartAgainWhenTheModeIsAlreadyLocked() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        shadowOf(context.getSystemService(ActivityManager::class.java)).setLockTaskModeState(ActivityManager.LOCK_TASK_MODE_LOCKED)
        val activity = Robolectric.buildActivity(CountingActivity::class.java).setup().get()
        controller.relock(activity)
        assertEquals(0, activity.lockTaskStarts)
        assertEquals(KioskState.Locked, controller.state.value)
    }

    @Test
    fun refreshStateFollowsTheLockTaskMode() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        val activity = Robolectric.buildActivity(CountingActivity::class.java).setup().get()
        controller.refreshState(activity)
        assertEquals(KioskState.Prepared, controller.state.value)
        shadowOf(context.getSystemService(ActivityManager::class.java)).setLockTaskModeState(ActivityManager.LOCK_TASK_MODE_LOCKED)
        controller.refreshState(activity)
        assertEquals(KioskState.Locked, controller.state.value)
    }

    @Test
    fun releaseRevertsPackagesSuspensionPreferredHomeAndFlag() {
        shadowOf(manager).setDeviceOwner(admin)
        controller.prepare()
        controller.release()
        assertEquals(0, manager.getLockTaskPackages(admin).size)
        assertFalse(manager.isPackageSuspended(admin, FAKE_LAUNCHER))
        assertFalse(manager.isPackageSuspended(admin, SETTINGS))
        val components = mutableListOf<android.content.ComponentName>()
        shadowOf(context.packageManager).getPersistentPreferredActivities(mutableListOf(), components, null)
        assertTrue(components.isEmpty())
        assertFalse(prefs.getBoolean("enabled", true))
        assertTrue(prefs.getStringSet("suspended", null).isNullOrEmpty())
        assertEquals(KioskState.NotOwner, controller.state.value)
    }

    @Test
    fun releaseWithoutTheOwnerRoleOnlyClearsTheFlag() {
        prefs.edit().putBoolean("enabled", true).commit()
        controller.release()
        assertFalse(prefs.getBoolean("enabled", true))
    }

    private fun homeIntent(): Intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME)

    private companion object {
        const val FAKE_LAUNCHER = "com.fake.launcher"
        const val SETTINGS = "com.android.settings"
    }
}

/** Counts the Lock Task entries the controller asks for. */
class CountingActivity : Activity() {
    var lockTaskStarts: Int = 0

    override fun startLockTask() {
        lockTaskStarts += 1
    }
}
