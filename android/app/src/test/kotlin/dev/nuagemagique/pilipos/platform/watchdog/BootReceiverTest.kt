package dev.nuagemagique.pilipos.platform.watchdog

import android.app.Application
import android.content.Context
import android.content.Intent
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class BootReceiverTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val app = ApplicationProvider.getApplicationContext<Application>()
    private val dir get() = File(tmp.root, "platform")

    private fun entryPoint(): BootEntryPoint {
        val audit = PlatformAudit(dir, { 1_000L }, UnconfinedTestDispatcher(), { 1L to "t" }, { 1 })
        val scope = CoroutineScope(Dispatchers.Unconfined)
        return object : BootEntryPoint {
            override fun platformAudit() = audit

            override fun applicationScope() = scope
        }
    }

    private fun auditEvents(): List<String> {
        val file = File(dir, "audit.jsonl")
        if (!file.exists()) return emptyList()
        return file.readLines().map { JSONObject(it).let { j -> "${j.getString("kind")}/${j.getString("event")}" } }
    }

    @Test
    fun bootCompletedIsAuditedAndStartsTheMainActivity() {
        val receiver = BootReceiver().apply { resolver = { entryPoint() } }
        receiver.onReceive(app, Intent(Intent.ACTION_BOOT_COMPLETED))
        assertEquals(listOf("boot/completed"), auditEvents())
        val started = shadowOf(app).nextStartedActivity
        assertNotNull(started)
        assertEquals(MainActivity::class.java.name, started.component?.className)
        assertTrue(started.flags and Intent.FLAG_ACTIVITY_NEW_TASK != 0)
    }

    @Test
    fun anotherActionIsIgnored() {
        val receiver = BootReceiver().apply { resolver = { entryPoint() } }
        receiver.onReceive(app, Intent(Intent.ACTION_MY_PACKAGE_REPLACED))
        receiver.onReceive(app, Intent("com.example.OTHER"))
        assertEquals(emptyList<String>(), auditEvents())
        assertNull(shadowOf(app).nextStartedActivity)
    }

    @Test
    fun withoutAHiltGraphTheActivityIsStillStartedAndNothingIsThrown() {
        BootReceiver().onReceive(app, Intent(Intent.ACTION_BOOT_COMPLETED))
        assertNotNull(shadowOf(app).nextStartedActivity)
    }

    @Test
    fun aFailingLookupIsSwallowed() {
        val receiver = BootReceiver().apply { resolver = { error("no graph") } }
        receiver.onReceive(app, Intent(Intent.ACTION_BOOT_COMPLETED))
        assertEquals(emptyList<String>(), auditEvents())
        assertNotNull(shadowOf(app).nextStartedActivity)
    }

    @Test
    fun aFailingActivityStartIsSwallowed() {
        val context = mockk<Context>(relaxed = true)
        every { context.packageName } returns "dev.nuagemagique.pilipos"
        every { context.startActivity(any()) } throws SecurityException("background start")
        BootReceiver().apply { resolver = { entryPoint() } }.onReceive(context, Intent(Intent.ACTION_BOOT_COMPLETED))
        assertEquals(listOf("boot/completed"), auditEvents())
    }
}
