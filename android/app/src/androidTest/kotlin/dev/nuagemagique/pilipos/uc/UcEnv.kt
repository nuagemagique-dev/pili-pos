package dev.nuagemagique.pilipos.uc

import android.content.Context
import android.content.Intent
import androidx.test.platform.app.InstrumentationRegistry
import dagger.hilt.android.EntryPointAccessors
import dev.nuagemagique.pilipos.MainActivity
import dev.nuagemagique.pilipos.debugtools.E2eEntryPoint
import dev.nuagemagique.pilipos.engine.EngineState
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout

/** The pieces the two UC instrumented classes share: the Hilt entry point of the app under test and the Ready wait. */
internal object UcEnv {
    /** The logcat tag of the `UCE2E_*` lines that `scripts/android/uc_e2e.sh` reads. */
    const val TAG = "PiliUcE2e"

    private const val READY_TIMEOUT_MS = 90_000L

    /** The context of the app under test. */
    val context: Context get() = InstrumentationRegistry.getInstrumentation().targetContext

    /** The singletons of the real Hilt graph that the debug source set exposes. */
    val app: E2eEntryPoint
        get() = EntryPointAccessors.fromApplication(context.applicationContext, E2eEntryPoint::class.java)

    /** Instrumentation argument [name], or null. */
    fun argument(name: String): String? = InstrumentationRegistry.getArguments().getString(name)

    /**
     * Brings the application to the front the way the package-replaced receiver does (a new task of the same
     * package, which Lock Task allows). `MainActivity` starts the engine service.
     */
    fun openMain() {
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    /** Waits until the engine is Ready; fails when it becomes Fatal or does not come up. */
    fun awaitReady(): EngineState.Ready {
        val state =
            runBlocking {
                withTimeout(READY_TIMEOUT_MS) { app.holder().state.first { it is EngineState.Ready || it is EngineState.Fatal } }
            }
        return state as? EngineState.Ready ?: throw AssertionError("the engine did not become Ready: $state")
    }
}
