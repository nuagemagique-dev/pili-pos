package dev.nuagemagique.pilipos.feature.reports

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.nav.FeatureEntry

/** Contributes the reports entry to the navigation set. */
@Module
@InstallIn(SingletonComponent::class)
internal abstract class ReportsModule {
    @Binds
    @IntoSet
    abstract fun bindReportsEntry(entry: ReportsEntry): FeatureEntry
}
