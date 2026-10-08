package dev.nuagemagique.pilipos.engine

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent

/** Binds the native seams of the engine runtime to the UniFFI implementations. */
@Module
@InstallIn(SingletonComponent::class)
abstract class EngineModule {
    @Binds
    abstract fun bindEngineNatives(natives: UniffiEngineNatives): EngineNatives

    @Binds
    abstract fun bindEngineFactory(factory: UniffiEngineFactory): EngineFactory
}
