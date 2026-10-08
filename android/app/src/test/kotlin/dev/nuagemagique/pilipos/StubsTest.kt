package dev.nuagemagique.pilipos

import android.app.Service
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineService
import dev.nuagemagique.pilipos.engine.LaunchConfig
import dev.nuagemagique.pilipos.engine.LaunchResult
import dev.nuagemagique.pilipos.platform.update.InstallResultReceiver
import dev.nuagemagique.pilipos.platform.update.PackageReplacedReceiver
import dev.nuagemagique.pilipos.platform.watchdog.BootReceiver
import dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver
import dev.nuagemagique.pilipos.provisioning.ProvisioningStore
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import java.io.File

@RunWith(RobolectricTestRunner::class)
class StubsTest {
    private val context: Context = ApplicationProvider.getApplicationContext()

    @Test
    fun engineServiceCreatesStartsAndDestroys() {
        val controller = Robolectric.buildService(EngineService::class.java, Intent(context, EngineService::class.java))
        val service = controller.create().get()
        assertNull(service.onBind(Intent()))
        controller.startCommand(0, 1)
        assertEquals(Service.START_NOT_STICKY, service.onStartCommand(Intent(), 0, 2))
        controller.destroy()
    }

    @Test
    fun platformReceiversHandleAnIntent() {
        val intent = Intent(Intent.ACTION_BOOT_COMPLETED)
        InstallResultReceiver().onReceive(context, intent)
        PackageReplacedReceiver().onReceive(context, Intent(Intent.ACTION_MY_PACKAGE_REPLACED))
        BootReceiver().onReceive(context, intent)
    }

    @Test
    fun adminReceiverHandlesAnIntent() {
        PiliAdminReceiver().onReceive(context, Intent("android.app.action.DEVICE_ADMIN_ENABLED"))
    }

    @Test
    fun provisioningStoreIsNotProvisionedYet() =
        runTest {
            assertEquals(LaunchResult.NotProvisioned, ProvisioningStore().launchConfig())
        }

    @Test
    fun launchResultsCarryTheirData() {
        val config = LaunchConfig(File("/data"), File("/data/bundle"), "device-1", File("/data/logs"))
        assertEquals(config, (LaunchResult.Ready(config) as LaunchResult.Ready).config)
        assertEquals("a", LaunchResult.DeviceMismatch(stored = "a", actual = "b").stored)
        assertEquals("no id", LaunchResult.DeviceIdUnavailable("no id").detail)
    }
}
