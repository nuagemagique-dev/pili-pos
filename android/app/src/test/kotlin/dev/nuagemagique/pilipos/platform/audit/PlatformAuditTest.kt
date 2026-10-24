package dev.nuagemagique.pilipos.platform.audit

import android.content.Context
import android.os.Process
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File
import java.time.Instant

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class PlatformAuditTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private lateinit var dir: File
    private var now = Instant.parse("2026-01-15T10:00:00Z").toEpochMilli()

    @Before
    fun setUp() {
        dir = File(tmp.root, "platform")
    }

    private fun audit(
        ioDispatcher: kotlinx.coroutines.CoroutineDispatcher = UnconfinedTestDispatcher(),
        pid: Int = 4242,
    ) = PlatformAudit(dir, { now }, ioDispatcher, { 17L to "1.2.3" }, { pid })

    private fun lines(file: File) = file.readLines().map { JSONObject(it) }

    @Test
    fun aLineCarriesEveryField() =
        runTest {
            audit().append("watchdog", "fatal", mapOf("reason" to "panic: x\nline \"two\""))
            val file = File(dir, "audit.jsonl")
            assertEquals(1, file.readLines().size)
            val json = lines(file).single()
            assertEquals("2026-01-15T11:00:00.000+01:00", json.getString("ts"))
            assertEquals("watchdog", json.getString("kind"))
            assertEquals("fatal", json.getString("event"))
            assertEquals("panic: x\nline \"two\"", json.getJSONObject("detail").getString("reason"))
            assertEquals(17L, json.getLong("versionCode"))
            assertEquals("1.2.3", json.getString("versionName"))
            assertEquals(4242, json.getInt("pid"))
            assertTrue(file.readText().endsWith("}\n"))
        }

    @Test
    fun theDetailDefaultsToAnEmptyObject() =
        runTest {
            audit().append("boot", "completed")
            assertEquals(0, lines(File(dir, "audit.jsonl")).single().getJSONObject("detail").length())
        }

    @Test
    fun theTimestampUsesTheBrusselsOffsetInWinterAndSummer() {
        assertEquals("2026-01-15T11:00:00.000+01:00", PlatformAudit.format(Instant.parse("2026-01-15T10:00:00Z").toEpochMilli()))
        assertEquals("2026-07-15T12:00:00.000+02:00", PlatformAudit.format(Instant.parse("2026-07-15T10:00:00Z").toEpochMilli()))
        assertEquals("2026-03-29T01:59:59.999+01:00", PlatformAudit.format(Instant.parse("2026-03-29T00:59:59.999Z").toEpochMilli()))
        assertEquals("2026-03-29T03:00:00.000+02:00", PlatformAudit.format(Instant.parse("2026-03-29T01:00:00Z").toEpochMilli()))
    }

    @Test
    fun theRuntimeZoneDoesNotMatter() {
        val saved = java.util.TimeZone.getDefault()
        try {
            java.util.TimeZone.setDefault(java.util.TimeZone.getTimeZone("Asia/Tokyo"))
            assertEquals("2026-07-15T12:00:00.000+02:00", PlatformAudit.format(Instant.parse("2026-07-15T10:00:00Z").toEpochMilli()))
        } finally {
            java.util.TimeZone.setDefault(saved)
        }
    }

    @Test
    fun linesAreAppended() =
        runTest {
            val audit = audit()
            audit.append("a", "one")
            audit.append("a", "two")
            assertEquals(listOf("one", "two"), lines(File(dir, "audit.jsonl")).map { it.getString("event") })
        }

    @Test
    fun theFileRotatesAtOneMiBAndKeepsFive() =
        runTest {
            val audit = audit()
            val big = "x".repeat(PlatformAudit.MAX_BYTES.toInt())
            for (i in 1..7) audit.append("big", "n$i", mapOf("pad" to big))
            assertFalse(File(dir, "audit.jsonl").exists())
            for (i in 1..5) assertTrue("audit.jsonl.$i", File(dir, "audit.jsonl.$i").isFile)
            assertFalse(File(dir, "audit.jsonl.6").exists())
            assertEquals("n7", lines(File(dir, "audit.jsonl.1")).single().getString("event"))
            assertEquals("n3", lines(File(dir, "audit.jsonl.5")).single().getString("event"))
            audit.append("small", "after")
            assertEquals("after", lines(File(dir, "audit.jsonl")).single().getString("event"))
        }

    @Test
    fun belowOneMiBTheFileIsNotRotated() =
        runTest {
            val audit = audit()
            audit.append("big", "a", mapOf("pad" to "x".repeat(500_000)))
            audit.append("big", "b", mapOf("pad" to "x".repeat(400_000)))
            assertTrue(File(dir, "audit.jsonl").length() < PlatformAudit.MAX_BYTES)
            assertFalse(File(dir, "audit.jsonl.1").exists())
            audit.append("big", "c", mapOf("pad" to "x".repeat(200_000)))
            assertTrue(File(dir, "audit.jsonl.1").isFile)
            assertEquals(listOf("a", "b", "c"), lines(File(dir, "audit.jsonl.1")).map { it.getString("event") })
        }

    @Test
    fun filesListsTheCurrentOneFirstThenTheRotatedOnes() =
        runTest {
            val audit = audit()
            assertTrue(audit.files().isEmpty())
            val big = "x".repeat(PlatformAudit.MAX_BYTES.toInt())
            audit.append("big", "a", mapOf("pad" to big))
            audit.append("big", "b", mapOf("pad" to big))
            audit.append("small", "c")
            assertEquals(listOf("audit.jsonl", "audit.jsonl.1", "audit.jsonl.2"), audit.files().map { it.name })
        }

    @Test
    fun concurrentAppendsKeepWholeLines() {
        val audit = audit(ioDispatcher = Dispatchers.IO)
        runBlocking {
            (1..60)
                .map {
                        i ->
                    async(Dispatchers.Default) { audit.append("c", "e$i", mapOf("i" to i.toString(), "pad" to "y".repeat(2_000))) }
                }
                .awaitAll()
        }
        val events = lines(File(dir, "audit.jsonl")).map { it.getString("event") }
        assertEquals(60, events.size)
        assertEquals((1..60).map { "e$it" }.toSet(), events.toSet())
    }

    @Test
    fun anAppendNeverThrows() =
        runTest {
            // the log directory cannot be created: a plain file stands where it should be
            tmp.newFile("platform")
            audit().append("kind", "event")
            assertFalse(File(dir, "audit.jsonl").exists())
        }

    @Test
    fun theInjectedConstructorWritesUnderFilesDirWithTheRealVersionAndPid() =
        runTest {
            val context = ApplicationProvider.getApplicationContext<Context>()
            val audit = PlatformAudit(context, UnconfinedTestDispatcher())
            audit.append("test", "real")
            val file = File(File(context.filesDir, "platform"), "audit.jsonl")
            val json = lines(file).last()
            assertEquals(Process.myPid(), json.getInt("pid"))
            assertTrue(json.getLong("versionCode") >= 0)
            assertTrue(json.getString("versionName").isNotEmpty())
            assertTrue(json.getString("ts").matches(Regex("""\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}[+-]\d{2}:\d{2}""")))
            assertEquals(file, audit.files().first())
            file.delete()
        }

    @Test
    fun theSystemClockIsTheWallClock() {
        val before = System.currentTimeMillis()
        val read = SystemPlatformClock.nowMillis()
        assertTrue(read >= before)
    }
}
