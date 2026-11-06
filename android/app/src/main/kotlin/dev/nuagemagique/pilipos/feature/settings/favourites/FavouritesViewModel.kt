package dev.nuagemagique.pilipos.feature.settings.favourites

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.feature.settings.SettingsFeatureEntryPoint
import dev.nuagemagique.pilipos.feature.settings.attempt
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.FavouritesMode
import uniffi.pos_ffi.FavouritesSettings

/**
 * State holder of the favourites panel of the settings screen, a port of the script of
 * `ui/src/components/FavouritesSettings.vue` (C12).
 *
 * Managers only: it loads when a manager is logged in and forgets everything otherwise. The engine owns every rule (who
 * may store the setting, the known products, the count): this reads the setting, and every tap (the mode, the removal
 * of a product) is sent whole and saved at once. The products are starred in the menu editor.
 */
internal class FavouritesViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(FavouritesUiState())

    /** What the panel draws. */
    val state: StateFlow<FavouritesUiState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(manager = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(manager = entry.session().isManager.value))

    init {
        viewModelScope.launch {
            entry.session().isManager.collect { manager ->
                if (manager) load() else own.value = FavouritesUiState()
            }
        }
    }

    /** Reads the setting and the names of the products (`onMounted`). */
    suspend fun load() {
        own.update { it.copy(error = null) }
        attempt {
            val settings = entry.gateway().call { engine -> engine.getFavouritesSettings() }
            val items = entry.gateway().call { engine -> engine.getMenuItems(null, entry.settings().language.value.code) }
            settings to items.associate { it.id to it.name }
        }.fold(
            onSuccess = { (settings, names) -> own.update { it.adopt(settings).copy(names = names) } },
            onFailure = { fail(it) },
        )
    }

    /** A tap on one of the two modes: saved at once with the selection kept. */
    fun setMode(mode: FavouritesMode) {
        val s = own.value
        if (!s.loaded || s.mode == mode) return
        save(FavouritesSettings(mode, s.selection))
    }

    /** The cross of a product of the selection: saved at once, the others keep their order. */
    fun remove(productId: Long) {
        val s = own.value
        val mode = s.mode ?: return
        save(FavouritesSettings(mode, s.selection.filter { it != productId }))
    }

    private fun save(next: FavouritesSettings) {
        if (own.value.busy) return
        own.update { it.copy(error = null, busy = true) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.setFavouritesSettings(next) } }.fold(
                onSuccess = { stored -> own.update { it.adopt(stored) } },
                onFailure = { fail(it) },
            )
            own.update { it.copy(busy = false) }
        }
    }

    private fun FavouritesUiState.adopt(settings: FavouritesSettings) = copy(mode = settings.mode, selection = settings.selection)

    private fun fail(error: Throwable) {
        own.update { it.copy(error = entry.errors().text(error, entry.settings().language.value)) }
    }
}
