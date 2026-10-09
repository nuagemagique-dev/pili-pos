package dev.nuagemagique.pilipos.platform.policy

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * One applied step.
 *
 * @property name stable step name (`restriction:<key>` for a restriction)
 * @property ok the step succeeded, or its effect was already in place ([skipped])
 * @property error the failure text when [ok] is false
 * @property skipped the effect was already in place, so nothing was called
 */
data class PolicyStep(
    val name: String,
    val ok: Boolean,
    val error: String? = null,
    val skipped: Boolean = false,
)

/** What one [PolicyApplier] run did. */
data class PolicyReport(
    val steps: List<PolicyStep>,
    val notOwner: Boolean = false,
) {
    /** The steps that failed. */
    val failed: List<PolicyStep> get() = steps.filterNot { it.ok }

    /** The number of steps that succeeded. */
    val okCount: Int get() = steps.count { it.ok }

    /** True when the app is the owner and no step failed. */
    val allOk: Boolean get() = !notOwner && failed.isEmpty()

    companion object {
        /** The app is not the device owner: nothing was applied. */
        val NotOwner = PolicyReport(emptyList(), notOwner = true)
    }
}

/**
 * Applies the device-owner policy set ([PolicySet], PD206).
 *
 * Every step runs in its own try/catch: a refusal is recorded in the [PolicyReport] and the next step still
 * runs. A restriction that `getUserRestrictions` already lists is not added again, to keep starts fast. When
 * the app is not the device owner nothing is called and the report is [PolicyReport.NotOwner].
 *
 * The order is the one proven in S4: the automatic zone off before the zone is set, then the automatic time,
 * the restrictions, the uninstall block, stay awake, keyguard, status bar, the update policy, Lock Task
 * features NONE, and last (release, provisioned) adb off.
 */
@Singleton
class PolicyApplier internal constructor(
    private val policies: DevicePolicies,
    private val packageName: String,
) {
    @Inject
    constructor(
        policies: DevicePolicies,
        @ApplicationContext context: Context,
    ) : this(policies, context.packageName)

    /** Applies the whole set; [release] and [provisioned] decide whether adb is turned off at the end. */
    fun apply(
        release: Boolean,
        provisioned: Boolean,
    ): PolicyReport {
        if (!policies.isDeviceOwner()) return notOwner()
        val steps = ArrayList<PolicyStep>()
        steps += step("auto_time_zone_off") { policies.setAutoTimeZoneEnabled(false) }
        steps += step("time_zone") { check(policies.setTimeZone(PolicySet.TIME_ZONE)) { "setTimeZone refused" } }
        steps += step("auto_time_on") { policies.setAutoTimeEnabled(true) }
        val present = presentRestrictions(steps)
        for (key in PolicySet.RESTRICTIONS) steps += restriction(key, present)
        steps += step("uninstall_block") { policies.setUninstallBlocked(packageName, true) }
        steps += step("stay_awake") { policies.setGlobalSetting(PolicySet.STAY_ON_SETTING, PolicySet.STAY_ON_VALUE) }
        steps += step("keyguard_off") { check(policies.setKeyguardDisabled(true)) { "setKeyguardDisabled refused" } }
        steps += step("status_bar_off") { check(policies.setStatusBarDisabled(true)) { "setStatusBarDisabled refused" } }
        steps += step("update_policy") { policies.setSystemUpdatePolicyPostpone() }
        steps += lockTaskStep()
        if (release && provisioned) steps += restriction(PolicySet.DEBUGGING_FEATURES, present)
        return PolicyReport(steps).also { log(it) }
    }

    /**
     * Applies only Lock Task features NONE. The OS reverts them to the default at every boot and the kiosk
     * sets the power menu first, so this runs at every activity start and resume.
     */
    fun applyLockTaskFeatures(): PolicyReport {
        if (!policies.isDeviceOwner()) return notOwner()
        return PolicyReport(listOf(lockTaskStep())).also { log(it) }
    }

    private fun lockTaskStep(): PolicyStep = step(LOCK_TASK_STEP) { policies.setLockTaskFeatures(PolicySet.LOCK_TASK_FEATURES_NONE) }

    private fun notOwner(): PolicyReport {
        Timber.i("policy: not the device owner, nothing applied")
        return PolicyReport.NotOwner
    }

    private fun presentRestrictions(steps: MutableList<PolicyStep>): Set<String> {
        val read = isolated { policies.getUserRestrictions() }
        read.onFailure { steps += PolicyStep("restrictions_read", ok = false, error = describe(it)) }
        return read.getOrDefault(emptySet())
    }

    private fun restriction(
        key: String,
        present: Set<String>,
    ): PolicyStep {
        require(key !in PolicySet.NEVER_APPLIED) { "$key must never be applied" }
        val name = "restriction:$key"
        if (key in present) return PolicyStep(name, ok = true, skipped = true)
        return step(name) { policies.addUserRestriction(key) }
    }

    private fun step(
        name: String,
        call: () -> Unit,
    ): PolicyStep =
        isolated(call).fold(
            onSuccess = { PolicyStep(name, ok = true) },
            onFailure = { PolicyStep(name, ok = false, error = describe(it)) },
        )

    @Suppress("TooGenericExceptionCaught")
    private fun <T> isolated(call: () -> T): Result<T> =
        try {
            Result.success(call())
        } catch (e: Exception) {
            Result.failure(e)
        }

    private fun describe(e: Throwable): String = "${e.javaClass.simpleName}: ${e.message ?: "no message"}"

    private fun log(report: PolicyReport) {
        Timber.i("policy: %d ok, failed=%s", report.okCount, report.failed.map { it.name })
    }

    companion object {
        /** Name of the Lock Task features step. */
        const val LOCK_TASK_STEP = "lock_task_features"
    }
}
