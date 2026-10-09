package dev.nuagemagique.pilipos.feature.settings

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.DeliveryStore
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore

/**
 * What the settings screen reads from the Hilt graph (PD205: no Hilt view model factory). The screen builds
 * [SettingsViewModel] and [HubriseViewModel] from these with `viewModel { }`.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
@Suppress("TooManyFunctions") // One accessor per store or service the screen reads.
internal interface SettingsFeatureEntryPoint {
    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The engine events (the HubRise catalog events). */
    fun events(): EngineEvents

    /** The stateless helpers: the screen draws the HubRise QR from [EngineHelpers.qrMatrix] (PD218). */
    fun helpers(): EngineHelpers

    /** Who is logged in, and whether that is a manager. */
    fun session(): SessionStore

    /** The language, the product label, the training mode and the engine settings. */
    fun settings(): PosSettingsStore

    /** The HubRise switch. */
    fun integrations(): IntegrationsStore

    /** The text of a refusal. */
    fun errors(): ErrorPresenter

    /** The toasts. */
    fun toasts(): ToastStore

    /** The Vue texts. */
    fun catalog(): MessageCatalog

    /** The settings rows the other features contribute (UI-PD308). */
    fun registry(): NavRegistry

    /** The mail status of the ticket deliveries panel (`useDeliveryStore().refreshMailStatus()` on open). */
    fun delivery(): DeliveryStore
}
