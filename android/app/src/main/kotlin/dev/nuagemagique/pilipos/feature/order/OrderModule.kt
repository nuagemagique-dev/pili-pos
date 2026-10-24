package dev.nuagemagique.pilipos.feature.order

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.FeatureEntry

/** Contributes the order entry to the navigation set. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class OrderModule {
    @Binds
    @IntoSet
    abstract fun bindOrderEntry(entry: OrderEntry): FeatureEntry
}
