package dev.nuagemagique.pilipos.provisioning

import android.Manifest
import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The device-owner role of this app. Importing a bundle and reading the hardware serial both need it
 * (DECISIONS section 3): the till refuses to be provisioned on a device that is not owned.
 */
@Singleton
class DeviceOwner
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
    ) {
        /** The admin component the device policy manager knows this app by. */
        val adminComponent: ComponentName = PiliAdminReceiver.component(context)

        private val policyManager: DevicePolicyManager?
            get() = context.getSystemService(DevicePolicyManager::class.java)

        /** True when this app is the device owner. */
        fun isDeviceOwner(): Boolean = policyManager?.isDeviceOwnerApp(context.packageName) == true

        /**
         * Grants the runtime permissions the app needs without a dialog (the serial needs READ_PHONE_STATE,
         * the foreground service notification needs POST_NOTIFICATIONS; S4 and Q15). Does nothing when the
         * app is not the device owner.
         */
        fun grantOwnPermissions() {
            val manager = policyManager
            if (manager == null || !manager.isDeviceOwnerApp(context.packageName)) {
                Timber.i("provisioning: not the device owner, no permission granted")
                return
            }
            for (permission in OWN_PERMISSIONS) {
                val granted =
                    runCatching {
                        manager.setPermissionGrantState(
                            adminComponent,
                            context.packageName,
                            permission,
                            DevicePolicyManager.PERMISSION_GRANT_STATE_GRANTED,
                        )
                    }.getOrElse {
                        Timber.w("provisioning: granting %s failed (%s)", permission, it.javaClass.simpleName)
                        false
                    }
                Timber.i("provisioning: grant %s -> %s", permission, granted)
            }
        }

        private companion object {
            val OWN_PERMISSIONS =
                listOf(
                    Manifest.permission.READ_PHONE_STATE,
                    Manifest.permission.POST_NOTIFICATIONS,
                )
        }
    }
