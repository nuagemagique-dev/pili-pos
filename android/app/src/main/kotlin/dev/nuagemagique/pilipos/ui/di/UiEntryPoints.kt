@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.ui.di

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.android.EntryPointAccessors
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.state.IntegrationsStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore

/**
 * What every screen reads from the Hilt graph: the shared stores, the texts and the engine seams.
 *
 * Screens do not use the Hilt view model factory (PD205): the foundation tests host them in a plain
 * `ComponentActivity`, which is not an `@AndroidEntryPoint`, so that factory would fail there. A screen reaches the
 * graph through [rememberEntryPoint] instead and builds its view model by hand:
 *
 * ```
 * @EntryPoint
 * @InstallIn(SingletonComponent::class)
 * interface TablesEntryPoint { fun tableStore(): TableStore }
 *
 * @Composable
 * fun TablesScreen() {
 *     val entry = rememberEntryPoint<TablesEntryPoint>()
 *     val model = viewModel { TablesViewModel(entry.tableStore()) }
 * }
 * ```
 *
 * `viewModel { }` is `androidx.lifecycle.viewmodel.compose.viewModel` with an initializer lambda.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
interface UiCommonEntryPoint {
    /** The engine settings, the language and the product label. */
    fun settings(): PosSettingsStore

    /** Who is logged in. */
    fun session(): SessionStore

    /** The HubRise switch. */
    fun integrations(): IntegrationsStore

    /** The Vue texts. */
    fun catalog(): MessageCatalog

    /** The screen text of an engine refusal. */
    fun errorTexts(): ErrorTexts

    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The stateless engine helpers (settlement, VAT, QR). */
    fun helpers(): EngineHelpers

    /** The engine events. */
    fun events(): EngineEvents
}

/** The entry point [T] of the application graph, looked up once per application context. */
@Composable
inline fun <reified T : Any> rememberEntryPoint(): T {
    val application = LocalContext.current.applicationContext
    return remember(application) { EntryPointAccessors.fromApplication(application, T::class.java) }
}
