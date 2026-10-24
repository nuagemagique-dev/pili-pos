package dev.nuagemagique.pilipos.feature.webshop

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.FeatureEntry

/** Contributes the webshop entry to the navigation set. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class WebshopModule {
    @Binds
    @IntoSet
    abstract fun bindWebshopEntry(entry: WebshopEntry): FeatureEntry
}
