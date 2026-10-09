package dev.nuagemagique.pilipos.platform

import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.engine.EngineState
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * P6-T11 (PD218): the platform features end to end on the emulator, with the real application and Hilt graph.
 * `scripts/android/plat_e2e.sh` prepares the device (device owner, bundle in the debug inbox, adb reverse rules,
 * host mocks, the registry mock) and runs this class once per step with `am instrument -e step <name>`, so that
 * it can wait for the process deaths between the steps (the install and the watchdog kill the process).
 *
 * Steps: `provision_and_policies`, `sales`, `export`, `update`, `update_refusals`, `fatal`, `check_audit`.
 * Other arguments: `managerInsz` (default 94060242576), `managerPin` (default 1111), `sales` (default 50),
 * `refusal` (`not_newer` or `sha256_mismatch`), `registryPort` (default 19392).
 *
 * Each step prints `PLATE2E_*` lines to logcat (tag `PlatE2e`) and fails by assertion; nothing is skipped.
 */
@RunWith(AndroidJUnit4::class)
class PlatformE2eTest {
    // No activity of its own: only the first step needs the UI, and it launches the activity itself.
    @get:Rule
    val compose = createEmptyComposeRule()

    private val env = PlatformE2eEnv()

    @Test
    fun step() {
        val step = InstrumentationRegistry.getArguments().getString("step") ?: throw AssertionError("the argument step is missing")
        if (step == "provision_and_policies") {
            provisionAndPolicies()
            return
        }
        env.openMain()
        when (step) {
            "sales" -> PlatformSalesStep.run(env)
            "export" -> PlatformExportStep.run(env)
            "update" -> PlatformUpdateSteps.update(env)
            "update_refusals" -> PlatformUpdateSteps.refusal(env)
            "fatal" -> PlatformWatchdogSteps.fatal(env)
            "check_audit" -> PlatformWatchdogSteps.checkAudit(env)
            else -> throw AssertionError("unknown step $step")
        }
    }

    private fun provisionAndPolicies() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            try {
                val state = env.app.holder().state.value
                if (state !is EngineState.Ready) {
                    compose.waitUntil(UI_TIMEOUT_MS) { compose.onAllNodesWithTag("shell_provisioning").fetchSemanticsNodes().isNotEmpty() }
                    compose.onNodeWithTag("prov_import_inbox").performClick()
                }
                val ready = env.awaitReady()
                PlatformE2eEnv.line("PLATE2E_READY posSwVersion=${ready.posSwVersion}")
                assertTrue("the record must exist", runBlocking { env.app.provisioning().refresh() } != null)
                PlatformPolicySteps.assertPolicies(env)
            } finally {
                leaveLockTask(scenario)
            }
        }
    }

    /** The kiosk pins the activity in Lock Task, which would keep the scenario from closing it. */
    private fun leaveLockTask(scenario: ActivityScenario<MainActivity>) {
        scenario.onActivity { activity ->
            try {
                activity.stopLockTask()
            } catch (e: IllegalStateException) {
                android.util.Log.i(PlatformE2eEnv.TAG, "stopLockTask: ${e.message}")
            }
        }
    }

    private companion object {
        const val UI_TIMEOUT_MS = 20_000L
    }
}
