package dev.nuagemagique.pilipos.feature.settings.vouchers

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
import uniffi.pos_ffi.GiftVoucherSettings

/**
 * State holder of the gift voucher panel of the settings screen, a port of the script of
 * `ui/src/components/GiftVoucherSettings.vue`.
 *
 * Managers only: it loads when a manager is logged in and forgets everything otherwise. The engine owns every rule
 * (the range of the validity, the largest amount, who may store the setting): this reads the setting, keeps the
 * draft the manager edits and sends it back whole.
 */
internal class GiftVoucherViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(GiftVoucherUiState())

    /** What the panel draws. */
    val state: StateFlow<GiftVoucherUiState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(manager = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(manager = entry.session().isManager.value))

    init {
        viewModelScope.launch {
            entry.session().isManager.collect { manager ->
                if (manager) load() else own.value = GiftVoucherUiState()
            }
        }
    }

    /** Reads the setting (`onMounted`). */
    suspend fun load() {
        own.update { it.copy(error = null) }
        attempt { entry.gateway().call { engine -> engine.getGiftVoucherSettings() } }.fold(
            onSuccess = { adopt(it, saved = false) },
            onFailure = { fail(it) },
        )
    }

    /** The switch. */
    fun toggleEnabled() = edit { it.copy(enabled = !it.enabled) }

    /** The validity input, in months. */
    fun setValidity(text: String) = edit { it.copy(validity = text) }

    /** The maximum amount input, in euros. */
    fun setMax(text: String) = edit { it.copy(maxEuros = text) }

    private fun edit(change: (GiftVoucherUiState) -> GiftVoucherUiState) {
        own.update { s -> if (s.stored == null) s else change(s).copy(saved = false) }
    }

    /** Saves the draft; the engine answers the stored value, which becomes the new reference. */
    fun save() {
        val draft = own.value.draft()
        if (own.value.busy) return
        own.update { it.copy(error = null, busy = true) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.setGiftVoucherSettings(draft) } }.fold(
                onSuccess = { adopt(it, saved = true) },
                onFailure = { fail(it) },
            )
            own.update { it.copy(busy = false) }
        }
    }

    private fun adopt(
        settings: GiftVoucherSettings,
        saved: Boolean,
    ) {
        own.update {
            it.copy(
                stored = settings,
                enabled = settings.enabled,
                validity = "${settings.validityMonths}",
                maxEuros = eurosText(settings.maxAmountCents),
                saved = saved,
            )
        }
    }

    private fun fail(error: Throwable) {
        own.update { it.copy(error = entry.errors().text(error, entry.settings().language.value)) }
    }
}
