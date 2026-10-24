package dev.nuagemagique.pilipos.platform

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class PlatformOpsTest {
    @Test
    fun theNameIsBusyWhileTheBlockRunsAndTheValueIsReturned() =
        runTest {
            val ops = PlatformOps()
            assertTrue(ops.busy.value.isEmpty())
            val seen = ops.run("update") { ops.busy.value }
            assertEquals(setOf("update"), seen)
            assertTrue(ops.busy.value.isEmpty())
            assertEquals(7, ops.run("export") { 7 })
        }

    @Test
    fun theNameLeavesTheSetWhenTheBlockThrows() =
        runTest {
            val ops = PlatformOps()
            try {
                ops.run("update") { error("boom") }
                fail("expected the exception")
            } catch (e: IllegalStateException) {
                assertEquals("boom", e.message)
            }
            assertTrue(ops.busy.value.isEmpty())
        }

    @Test
    fun twoRunsOfTheSameNameKeepItBusyUntilTheLastEnds() =
        runTest {
            val ops = PlatformOps()
            val first = CompletableDeferred<Unit>()
            val second = CompletableDeferred<Unit>()
            launch { ops.run("export") { first.await() } }
            launch { ops.run("export") { second.await() } }
            runCurrent()
            assertEquals(setOf("export"), ops.busy.value)
            first.complete(Unit)
            runCurrent()
            assertEquals(setOf("export"), ops.busy.value)
            second.complete(Unit)
            runCurrent()
            assertTrue(ops.busy.value.isEmpty())
        }

    @Test
    fun differentNamesAreIndependent() =
        runTest {
            val ops = PlatformOps()
            val gate = CompletableDeferred<Unit>()
            launch { ops.run("update") { gate.await() } }
            runCurrent()
            val inner = ops.run("export") { ops.busy.value }
            assertEquals(setOf("update", "export"), inner)
            assertEquals(setOf("update"), ops.busy.value)
            gate.complete(Unit)
            runCurrent()
            assertTrue(ops.busy.value.isEmpty())
        }
}
