package dev.nuagemagique.pilipos.shell

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineHolder
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.provisioning.BundleSource
import dev.nuagemagique.pilipos.provisioning.DeviceOwner
import dev.nuagemagique.pilipos.provisioning.ImportResult
import dev.nuagemagique.pilipos.provisioning.ProvisioningPaths
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.filterIsInstance
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.PosEvent
import javax.inject.Inject

/** One-shot effects of the shell for the UI. */
sealed interface ShellEffect {
    /** The engine locked the session: go back to the login screen. */
    data object NavigateToLogin : ShellEffect
}

/**
 * State holder of the shell: the [ShellState] to show, the import of a bundle and the effects.
 *
 * The engine owns the idle lock; the shell only navigates when it announces [PosEvent.SessionLocked].
 * Every state change is logged under the tag `PiliShell` (P4-T12 reads it from logcat).
 */
@HiltViewModel
class ShellViewModel
    @Inject
    constructor(
        private val deviceOwner: DeviceOwner,
        private val holder: EngineHolder,
        engineEvents: EngineEvents,
        private val coordinator: ProvisioningCoordinator,
        private val paths: ProvisioningPaths,
        /** The navigation contributions shown when the engine is ready. */
        val registry: NavRegistry,
    ) : ViewModel() {
        private val progress = MutableStateFlow(ShellProgress())
        private val effectChannel = Channel<ShellEffect>(Channel.BUFFERED)

        /** The screen to show. */
        val state: StateFlow<ShellState> =
            combine(holder.state, progress) { engine, shellProgress ->
                mapShellState(deviceOwner.isDeviceOwner(), engine, shellProgress)
            }.onEach { Timber.tag(TAG).i("state=%s", it::class.simpleName) }
                .stateIn(
                    viewModelScope,
                    SharingStarted.Eagerly,
                    mapShellState(deviceOwner.isDeviceOwner(), holder.state.value, progress.value),
                )

        /** One-shot effects, each delivered once. */
        val effects: Flow<ShellEffect> = effectChannel.receiveAsFlow()

        init {
            // Subscribe before anything can happen: the event flow has no replay.
            viewModelScope.launch(start = CoroutineStart.UNDISPATCHED) {
                engineEvents.events
                    .filterIsInstance<PosEvent.SessionLocked>()
                    .collect { effectChannel.send(ShellEffect.NavigateToLogin) }
            }
            viewModelScope.launch {
                holder.open()
                progress.update { it.copy(firstOpenDone = true) }
            }
        }

        /** Imports the bundle of [source] (close the engine, import, open it again). Ignored while one runs. */
        fun importFrom(source: BundleSource) {
            val started = progress.updateAndGetStarted()
            if (!started) return
            viewModelScope.launch {
                val result = coordinator.import(source)
                progress.update {
                    it.copy(
                        importing = false,
                        problem = (result as? ImportResult.Failure)?.problem,
                    )
                }
            }
        }

        /** Imports from the debug inbox directory (debug builds only call this). */
        fun importFromInbox() {
            val inbox = paths.debugInboxDir()
            if (inbox == null) {
                progress.update { it.copy(problem = ProvisioningProblem.of("copy_failed", "detail" to "no inbox")) }
            } else {
                importFrom(BundleSource.Directory(inbox))
            }
        }

        /** Marks an import as running; false when one already runs. */
        private fun MutableStateFlow<ShellProgress>.updateAndGetStarted(): Boolean {
            while (true) {
                val current = value
                if (current.importing) return false
                if (compareAndSet(current, current.copy(importing = true, problem = null))) return true
            }
        }

        private companion object {
            const val TAG = "PiliShell"
        }
    }
