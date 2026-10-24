package dev.nuagemagique.pilipos.nightly

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.ExportStatus
import java.util.Locale
import javax.inject.Inject

/** What the nightly export screen shows. */
sealed interface NightlyUiState {
    /** The first status is not here yet. */
    data object Loading : NightlyUiState

    /** The status of the export; [running] is true while a "retry now" is being handled. */
    data class Loaded(
        val status: ExportStatus,
        val running: Boolean = false,
    ) : NightlyUiState

    /** The last call failed; [message] is the screen text of the failure. */
    data class Failed(
        val message: String,
    ) : NightlyUiState
}

/** The language of the error texts of the screen. */
fun interface NightlyLanguage {
    /** The language to describe an engine failure in. */
    fun current(): AppLanguage
}

/** The language of the device, as the string resources of the screen follow it. */
internal class SystemNightlyLanguage
    @Inject
    constructor() : NightlyLanguage {
        override fun current(): AppLanguage = AppLanguage.fromCode(Locale.getDefault().language)
    }

/** Pause between two status reads while the screen is visible. */
internal const val NIGHTLY_POLL_MS: Long = 30_000L

/**
 * State holder of the nightly export screen. Every engine call goes through [EngineGateway]; the polling job
 * runs only between [startPolling] and [stopPolling], which the screen ties to its STARTED lifecycle state.
 */
@HiltViewModel
class NightlyViewModel
    @Inject
    constructor(
        private val gateway: EngineGateway,
        private val errorTexts: ErrorTexts,
        private val language: NightlyLanguage,
    ) : ViewModel() {
        private val mutableState = MutableStateFlow<NightlyUiState>(NightlyUiState.Loading)

        /** The screen state. */
        val state: StateFlow<NightlyUiState> = mutableState.asStateFlow()

        private var pollJob: Job? = null

        /** Reads the status now. */
        fun refresh() {
            viewModelScope.launch { guarded { load(keepRunning = true) } }
        }

        /** Makes the pending exports due at once, then reads the status; ignored while one is running. */
        fun runNow() {
            val current = mutableState.value as? NightlyUiState.Loaded ?: return
            if (current.running) return
            mutableState.value = current.copy(running = true)
            viewModelScope.launch {
                guarded {
                    gateway.call { it.exportRunNow() }
                    load(keepRunning = false)
                }
            }
        }

        /** Starts reading the status every [NIGHTLY_POLL_MS]; does nothing when it already runs. */
        fun startPolling() {
            if (pollJob?.isActive == true) return
            pollJob =
                viewModelScope.launch {
                    while (isActive) {
                        guarded { load(keepRunning = true) }
                        delay(NIGHTLY_POLL_MS)
                    }
                }
        }

        /** Stops the polling job. */
        fun stopPolling() {
            pollJob?.cancel()
            pollJob = null
        }

        /** Reads the status; a read that lands while a retry is running keeps the running flag when [keepRunning]. */
        private suspend fun load(keepRunning: Boolean) {
            val status = gateway.call { it.exportStatus() }
            mutableState.update { previous ->
                val running = keepRunning && (previous as? NightlyUiState.Loaded)?.running == true
                NightlyUiState.Loaded(status, running)
            }
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun guarded(block: suspend () -> Unit) {
            try {
                block()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.tag("PiliNightly").w(e, "export call failed")
                mutableState.value = NightlyUiState.Failed(errorTexts.describe(e, language.current()))
            }
        }
    }
