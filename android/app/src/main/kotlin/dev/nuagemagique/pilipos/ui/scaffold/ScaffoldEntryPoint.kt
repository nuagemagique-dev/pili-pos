package dev.nuagemagique.pilipos.ui.scaffold

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.fdm.FdmStore
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore

/**
 * What the staff scaffold reads from the Hilt graph (PD205: no Hilt view model factory, the foundation tests host
 * the screens in a plain activity). [PosScaffold] builds [HeaderViewModel] from these with `viewModel { }`.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface ScaffoldEntryPoint {
    /** Who is logged in, and the lock. */
    fun session(): SessionStore

    /** The language and the training mode. */
    fun settings(): PosSettingsStore

    /** The HubRise switch: the online orders button and light only show while it is visible. */
    fun integrations(): IntegrationsStore

    /** The order being taken: the table number, the leave guard, the new takeaway. */
    fun order(): OrderStore

    /** The FDM health: the light and the blocking veil. */
    fun fdm(): FdmStore

    /** The pending online orders for the badge. */
    fun webshop(): WebshopOrdersStore

    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The text of a refusal. */
    fun errors(): ErrorPresenter

    /** The toasts. */
    fun toasts(): ToastStore

    /** The Vue texts. */
    fun catalog(): MessageCatalog
}
