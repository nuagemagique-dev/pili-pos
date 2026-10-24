package dev.nuagemagique.pilipos.platform.about

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** What the About screen shows. */
sealed interface AboutUiState {
    /** The first report is being built. */
    data object Loading : AboutUiState

    /** The last report; a refresh keeps showing it until the new one is ready. */
    data class Loaded(
        val report: IdentityReport,
    ) : AboutUiState
}

/**
 * Builds the identity report on creation and again on [refresh].
 *
 * Created by [AboutScreen] with the [IdentityProvider] of the Hilt graph, taken from an entry point rather
 * than from `hiltViewModel()`, so that the screen also works under a host activity that is not an
 * `@AndroidEntryPoint` (the navigation tests of the app).
 */
class AboutViewModel(
    private val identity: IdentityProvider,
) : ViewModel() {
    private val mutable = MutableStateFlow<AboutUiState>(AboutUiState.Loading)
    private var job: Job? = null

    /** The screen state. */
    val state: StateFlow<AboutUiState> = mutable.asStateFlow()

    init {
        refresh()
    }

    /** Rebuilds the report; a refresh that is already running is replaced. */
    fun refresh() {
        job?.cancel()
        job = viewModelScope.launch { mutable.value = AboutUiState.Loaded(identity.report()) }
    }
}

/** Gives [AboutScreen] the [IdentityProvider] of the Hilt graph, whatever activity hosts the screen. */
@EntryPoint
@InstallIn(SingletonComponent::class)
internal interface AboutEntryPoint {
    fun identityProvider(): IdentityProvider
}
