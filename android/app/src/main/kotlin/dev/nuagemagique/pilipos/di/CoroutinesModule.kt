package dev.nuagemagique.pilipos.di

import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import javax.inject.Qualifier
import javax.inject.Singleton

/** Dispatcher for blocking I/O (files, engine calls). */
@Qualifier
@Retention(AnnotationRetention.RUNTIME)
annotation class IoDispatcher

/** Process-wide scope for work that outlives a screen; a failing child does not cancel its siblings. */
@Qualifier
@Retention(AnnotationRetention.RUNTIME)
annotation class ApplicationScope

/**
 * Coroutine seams injected instead of the global dispatchers, so tests can replace them.
 *
 * Extended by the cards that need more dispatchers or scopes (P4-T5, P6).
 */
@Module
@InstallIn(SingletonComponent::class)
object CoroutinesModule {
    @Provides
    @IoDispatcher
    fun provideIoDispatcher(): CoroutineDispatcher = Dispatchers.IO

    @Provides
    @Singleton
    @ApplicationScope
    fun provideApplicationScope(): CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
}
