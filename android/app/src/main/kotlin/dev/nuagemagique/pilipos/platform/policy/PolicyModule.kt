package dev.nuagemagique.pilipos.platform.policy

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.di.AppLifecycleHook

/** Binds the device-owner policy hook into the lifecycle hook set and the production [DevicePolicies]. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class PolicyModule {
    @Binds
    @IntoSet
    abstract fun bindPolicyHook(hook: PolicyHook): AppLifecycleHook

    @Binds
    abstract fun bindDevicePolicies(policies: AndroidDevicePolicies): DevicePolicies
}
