package dev.nuagemagique.pilipos.feature.tables

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.FeatureEntry

/** Contributes the tables entry to the navigation set. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class TablesModule {
    @Binds
    @IntoSet
    abstract fun bindTablesEntry(entry: TablesEntry): FeatureEntry
}
