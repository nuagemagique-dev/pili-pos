package dev.nuagemagique.pilipos.platform.watchdog

import android.app.Application
import android.os.Process
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineService
import dev.nuagemagique.pilipos.platform.audit.SystemPlatformClock
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.shadows.ShadowProcess

@RunWith(RobolectricTestRunner::class)
class WatchdogSeamsTest {
    private val app = ApplicationProvider.getApplicationContext<Application>()

    @Test
    fun theProductionKillerEndsTheOwnProcess() {
        ShadowProcess.clearKilledProcesses()
        AndroidProcessKiller().kill()
        assertTrue(ShadowProcess.wasKilled(Process.myPid()))
    }

    @Test
    fun theProductionStarterStartsTheEngineService() {
        EngineServiceStarter(app).start()
        assertEquals(EngineService::class.java.name, shadowOf(app).nextStartedService.component?.className)
    }

    @Test
    fun theModuleProvidesTheSystemClock() {
        assertSame(SystemPlatformClock, WatchdogModule.providePlatformClock())
    }
}
