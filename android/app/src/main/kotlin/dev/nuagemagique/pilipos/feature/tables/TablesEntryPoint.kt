package dev.nuagemagique.pilipos.feature.tables

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore

/**
 * What the tables screen reads from the Hilt graph (PD205: no Hilt view model factory, the foundation tests host
 * the screen in a plain activity). The screen builds [TablesViewModel] from these with `viewModel { }`.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface TablesEntryPoint {
    /** Who is logged in: the operator that opens the orders. */
    fun session(): SessionStore

    /** The order being taken: a table or a takeaway is opened through it. */
    fun order(): OrderStore

    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The engine events: a resync reloads the tables. */
    fun events(): EngineEvents

    /** The text of a refusal. */
    fun errors(): ErrorPresenter

    /** The language of the texts. */
    fun settings(): PosSettingsStore
}
