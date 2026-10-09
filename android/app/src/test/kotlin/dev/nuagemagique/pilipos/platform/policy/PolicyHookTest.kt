package dev.nuagemagique.pilipos.platform.policy

import android.app.Activity
import android.app.Application
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import dev.nuagemagique.pilipos.provisioning.ProvisioningRecord
import io.mockk.mockk
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class PolicyHookTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val app = mockk<Application>()
    private val activity = mockk<Activity>()
    private val record = ProvisioningRecord("serial", "sha", "comment", "2026-10-09T00:00:00Z")

    private class Rig(
        val fake: FakeDevicePolicies,
        val state: MutableStateFlow<ProvisioningRecord?>,
        val hook: PolicyHook,
        val dir: File,
    ) {
        var refreshes = 0

        /** The audit rows as `event` plus their detail map. */
        fun rows(): List<Pair<String, JSONObject>> {
            val file = File(dir, "audit.jsonl")
            if (!file.exists()) return emptyList()
            return file.readLines().map { JSONObject(it) }.also { rows ->
                rows.forEach { assertEquals("policy", it.getString("kind")) }
            }.map { it.getString("event") to it.getJSONObject("detail") }
        }
    }

    private fun rig(
        scope: TestScope,
        release: Boolean,
        recorded: Boolean,
        fake: FakeDevicePolicies = FakeDevicePolicies(),
    ): Rig {
        val dir = File(tmp.root, "platform")
        val io = UnconfinedTestDispatcher(scope.testScheduler)
        val audit = PlatformAudit(dir, PlatformClock { 1_800_000_000_000L }, io, { 1L to "t" }, { 1 })
        val state = MutableStateFlow(if (recorded) record else null)
        lateinit var rig: Rig
        val hook =
            PolicyHook(
                applier = PolicyApplier(fake, "pkg"),
                audit = audit,
                record = state,
                refreshRecord = { rig.refreshes++ },
                release = release,
                scope = scope.backgroundScope,
                ioDispatcher = io,
            )
        rig = Rig(fake, state, hook, dir)
        return rig
    }

    @Test
    fun theHookRunsAt100() {
        runTest {
            assertEquals(100, rig(this, release = false, recorded = false).hook.order)
        }
    }

    @Test
    fun applicationStartAppliesTheSetOnceAndWritesOneSummaryRow() =
        runTest {
            val rig = rig(this, release = true, recorded = true)
            rig.hook.onApplicationCreated(app)
            runCurrent()
            assertEquals(1, rig.refreshes)
            assertEquals(1, rig.fake.calls.count { it == "setTimeZone(Europe/Brussels)" })
            assertEquals("addUserRestriction(no_debugging_features)", rig.fake.calls.last { it.startsWith("addUser") })
            val rows = rig.rows()
            assertEquals(listOf("applied"), rows.map { it.first })
            val detail = rows.single().second
            assertEquals("start", detail.getString("trigger"))
            assertEquals("", detail.getString("failed"))
            assertEquals((3 + PolicySet.RESTRICTIONS.size + 6 + 1).toString(), detail.getString("ok"))
        }

    @Test
    fun debugBuildNeverAppliesAdbOff() =
        runTest {
            val rig = rig(this, release = false, recorded = true)
            rig.hook.onApplicationCreated(app)
            runCurrent()
            assertTrue(rig.fake.calls.none { it.contains("no_debugging_features") })
        }

    @Test
    fun notOwnerWritesASingleNotOwnerRowAndCallsNothing() =
        runTest {
            val rig = rig(this, release = true, recorded = true, fake = FakeDevicePolicies(owner = false))
            rig.hook.onApplicationCreated(app)
            rig.hook.onActivityCreated(activity)
            rig.hook.onActivityResumed(activity)
            runCurrent()
            assertTrue(rig.fake.calls.isEmpty())
            assertEquals(listOf("not_owner"), rig.rows().map { it.first })
        }

    @Test
    fun aReleaseBuildWithoutRecordAppliesAdbOffWhenTheRecordAppears() =
        runTest {
            val rig = rig(this, release = true, recorded = false)
            rig.hook.onApplicationCreated(app)
            runCurrent()
            assertTrue(rig.fake.calls.none { it.contains("no_debugging_features") })
            rig.state.value = record
            runCurrent()
            assertEquals(1, rig.fake.calls.count { it == "addUserRestriction(no_debugging_features)" })
            assertEquals(listOf("start", "provisioned"), rig.rows().map { it.second.getString("trigger") })
        }

    @Test
    fun aDebugBuildDoesNotWaitForTheRecord() =
        runTest {
            val rig = rig(this, release = false, recorded = false)
            rig.hook.onApplicationCreated(app)
            runCurrent()
            rig.state.value = record
            runCurrent()
            assertEquals(1, rig.rows().size)
            assertEquals(1, rig.fake.calls.count { it == "setTimeZone(Europe/Brussels)" })
        }

    @Test
    fun lockTaskFeaturesAreAppliedOnEveryActivityCreateAndResume() =
        runTest {
            val rig = rig(this, release = false, recorded = true)
            rig.hook.onApplicationCreated(app)
            runCurrent()
            val before = rig.fake.calls.count { it == "setLockTaskFeatures(0)" }
            rig.hook.onActivityCreated(activity)
            assertEquals(before + 1, rig.fake.calls.count { it == "setLockTaskFeatures(0)" })
            rig.hook.onActivityResumed(activity)
            assertEquals(before + 2, rig.fake.calls.count { it == "setLockTaskFeatures(0)" })
            rig.hook.onActivityResumed(activity)
            assertEquals(before + 3, rig.fake.calls.count { it == "setLockTaskFeatures(0)" })
            // Only the setter ran again (the previous application had no failure), and nothing was audited.
            assertEquals(1, rig.fake.calls.count { it == "setTimeZone(Europe/Brussels)" })
            assertEquals(1, rig.rows().size)
        }

    @Test
    fun lockTaskFeaturesAreAppliedEvenBeforeTheApplicationRunFinished() =
        runTest {
            val rig = rig(this, release = false, recorded = true)
            rig.hook.onActivityCreated(activity)
            assertEquals(listOf("setLockTaskFeatures(0)"), rig.fake.calls)
        }

    @Test
    fun aFailedStepIsAuditedAndTheOthersStillRun() =
        runTest {
            val fake = FakeDevicePolicies(failing = mutableSetOf("addUserRestriction(no_user_switch)", "setStatusBarDisabled"))
            val rig = rig(this, release = true, recorded = true, fake = fake)
            rig.hook.onApplicationCreated(app)
            runCurrent()
            val rows = rig.rows()
            assertEquals(listOf("step_failed", "step_failed", "applied"), rows.map { it.first })
            assertEquals(listOf("restriction:no_user_switch", "status_bar_off"), rows.take(2).map { it.second.getString("step") })
            assertTrue(rows[0].second.getString("error").startsWith("SecurityException"))
            assertEquals("restriction:no_user_switch,status_bar_off", rows.last().second.getString("failed"))
            assertEquals("addUserRestriction(no_debugging_features)", fake.calls.last { it.startsWith("addUser") })
        }

    @Test
    fun aResumeAfterAFailedRunAppliesTheWholeSetAgain() =
        runTest {
            val fake = FakeDevicePolicies(failing = mutableSetOf("setStatusBarDisabled"))
            val rig = rig(this, release = false, recorded = true, fake = fake)
            rig.hook.onApplicationCreated(app)
            runCurrent()
            assertEquals(1, fake.calls.count { it == "setTimeZone(Europe/Brussels)" })
            fake.failing.clear()
            rig.hook.onActivityResumed(activity)
            runCurrent()
            assertEquals(2, fake.calls.count { it == "setTimeZone(Europe/Brussels)" })
            // The retry succeeded: the next resume only sets the lock task features.
            rig.hook.onActivityResumed(activity)
            runCurrent()
            assertEquals(2, fake.calls.count { it == "setTimeZone(Europe/Brussels)" })
            // The first run audited its failure and its summary; the clean retry wrote nothing.
            assertEquals(listOf("step_failed", "applied"), rig.rows().map { it.first })
        }

    @Test
    fun aFailedLockTaskStepOnResumeIsAudited() =
        runTest {
            val fake = FakeDevicePolicies(failing = mutableSetOf("setLockTaskFeatures"))
            val rig = rig(this, release = false, recorded = true, fake = fake)
            rig.hook.onActivityResumed(activity)
            runCurrent()
            assertEquals(listOf("step_failed"), rig.rows().map { it.first })
            assertEquals("lock_task_features", rig.rows().single().second.getString("step"))
        }
}
