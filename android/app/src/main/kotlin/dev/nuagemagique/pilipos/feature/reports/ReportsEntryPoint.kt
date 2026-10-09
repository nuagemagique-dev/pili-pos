package dev.nuagemagique.pilipos.feature.reports

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore

/**
 * What the reports screen reads from the Hilt graph (PD205: no Hilt view model factory). The screen builds
 * [ReportsViewModel] from these with `viewModel { }`.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface ReportsEntryPoint {
    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The engine helpers (VAT split and cash rounding) the refund preview of the screen needs. */
    fun helpers(): EngineHelpers

    /** The engine events: a `Resync` reloads the day tab. */
    fun events(): EngineEvents

    /** The operator: the INSZ every report and cash command is signed with. */
    fun session(): SessionStore

    /** The language of the texts. */
    fun settings(): PosSettingsStore

    /** The text of a refusal. */
    fun errors(): ErrorPresenter

    /** The toasts. */
    fun toasts(): ToastStore

    /** The Vue texts. */
    fun catalog(): MessageCatalog
}
