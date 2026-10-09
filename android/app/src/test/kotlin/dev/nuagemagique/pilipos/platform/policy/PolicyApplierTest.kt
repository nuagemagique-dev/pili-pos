package dev.nuagemagique.pilipos.platform.policy

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PolicyApplierTest {
    private val pkg = "dev.nuagemagique.pilipos"

    private fun rig(fake: FakeDevicePolicies = FakeDevicePolicies()): Pair<PolicyApplier, FakeDevicePolicies> =
        PolicyApplier(fake, pkg) to fake

    private val restrictionCalls =
        PolicySet.RESTRICTIONS.map { "addUserRestriction($it)" }

    @Test
    fun theCallsRunInTheDocumentedOrder() {
        val (applier, fake) = rig()
        val report = applier.apply(release = true, provisioned = true)
        assertEquals(
            listOf(
                "setAutoTimeZoneEnabled(false)",
                "setTimeZone(Europe/Brussels)",
                "setAutoTimeEnabled(true)",
                "getUserRestrictions",
            ) + restrictionCalls +
                listOf(
                    "setUninstallBlocked($pkg,true)",
                    "setGlobalSetting(stay_on_while_plugged_in,7)",
                    "setKeyguardDisabled(true)",
                    "setStatusBarDisabled(true)",
                    "setSystemUpdatePolicyPostpone",
                    "setLockTaskFeatures(0)",
                    "addUserRestriction(no_debugging_features)",
                ),
            fake.calls,
        )
        assertTrue(report.allOk)
        assertFalse(report.notOwner)
        assertEquals(report.steps.size, report.okCount)
    }

    @Test
    fun theAutoZoneIsOffBeforeTheZoneIsSet() {
        val (applier, fake) = rig()
        applier.apply(release = false, provisioned = false)
        assertTrue(fake.calls.indexOf("setAutoTimeZoneEnabled(false)") < fake.calls.indexOf("setTimeZone(Europe/Brussels)"))
    }

    @Test
    fun adbOffIsAppliedOnlyInReleaseWithARecordAndComesLast() {
        for ((release, provisioned) in listOf(false to false, false to true, true to false)) {
            val (applier, fake) = rig()
            applier.apply(release, provisioned)
            assertFalse(
                "release=$release provisioned=$provisioned",
                fake.calls.any { it.contains("no_debugging_features") },
            )
            assertEquals("setLockTaskFeatures(0)", fake.calls.last())
        }
        val (applier, fake) = rig()
        applier.apply(release = true, provisioned = true)
        assertEquals("addUserRestriction(no_debugging_features)", fake.calls.last())
        assertEquals(1, fake.calls.count { it.contains("no_debugging_features") })
    }

    @Test
    fun notOwnerCallsNothing() {
        val (applier, fake) = rig(FakeDevicePolicies(owner = false))
        val report = applier.apply(release = true, provisioned = true)
        assertTrue(report.notOwner)
        assertFalse(report.allOk)
        assertTrue(report.steps.isEmpty())
        assertTrue(fake.calls.isEmpty())
        assertTrue(applier.applyLockTaskFeatures().notOwner)
        assertTrue(fake.calls.isEmpty())
    }

    @Test
    fun aThrowingStepIsReportedAndTheNextStepsStillRun() {
        val (applier, fake) =
            rig(
                FakeDevicePolicies(
                    failing = mutableSetOf("setTimeZone", "addUserRestriction(no_safe_boot)", "setKeyguardDisabled"),
                ),
            )
        val report = applier.apply(release = true, provisioned = true)
        assertEquals(
            listOf("time_zone", "restriction:no_safe_boot", "keyguard_off"),
            report.failed.map { it.name },
        )
        assertTrue(report.failed.all { it.error!!.startsWith("SecurityException: refused") })
        assertFalse(report.allOk)
        // Everything after the failures was still called, adb off included.
        assertEquals("addUserRestriction(no_debugging_features)", fake.calls.last())
        assertTrue("setStatusBarDisabled(true)" in fake.calls)
        assertTrue("addUserRestriction(no_network_reset)" in fake.calls)
    }

    @Test
    fun aRefusedBooleanIsAFailedStep() {
        val (applier, _) = rig(FakeDevicePolicies(refuseBooleans = setOf("setTimeZone", "setStatusBarDisabled")))
        val report = applier.apply(release = false, provisioned = false)
        assertEquals(listOf("time_zone", "status_bar_off"), report.failed.map { it.name })
        assertEquals("IllegalStateException: setTimeZone refused", report.failed.first().error)
    }

    @Test
    fun restrictionsAlreadyPresentAreSkippedAndCountOk() {
        val present = setOf("no_factory_reset", "no_add_user")
        val (applier, fake) = rig(FakeDevicePolicies(present = present))
        val report = applier.apply(release = false, provisioned = false)
        assertFalse(fake.calls.contains("addUserRestriction(no_factory_reset)"))
        assertFalse(fake.calls.contains("addUserRestriction(no_add_user)"))
        assertTrue(fake.calls.contains("addUserRestriction(no_safe_boot)"))
        val skipped = report.steps.filter { it.skipped }
        assertEquals(listOf("restriction:no_factory_reset", "restriction:no_add_user"), skipped.map { it.name })
        assertTrue(skipped.all { it.ok && it.error == null })
        assertTrue(report.allOk)
    }

    @Test
    fun adbOffAlreadyPresentIsNotReapplied() {
        val (applier, fake) = rig(FakeDevicePolicies(present = setOf("no_debugging_features")))
        val report = applier.apply(release = true, provisioned = true)
        assertFalse(fake.calls.contains("addUserRestriction(no_debugging_features)"))
        assertTrue(report.steps.last().skipped)
    }

    @Test
    fun anUnreadableRestrictionListIsAFailedStepAndEverythingIsAdded() {
        val (applier, fake) = rig(FakeDevicePolicies(failing = mutableSetOf("getUserRestrictions")))
        val report = applier.apply(release = false, provisioned = false)
        assertEquals(listOf("restrictions_read"), report.failed.map { it.name })
        assertEquals(PolicySet.RESTRICTIONS.size, fake.calls.count { it.startsWith("addUserRestriction(") })
    }

    @Test
    fun theAbsentRestrictionsAreNeverAdded() {
        val (applier, fake) = rig()
        applier.apply(release = true, provisioned = true)
        applier.applyLockTaskFeatures()
        assertTrue(fake.calls.none { it.contains("no_mount_physical_media") || it.contains("no_config_wifi") })
    }

    @Test
    fun lockTaskFeaturesAloneCallsOnlyThatSetter() {
        val (applier, fake) = rig()
        val report = applier.applyLockTaskFeatures()
        assertEquals(listOf("setLockTaskFeatures(0)"), fake.calls)
        assertTrue(report.allOk)
        assertNull(report.steps.single().error)
        assertEquals(PolicyApplier.LOCK_TASK_STEP, report.steps.single().name)
    }

    @Test
    fun aFailedLockTaskStepIsReported() {
        val (applier, _) = rig(FakeDevicePolicies(failing = mutableSetOf("setLockTaskFeatures")))
        assertEquals(listOf("lock_task_features"), applier.applyLockTaskFeatures().failed.map { it.name })
    }
}
