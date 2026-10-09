package dev.nuagemagique.pilipos.platform.update

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
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

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class UpdateCompletionHookTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private var now = UPDATE_NOW
    private var running = 102L
    private var sha1: String? = "f".repeat(40)
    private lateinit var markers: UpdateMarkerStore
    private lateinit var audit: PlatformAudit
    private val platformDir get() = File(tmp.root, "platform")

    @Before
    fun setUp() {
        audit = PlatformAudit(platformDir, { now }, UnconfinedTestDispatcher(), { running to "t" }, { 1 })
        markers = UpdateMarkerStore(File(platformDir, "update"))
    }

    private fun hook() =
        UpdateCompletionHook(
            markers = markers,
            audit = audit,
            identity = { sha1 },
            versionCode = { running },
            clock = { now },
            scope = CoroutineScope(Dispatchers.Unconfined),
            ioDispatcher = UnconfinedTestDispatcher(),
        )

    private fun marker(startedAt: Long = UPDATE_NOW - 60_000) = UpdateMarker(101, 102, "1.2.0", "a".repeat(40), startedAt)

    private fun events(): List<Pair<String, JSONObject>> {
        val file = File(platformDir, "audit.jsonl")
        if (!file.exists()) return emptyList()
        return file.readLines().map { JSONObject(it) }.map { it.getString("event") to it.getJSONObject("detail") }
    }

    @Test
    fun theNewVersionWritesDoneWithTheInstalledSha1AndDeletesTheMarker() =
        runTest {
            markers.write(marker())
            hook().complete()

            val (event, detail) = events().single()
            assertEquals("done", event)
            assertEquals("f".repeat(40), detail.getString("apkSha1"))
            assertEquals("a".repeat(40), detail.getString("declaredSha1"))
            assertEquals("101", detail.getString("from"))
            assertEquals("102", detail.getString("to"))
            assertEquals("1.2.0", detail.getString("version"))
            assertFalse(markers.exists())
        }

    @Test
    fun doneIsWrittenEvenWhenTheShaIsUnavailable() =
        runTest {
            sha1 = null
            markers.write(marker())
            hook().complete()
            assertEquals("", events().single().second.getString("apkSha1"))
            assertFalse(markers.exists())
        }

    @Test
    fun theOldVersionWithAnOldMarkerIsNotCompleted() =
        runTest {
            running = 101
            markers.write(marker(startedAt = UPDATE_NOW - 5 * 60_000 - 1))
            hook().complete()

            val (event, detail) = events().single()
            assertEquals("not_completed", event)
            assertEquals("still_old_version", detail.getString("reason"))
            assertFalse(markers.exists())
        }

    @Test
    fun theOldVersionWithAYoungMarkerIsLeftAlone() =
        runTest {
            running = 101
            markers.write(marker(startedAt = UPDATE_NOW - 5 * 60_000))
            hook().complete()

            assertTrue(events().isEmpty())
            assertTrue(markers.exists())
        }

    @Test
    fun anUnexpectedVersionIsNotCompleted() =
        runTest {
            running = 150
            markers.write(marker())
            hook().complete()

            val (event, detail) = events().single()
            assertEquals("not_completed", event)
            assertEquals("unexpected_version", detail.getString("reason"))
            assertFalse(markers.exists())
        }

    @Test
    fun aCorruptMarkerIsAuditedAndDeleted() =
        runTest {
            File(platformDir, "update").mkdirs()
            File(platformDir, "update/pending.json").writeText("{ truncated")
            hook().complete()

            assertEquals("marker_corrupt", events().single().first)
            assertFalse(markers.exists())
        }

    @Test
    fun noMarkerWritesNothing() =
        runTest {
            hook().complete()
            assertTrue(events().isEmpty())
        }

    @Test
    fun aFailingVersionLookupDoesNotThrow() =
        runTest {
            markers.write(marker())
            val broken =
                UpdateCompletionHook(
                    markers = markers,
                    audit = audit,
                    identity = { sha1 },
                    versionCode = { error("no package") },
                    clock = { now },
                    scope = CoroutineScope(Dispatchers.Unconfined),
                    ioDispatcher = UnconfinedTestDispatcher(),
                )
            broken.complete()
            assertTrue(markers.exists())
        }

    @Test
    fun applicationCreatedRunsTheCheck() {
        markers.write(marker())
        hook().onApplicationCreated(ApplicationProvider.getApplicationContext<Application>())
        assertEquals("done", events().single().first)
    }

    @Test
    fun theHookRunsAfterThePolicyAndTheWatchdog() {
        assertEquals(300, hook().order)
    }
}
