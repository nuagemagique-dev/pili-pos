package dev.nuagemagique.pilipos.provisioning

import android.Manifest
import android.app.admin.DevicePolicyManager
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf

@RunWith(RobolectricTestRunner::class)
class DeviceOwnerTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private val manager: DevicePolicyManager = context.getSystemService(DevicePolicyManager::class.java)

    @Test
    fun adminComponentNamesTheReceiver() {
        val owner = DeviceOwner(context)
        assertEquals(PiliAdminReceiver::class.java.name, owner.adminComponent.className)
        assertEquals(context.packageName, owner.adminComponent.packageName)
        assertEquals(owner.adminComponent, PiliAdminReceiver.component(context))
    }

    @Test
    fun isDeviceOwnerIsFalseUntilTheRoleIsGranted() {
        val owner = DeviceOwner(context)
        assertFalse(owner.isDeviceOwner())
        shadowOf(manager).setDeviceOwner(owner.adminComponent)
        assertTrue(owner.isDeviceOwner())
    }

    @Test
    fun grantOwnPermissionsGrantsPhoneStateAndNotificationsToTheOwner() {
        val owner = DeviceOwner(context)
        shadowOf(manager).setDeviceOwner(owner.adminComponent)
        owner.grantOwnPermissions()
        for (permission in listOf(Manifest.permission.READ_PHONE_STATE, Manifest.permission.POST_NOTIFICATIONS)) {
            assertEquals(
                permission,
                DevicePolicyManager.PERMISSION_GRANT_STATE_GRANTED,
                manager.getPermissionGrantState(owner.adminComponent, context.packageName, permission),
            )
        }
    }

    @Test
    fun grantOwnPermissionsDoesNothingWithoutTheRole() {
        val owner = DeviceOwner(context)
        owner.grantOwnPermissions()
        shadowOf(manager).setDeviceOwner(owner.adminComponent)
        assertEquals(
            DevicePolicyManager.PERMISSION_GRANT_STATE_DEFAULT,
            manager.getPermissionGrantState(owner.adminComponent, context.packageName, Manifest.permission.READ_PHONE_STATE),
        )
    }

    @Test
    fun adminReceiverLogsItsCallbacks() {
        val receiver = PiliAdminReceiver()
        receiver.onEnabled(context, Intent(android.app.admin.DeviceAdminReceiver.ACTION_DEVICE_ADMIN_ENABLED))
        receiver.onDisabled(context, Intent(android.app.admin.DeviceAdminReceiver.ACTION_DEVICE_ADMIN_DISABLED))
    }
}
