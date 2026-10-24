package dev.nuagemagique.pilipos.provisioning

import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.engine.LaunchConfigSource
import java.time.Clock
import javax.inject.Qualifier
import javax.inject.Singleton

/** The minisign public key bundles are verified with (build-time constant, PD108). */
@Qualifier
@Retention(AnnotationRetention.RUNTIME)
annotation class ProvisioningPublicKey

/** The clock that stamps imports. */
@Qualifier
@Retention(AnnotationRetention.RUNTIME)
annotation class ProvisioningClock

/** Binds the provisioning store as the source of the launch configuration, and its seams. */
@Module
@InstallIn(SingletonComponent::class)
abstract class ProvisioningModule {
    @Binds
    @Singleton
    abstract fun bindLaunchConfigSource(store: ProvisioningStore): LaunchConfigSource

    @Binds
    abstract fun bindNatives(natives: UniffiProvisioningNatives): ProvisioningNatives

    @Binds
    abstract fun bindTreeReader(reader: SafTreeReader): TreeReader
}

/** Build-time and time seams of provisioning. */
@Module
@InstallIn(SingletonComponent::class)
object ProvisioningValuesModule {
    @Provides
    @ProvisioningPublicKey
    fun providePublicKey(): String = BuildConfig.PROVISIONING_PUBLIC_KEY

    @Provides
    @ProvisioningClock
    fun provideClock(): Clock = Clock.systemUTC()
}
