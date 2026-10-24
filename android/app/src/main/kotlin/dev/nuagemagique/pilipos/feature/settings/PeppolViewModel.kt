package dev.nuagemagique.pilipos.feature.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ERROR_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** How many deliveries the panel asks for (`LIMIT` of `PeppolPanel.vue`). */
private const val DELIVERIES_LIMIT: Long = 100L

/** The reading period of the panel while it is shown, in milliseconds (`POLL_MS`, 30 s). */
internal const val PEPPOL_POLL_MS: Long = 30_000L

/** Lifetime of the toast of a delivery put back in the queue. */
private const val RETRIED_TOAST_MS: Long = 3000L

/**
 * State holder of the Peppol panel of the settings screen, a port of the script of
 * `ui/src/components/PeppolPanel.vue`.
 *
 * It only reads for a manager (the engine refuses the command to the others). The engine owns every rule
 * (which row is parked, when a retry is allowed); this calls it through the gateway and keeps what the panel
 * shows. A read that fails keeps the last answer on screen and shows the refusal above it.
 */
internal class PeppolViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(PeppolUiState())
    private var polling: Job? = null

    /** What the panel draws. */
    val state: StateFlow<PeppolUiState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(isManager = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(isManager = entry.session().isManager.value))

    /** Reads the panel once; nothing is asked of the engine for an operator who is not a manager. */
    suspend fun load() {
        if (!entry.session().isManager.value) return
        attempt { entry.gateway().call { engine -> engine.listPeppolDeliveries(DELIVERIES_LIMIT) } }.fold(
            onSuccess = { panel -> own.update { it.copy(panel = panel, error = null) } },
            onFailure = { failure -> own.update { it.copy(error = entry.errors().text(failure, language())) } },
        )
    }

    /** Reads the panel now and every [PEPPOL_POLL_MS] while the screen is shown; a second call changes nothing. */
    fun startPolling() {
        if (polling?.isActive == true) return
        polling =
            viewModelScope.launch {
                while (true) {
                    load()
                    delay(PEPPOL_POLL_MS)
                }
            }
    }

    /** Stops the periodic reading (the screen is left); [startPolling] starts it again. */
    fun stopPolling() {
        polling?.cancel()
        polling = null
    }

    /** The Réessayer button of a parked delivery: puts it back in the queue, then reads the panel again. */
    fun retry(deliveryId: Long) {
        if (own.value.retrying != null || !entry.session().isManager.value) return
        own.update { it.copy(retrying = deliveryId) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.retryPeppolDelivery(deliveryId) } }.fold(
                onSuccess = {
                    entry.toasts().show(
                        entry.catalog().text("peppol_retried", language()),
                        ToastVariant.Success,
                        RETRIED_TOAST_MS,
                    )
                },
                onFailure = { entry.toasts().show(entry.errors().text(it, language()), ToastVariant.Error, ERROR_TOAST_TTL_MS) },
            )
            own.update { it.copy(retrying = null) }
            load()
        }
    }

    private fun language(): AppLanguage = entry.settings().language.value
}
