package dev.nuagemagique.pilipos.soak

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
import uniffi.pos_ffi.PosEngineInterface
import java.io.File

/**
 * The driver's only door to the engine: every call goes through `EngineGateway.call { }` (the way
 * `PlatformSalesStep` does it), from the instrumentation thread, never from the main thread.
 */
internal class SoakEngine {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()

    /** The context of the app under test. */
    val context: Context get() = instrumentation.targetContext

    /** The singletons of the real Hilt graph that the debug source set exposes. */
    private val app: E2eEntryPoint
        get() = EntryPointAccessors.fromApplication(context.applicationContext, E2eEntryPoint::class.java)

    /** The instrumentation arguments. */
    val arguments get() = InstrumentationRegistry.getArguments()

    /** `<externalFilesDir>/soak`. */
    fun soakDir(): File {
        val base = context.getExternalFilesDir(null) ?: throw AssertionError("no external files directory")
        return File(base, "soak")
    }

    /** Runs [block] on the engine through the gateway and returns its result. */
    fun <T> call(block: suspend (PosEngineInterface) -> T): T = runBlocking { app.gateway().call(block) }

    /** Waits until the engine is Ready (and fails when it becomes Fatal). */
    fun awaitReady() {
        val state =
            runBlocking {
                withTimeout(READY_TIMEOUT_MS) { app.holder().state.first { it is EngineState.Ready || it is EngineState.Fatal } }
            }
        if (state !is EngineState.Ready) throw AssertionError("the engine did not become Ready: $state")
    }

    /** Brings the application to the front the way the package-replaced receiver does (Lock Task allows it). */
    fun openMain() {
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    private companion object {
        const val READY_TIMEOUT_MS = 90_000L
    }
}
