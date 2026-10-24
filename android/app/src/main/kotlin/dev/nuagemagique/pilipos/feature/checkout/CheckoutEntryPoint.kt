package dev.nuagemagique.pilipos.feature.checkout

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.format.CartAmounts
import dev.nuagemagique.pilipos.ui.state.DeliveryStore
import dev.nuagemagique.pilipos.ui.state.EftStore
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore

/**
 * What the checkout screen reads from the Hilt graph (PD205: no Hilt view model factory, the foundation tests host
 * the screen in a plain activity). The screen builds [CheckoutViewModel] from these with `viewModel { }`.
 */
@Suppress("TooManyFunctions") // One accessor per store the screen reads.
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface CheckoutEntryPoint {
    /** The only way to call the engine (`checkout`). */
    fun gateway(): EngineGateway

    /** The stateless engine helpers: the cash rest and the rounded cash due come from `settlePreview`. */
    fun helpers(): EngineHelpers

    /** The order being paid, its lines and its engine total. */
    fun order(): OrderStore

    /** Who is logged in: only a cashier or an administrator takes a payment. */
    fun session(): SessionStore

    /** The ticket-delivery choice of the customer, shared with the refund and the copy. */
    fun delivery(): DeliveryStore

    /** The language and the product label. */
    fun settings(): PosSettingsStore

    /** The text of a refusal. */
    fun errors(): ErrorPresenter

    /** The slip warning after the sale. */
    fun toasts(): ToastStore

    /** The Vue texts used outside a composition (the slip warning). */
    fun catalog(): MessageCatalog

    /** The linked card payment terminal: charging a card part, recovery, the blocking overlay. */
    fun eft(): EftStore

    /** The line amounts and the VAT rows of the recap, from the engine VAT split. */
    fun amounts(): CartAmounts
}
