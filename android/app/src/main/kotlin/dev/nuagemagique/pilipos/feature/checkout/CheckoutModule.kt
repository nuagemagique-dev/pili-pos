package dev.nuagemagique.pilipos.feature.checkout

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.FeatureEntry

/** Contributes the checkout entry to the navigation set. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class CheckoutModule {
    @Binds
    @IntoSet
    abstract fun bindCheckoutEntry(entry: CheckoutEntry): FeatureEntry
}
