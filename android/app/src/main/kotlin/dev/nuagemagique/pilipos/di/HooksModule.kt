package dev.nuagemagique.pilipos.di

import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.Multibinds

/**
 * Declares the set of [AppLifecycleHook]s so that it exists (empty) when no lane binds one. A lane adds a
 * hook with `@Binds @IntoSet` in a module of its own package.
 */
@Module
@InstallIn(SingletonComponent::class)
abstract class HooksModule {
    @Multibinds
    abstract fun appLifecycleHooks(): Set<@JvmSuppressWildcards AppLifecycleHook>
}
