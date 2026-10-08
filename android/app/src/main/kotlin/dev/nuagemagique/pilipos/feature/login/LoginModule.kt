package dev.nuagemagique.pilipos.feature.login

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.FeatureEntry

/** Contributes the login entry to the navigation set. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class LoginModule {
    @Binds
    @IntoSet
    abstract fun bindLoginEntry(entry: LoginEntry): FeatureEntry
}
