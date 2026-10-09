package dev.nuagemagique.pilipos.feature.order

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.format.CartAmounts
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore

/**
 * What the order screen reads from the Hilt graph (PD205: no Hilt view model factory, the foundation tests host the
 * screen in a plain activity). The screen builds [MenuViewModel] from these with `viewModel { }`.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface OrderEntryPoint {
    /** Who is logged in: the menu is read for a user only, and the pay button needs the payment right. */
    fun session(): SessionStore

    /** The order being taken: the cart, the total, and every change of a line. */
    fun order(): OrderStore

    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The engine events: the catalog changed, or the engine asks for a full re-read. */
    fun events(): EngineEvents

    /** The text of a refusal. */
    fun errors(): ErrorPresenter

    /** The language of the menu and the product label (name or code). */
    fun settings(): PosSettingsStore

    /** The HubRise switch: with it on, a product is checked for options before it is added. */
    fun integrations(): IntegrationsStore

    /** The amount of each cart line, from the engine VAT helper. */
    fun amounts(): CartAmounts

    /** The success toasts of the cart (sent to the kitchen, bill printed, sale closed unpaid). */
    fun toasts(): ToastStore

    /** The Vue texts of those toasts, which a state holder cannot ask a composition for. */
    fun catalog(): MessageCatalog
}
