package dev.nuagemagique.pilipos.platform.policy

import android.app.Activity
import android.app.Application
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.di.AppLifecycleHook
import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.provisioning.ProvisioningRecord
import dev.nuagemagique.pilipos.provisioning.ProvisioningStore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicBoolean
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Runs the device-owner policies at the lifecycle points (PD206). It runs after the kiosk, which the
 * foundation calls directly in `MainActivity.onCreate`, so its Lock Task features NONE override the power menu
 * the kiosk enables.
 *
 * - Application start: the whole set, once, in the application scope on the I/O dispatcher, then one audit
 *   summary row. In a release build with no provisioning record yet, it waits for the record (the first
 *   import) and applies the set again, which adds adb off.
 * - Activity create and resume: Lock Task features NONE, synchronously (the OS reverts them at every boot, S4
 *   section 2.3), and the whole set again only when the previous run had failed steps.
 *
 * Failed steps are audited (kind `policy`); a successful resume writes nothing.
 */
@Singleton
class PolicyHook
    @Suppress("LongParameterList")
    internal constructor(
        private val applier: PolicyApplier,
        private val audit: PlatformAudit,
        private val record: StateFlow<ProvisioningRecord?>,
        private val refreshRecord: suspend () -> Unit,
        private val release: Boolean,
        private val scope: CoroutineScope,
        private val ioDispatcher: CoroutineDispatcher,
    ) : AppLifecycleHook {
        @Inject
        constructor(
            applier: PolicyApplier,
            audit: PlatformAudit,
            provisioning: ProvisioningStore,
            @ApplicationScope scope: CoroutineScope,
            @IoDispatcher ioDispatcher: CoroutineDispatcher,
        ) : this(
            applier = applier,
            audit = audit,
            record = provisioning.record,
            refreshRecord = { provisioning.refresh() },
            release = !BuildConfig.DEBUG,
            scope = scope,
            ioDispatcher = ioDispatcher,
        )

        override val order: Int = ORDER

        private val lock = Any()
        private val retrying = AtomicBoolean(false)

        @Volatile
        private var needsRetry = false

        override fun onApplicationCreated(app: Application) {
            scope.launch(ioDispatcher) {
                refreshRecord()
                summarize(runFull(), TRIGGER_START)
                if (release && record.value == null) {
                    record.first { it != null }
                    summarize(runFull(), TRIGGER_PROVISIONED)
                }
            }
        }

        override fun onActivityCreated(activity: Activity) = onActivityPoint()

        override fun onActivityResumed(activity: Activity) = onActivityPoint()

        private fun onActivityPoint() {
            val report = applier.applyLockTaskFeatures()
            if (report.failed.isNotEmpty()) scope.launch(ioDispatcher) { auditFailures(report) }
            if (needsRetry && retrying.compareAndSet(false, true)) {
                scope.launch(ioDispatcher) {
                    try {
                        auditFailures(runFull())
                    } finally {
                        retrying.set(false)
                    }
                }
            }
        }

        private fun runFull(): PolicyReport {
            val report = synchronized(lock) { applier.apply(release = release, provisioned = record.value != null) }
            needsRetry = report.failed.isNotEmpty()
            return report
        }

        private suspend fun summarize(
            report: PolicyReport,
            trigger: String,
        ) {
            auditFailures(report)
            audit.append(
                KIND,
                if (report.notOwner) "not_owner" else "applied",
                mapOf(
                    "trigger" to trigger,
                    "ok" to report.okCount.toString(),
                    "failed" to report.failed.joinToString(",") { it.name },
                ),
            )
        }

        private suspend fun auditFailures(report: PolicyReport) {
            for (step in report.failed) {
                audit.append(KIND, "step_failed", mapOf("step" to step.name, "error" to (step.error ?: "")))
            }
        }

        companion object {
            /** Position among the lifecycle hooks: after the kiosk, before the watchdog (200). */
            const val ORDER = 100

            /** Audit kind of every row written here. */
            const val KIND = "policy"

            private const val TRIGGER_START = "start"
            private const val TRIGGER_PROVISIONED = "provisioned"
        }
    }
