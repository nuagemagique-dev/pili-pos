package dev.nuagemagique.pilipos.engine

import uniffi.pos_ffi.EngineOptions
import uniffi.pos_ffi.PosEngine
import uniffi.pos_ffi.PosEngineInterface
import javax.inject.Inject
import javax.inject.Singleton

/** Opens the engine; synchronous, so callers run it on the I/O dispatcher. */
fun interface EngineFactory {
    /** Opens the database, runs the migrations, takes the engine lock and starts the background tasks. */
    fun open(options: EngineOptions): PosEngineInterface
}

/** [EngineFactory] backed by the generated UniFFI bindings. */
@Singleton
class UniffiEngineFactory
    @Inject
    constructor() : EngineFactory {
        override fun open(options: EngineOptions): PosEngineInterface = PosEngine.open(options)
    }
