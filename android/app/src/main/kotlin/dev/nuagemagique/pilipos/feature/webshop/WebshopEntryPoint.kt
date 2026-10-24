package dev.nuagemagique.pilipos.feature.webshop

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.EftStore
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore

/**
 * What the webshop screen reads from the Hilt graph (PD205: no Hilt view model factory, the foundation tests host
 * the screen in a plain activity). The screen builds [WebshopViewModel] from these with `viewModel { }`.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface WebshopEntryPoint {
    /** The online orders and the board actions (every webshop command goes through it). */
    fun webshop(): WebshopOrdersStore

    /** Who is logged in: the manager that confirms a refund. */
    fun session(): SessionStore

    /** The language of the texts. */
    fun settings(): PosSettingsStore

    /** The HubRise switch: the board is only read while HubRise is visible. */
    fun integrations(): IntegrationsStore

    /** The text of a refusal that did not come from the store. */
    fun errors(): ErrorPresenter

    /** The linked card terminal: the overlay of a card collected on it. */
    fun eft(): EftStore

    /** The warning shown when a refund is asked without a manager. */
    fun toasts(): ToastStore

    /** The Vue texts used by the view model. */
    fun catalog(): MessageCatalog
}
