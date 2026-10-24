package dev.nuagemagique.pilipos.platform.watchdog

import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.di.AppLifecycleHook
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import dev.nuagemagique.pilipos.platform.audit.SystemPlatformClock

/** Binds the crash watchdog as an [AppLifecycleHook] and the seams it uses. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class WatchdogModule {
    @Binds
    @IntoSet
    abstract fun bindEngineWatchdog(watchdog: EngineWatchdog): AppLifecycleHook

    @Binds
    abstract fun bindProcessKiller(killer: AndroidProcessKiller): ProcessKiller

    @Binds
    abstract fun bindEngineStarter(starter: EngineServiceStarter): EngineStarter

    companion object {
        @Provides
        fun providePlatformClock(): PlatformClock = SystemPlatformClock
    }
}
