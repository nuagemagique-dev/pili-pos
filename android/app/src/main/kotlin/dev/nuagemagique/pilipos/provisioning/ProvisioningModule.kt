package dev.nuagemagique.pilipos.provisioning

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.LaunchConfigSource
import javax.inject.Singleton

/** Binds the provisioning store as the source of the launch configuration. */
@Module
@InstallIn(SingletonComponent::class)
abstract class ProvisioningModule {
    @Binds
    @Singleton
    abstract fun bindLaunchConfigSource(store: ProvisioningStore): LaunchConfigSource
}
