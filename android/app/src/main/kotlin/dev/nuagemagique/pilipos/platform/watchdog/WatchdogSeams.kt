package dev.nuagemagique.pilipos.platform.watchdog

import android.content.Context
import android.os.Process
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.engine.EngineService
import javax.inject.Inject

/** Ends the process. A seam so that tests do not die. */
fun interface ProcessKiller {
    fun kill()
}

/** Kills the own process: the system restarts the persistent HOME app and the sticky engine service (PD118, PD207). */
class AndroidProcessKiller
    @Inject
    constructor() : ProcessKiller {
        override fun kill() {
            Process.killProcess(Process.myPid())
        }
    }

/** Starts the engine service again. A seam so that tests do not need a service. */
fun interface EngineStarter {
    fun start()
}

/** Calls [EngineService.start] with the application context. */
class EngineServiceStarter
    @Inject
    constructor(
        @ApplicationContext private val context: Context,
    ) : EngineStarter {
        override fun start() = EngineService.start(context)
    }
