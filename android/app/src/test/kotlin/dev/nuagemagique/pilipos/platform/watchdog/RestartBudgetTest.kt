package dev.nuagemagique.pilipos.platform.watchdog

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@RunWith(RobolectricTestRunner::class)
class RestartBudgetTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val minute = 60_000L
    private val file get() = File(tmp.root, "platform/restarts.json")

    @Test
    fun anEmptyBudgetAllowsAndHasNoLastRestart() {
        val budget = RestartBudget(file)
        assertTrue(budget.allow(1_000_000))
        assertNull(budget.lastRestart())
    }

    @Test
    fun threeRestartsInTenMinutesExhaustTheBudget() {
        val budget = RestartBudget(file)
        val t0 = 100 * minute
        budget.record(t0)
        budget.record(t0 + minute)
        assertTrue(budget.allow(t0 + 2 * minute))
        budget.record(t0 + 2 * minute)
        assertFalse(budget.allow(t0 + 3 * minute))
        assertFalse(budget.allow(t0 + 9 * minute))
        // the first one leaves the window exactly ten minutes after it
        assertTrue(budget.allow(t0 + 10 * minute))
    }

    @Test
    fun theBudgetSurvivesANewInstance() {
        val first = RestartBudget(file)
        first.record(5 * minute)
        first.record(6 * minute)
        first.record(7 * minute)
        val second = RestartBudget(file)
        assertFalse(second.allow(8 * minute))
        assertEquals(7 * minute, second.lastRestart())
    }

    @Test
    fun onlyTheLastTenAreKept() {
        val budget = RestartBudget(file)
        for (i in 1..15) budget.record(i * 1_000L)
        val kept = org.json.JSONObject(file.readText()).getJSONArray("restarts")
        assertEquals(10, kept.length())
        assertEquals(6_000L, kept.getLong(0))
        assertEquals(15_000L, kept.getLong(9))
    }

    @Test
    fun aCorruptFileIsAnEmptyBudget() {
        file.parentFile!!.mkdirs()
        file.writeText("{not json")
        val budget = RestartBudget(file)
        assertTrue(budget.allow(1_000_000))
        assertNull(budget.lastRestart())
        budget.record(1_000_000)
        assertEquals(1_000_000L, budget.lastRestart())
    }

    @Test
    fun aFileWithoutTheArrayIsAnEmptyBudget() {
        file.parentFile!!.mkdirs()
        file.writeText("{}")
        assertTrue(RestartBudget(file).allow(1_000_000))
    }

    @Test
    fun restartsInTheFutureDoNotCount() {
        val budget = RestartBudget(file)
        budget.record(1_000 * minute)
        budget.record(1_001 * minute)
        budget.record(1_002 * minute)
        assertTrue(budget.allow(10 * minute))
    }

    @Test
    fun aWriteFailureIsLoggedNotThrown() {
        tmp.newFile("platform")
        RestartBudget(file).record(1_000)
    }

    @Test
    fun theInjectedConstructorUsesFilesDir() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val budget = RestartBudget(context)
        budget.record(123_456)
        val stored = File(File(context.filesDir, "platform"), "restarts.json")
        assertTrue(stored.isFile)
        assertEquals(123_456L, budget.lastRestart())
        stored.delete()
    }
}
