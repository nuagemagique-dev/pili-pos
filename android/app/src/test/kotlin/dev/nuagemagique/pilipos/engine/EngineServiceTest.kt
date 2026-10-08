package dev.nuagemagique.pilipos.engine

import android.app.NotificationManager
import android.app.Service
import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dagger.hilt.android.testing.BindValue
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dagger.hilt.android.testing.UninstallModules
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.di.CoroutinesModule
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.provisioning.ProvisioningModule
import dev.nuagemagique.pilipos.provisioning.ProvisioningNatives
import dev.nuagemagique.pilipos.provisioning.TreeReader
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import javax.inject.Inject

@OptIn(ExperimentalCoroutinesApi::class)
@HiltAndroidTest
@UninstallModules(EngineModule::class, ProvisioningModule::class, CoroutinesModule::class)
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class EngineServiceTest {
    @get:Rule(order = 0)
    val hiltRule = HiltAndroidRule(this)

    @get:Rule(order = 1)
    val tmp = TemporaryFolder()

    private val log = CallLog()
    private val engine = mockEngine(log)
    private val dispatcher = UnconfinedTestDispatcher()

    @BindValue
    @JvmField
    val source: LaunchConfigSource = FakeLaunchSource(LaunchResult.NotProvisioned)

    @BindValue
    @JvmField
    val natives: EngineNatives = FakeNatives(log)

    @BindValue
    @JvmField
    val factory: EngineFactory = FakeFactory(log, engine)

    // The shell's ProvisioningCoordinator reaches BundleImporter, which needs these two seams of the
    // uninstalled ProvisioningModule.
    @BindValue
    @JvmField
    val provisioningNatives: ProvisioningNatives = mockk()

    @BindValue
    @JvmField
    val treeReader: TreeReader = mockk()

    @BindValue
    @JvmField
    @IoDispatcher
    val io: CoroutineDispatcher = dispatcher

    @BindValue
    @JvmField
    @ApplicationScope
    val appScope: CoroutineScope = CoroutineScope(SupervisorJob() + dispatcher)

    @Inject
    lateinit var holder: EngineHolder

    private val context: Context = ApplicationProvider.getApplicationContext()

    @Before
    fun setUp() {
        hiltRule.inject()
    }

    @Test
    fun onCreatePostsTheForegroundNotificationOnTheEngineChannel() {
        val controller = Robolectric.buildService(EngineService::class.java)
        val service = controller.create().get()
        val shadow = shadowOf(service)
        assertEquals(1, shadow.lastForegroundNotificationId)
        val notification = checkNotNull(shadow.lastForegroundNotification)
        assertEquals("engine", notification.channelId)
        assertEquals(context.getString(R.string.engine_notification_title), notification.extras.getString("android.title"))
        assertEquals(context.getString(R.string.engine_notification_text), notification.extras.getString("android.text"))
        assertTrue(notification.flags and android.app.Notification.FLAG_ONGOING_EVENT != 0)
        val manager = context.getSystemService(NotificationManager::class.java)
        val channel = manager.getNotificationChannel("engine")
        assertNotNull(channel)
        assertEquals(NotificationManager.IMPORTANCE_LOW, channel.importance)
        assertEquals(context.getString(R.string.engine_channel_name), channel.name.toString())
        assertNull(service.onBind(android.content.Intent()))
    }

    @Test
    fun onStartCommandOpensTheEngineAndIsSticky() {
        (source as FakeLaunchSource).result = readyLaunch(tmp.root)
        val controller = Robolectric.buildService(EngineService::class.java).create()
        val service = controller.get()
        val result = service.onStartCommand(null, 0, 1)
        assertEquals(Service.START_STICKY, result)
        assertEquals(EngineState.Ready("pili-pos-android/test"), holder.state.value)
        controller.destroy()
    }

    @Test
    fun onStartCommandWithoutAConfigLeavesTheEngineClosed() {
        val controller = Robolectric.buildService(EngineService::class.java).create()
        controller.get().onStartCommand(null, 0, 1)
        assertEquals(EngineState.Closed(ClosedReason.NotProvisioned), holder.state.value)
        controller.destroy()
    }

    @Test
    fun onDestroyShutsTheEngineDown() {
        (source as FakeLaunchSource).result = readyLaunch(tmp.root)
        val controller = Robolectric.buildService(EngineService::class.java).create()
        controller.get().onStartCommand(null, 0, 1)
        controller.destroy()
        coVerify(exactly = 1) { engine.shutdown() }
        assertEquals(EngineState.Closed(ClosedReason.Shutdown), holder.state.value)
    }

    @Test
    fun startIssuesAForegroundServiceIntentForTheEngineService() {
        EngineService.start(context)
        val started = shadowOf(context as android.app.Application).nextStartedService
        assertEquals(EngineService::class.java.name, started.component?.className)
    }
}
