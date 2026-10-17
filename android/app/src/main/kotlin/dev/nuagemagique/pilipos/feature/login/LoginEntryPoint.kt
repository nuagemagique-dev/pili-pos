package dev.nuagemagique.pilipos.feature.login

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.badge.BadgeStore
import dev.nuagemagique.pilipos.ui.fdm.FdmStore
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore

/**
 * What the login screen reads from the Hilt graph (PD205: no Hilt view model factory, the foundation tests host
 * the screen in a plain activity). The screen builds [LoginViewModel] from these with `viewModel { }`.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface LoginEntryPoint {
    /** Opens and closes the session. */
    fun session(): SessionStore

    /** The language, the settings (restaurant, terminal, version) and the training mode. */
    fun settings(): PosSettingsStore

    /** The HubRise switch: its check only runs when it is on. */
    fun integrations(): IntegrationsStore

    /** The FDM health: the start-up check asks it for a fresh reading. */
    fun fdm(): FdmStore

    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The text of a refusal. */
    fun errors(): ErrorPresenter

    /** The toasts. */
    fun toasts(): ToastStore

    /** The Vue texts. */
    fun catalog(): MessageCatalog

    /** The badge reader: a manager's badge opens their PIN pad, a cashier's badge logs in (lane 5). */
    fun badges(): BadgeStore

    /** The screen text of an engine refusal. */
    fun errorTexts(): ErrorTexts
}
