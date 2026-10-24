package dev.nuagemagique.pilipos.uc

import android.util.Log
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.engine.EngineState
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Provisions a freshly installed app from the development bundle in the debug inbox (the import the manager does
 * with the "inbox" button), once per scenario group. `scripts/android/uc_e2e.sh` pushes the bundle, then runs this
 * class with `am instrument`; the engine is Ready afterwards and the scenarios of the group follow.
 */
@RunWith(AndroidJUnit4::class)
class UcProvisionTest {
    @get:Rule
    val compose = createEmptyComposeRule()

    @Test
    fun theBundleOfTheInboxProvisionsTheTill() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            try {
                if (UcEnv.app.holder().state.value !is EngineState.Ready) {
                    compose.waitUntil(UI_TIMEOUT_MS) { compose.onAllNodesWithTag("shell_provisioning").fetchSemanticsNodes().isNotEmpty() }
                    compose.onNodeWithTag("prov_import_inbox").performClick()
                }
                val ready = UcEnv.awaitReady()
                Log.i(UcEnv.TAG, "UCE2E_PROVISIONED posSwVersion=${ready.posSwVersion}")
                assertTrue("the posSwVersion must be the Android one", ready.posSwVersion.startsWith("pili-pos-android/"))
            } finally {
                // The kiosk pins the activity in Lock Task, which would keep the scenario from closing it.
                scenario.onActivity { activity ->
                    try {
                        activity.stopLockTask()
                    } catch (e: IllegalStateException) {
                        Log.i(UcEnv.TAG, "stopLockTask: ${e.message}")
                    }
                }
            }
        }
    }

    private companion object {
        const val UI_TIMEOUT_MS = 20_000L
    }
}
