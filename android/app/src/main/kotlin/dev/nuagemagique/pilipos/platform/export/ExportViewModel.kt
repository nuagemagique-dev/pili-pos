package dev.nuagemagique.pilipos.platform.export

import android.content.Intent
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** What the export screen shows. */
sealed interface ExportUiState {
    /** The first check (user, open orders) is running. */
    data object Checking : ExportUiState

    /** The logged-in user is not a manager: nothing else is shown. */
    data object NotManager : ExportUiState

    /** The export cannot start now because of [problem] (an open order or takeaway, the engine). */
    data class Blocked(
        val problem: ExportProblem,
    ) : ExportUiState

    /** No removable volume is mounted: "insert the USB stick or SD card". */
    data object NoStick : ExportUiState

    /** [volume] has no folder yet; [error] says why the last pick was refused. */
    data class NeedsFolder(
        val volumes: List<RemovableVolume>,
        val volume: RemovableVolume,
        val error: ExportProblem? = null,
    ) : ExportUiState

    /** The export can start into [folderName] of [volume]. */
    data class Ready(
        val volumes: List<RemovableVolume>,
        val volume: RemovableVolume,
        val folderName: String,
    ) : ExportUiState

    /** An export is running. */
    data class Running(
        val progress: ExportProgress,
    ) : ExportUiState

    /** The export finished and every file was verified. */
    data class Done(
        val result: ExportResult,
    ) : ExportUiState

    /** The export stopped; what was written is kept. */
    data class Failed(
        val failure: ExportFailure,
    ) : ExportUiState
}

/**
 * The state machine of the export screen (Q17): Checking, then NotManager, Blocked, NoStick, NeedsFolder or
 * Ready; Running while [Exporter.run] works; Done or Failed until [dismiss].
 *
 * [refresh] re-reads the rights and the volumes (call it when the screen comes to the front);
 * [refreshVolumes] only re-reads the volumes (the screen calls it every couple of seconds, a stick can be
 * inserted at any time). Created by [ExportScreen] with the collaborators of the Hilt graph.
 */
class ExportViewModel(
    private val exporter: Exporter,
    private val probe: VolumeProbe,
    private val folders: ExportFolderStore,
    private val targets: ExportTargets,
) : ViewModel() {
    private val mutable = MutableStateFlow<ExportUiState>(ExportUiState.Checking)
    private var access: ExportProblem? = null
    private var accessKnown = false
    private var selected: String? = null
    private var pickError: ExportProblem? = null
    private var job: Job? = null

    /** The screen state. */
    val state: StateFlow<ExportUiState> = mutable.asStateFlow()

    /** Re-checks the rights, then the volumes. */
    fun refresh() {
        viewModelScope.launch {
            access = exporter.access()
            accessKnown = true
            idle()
        }
    }

    /** Re-reads the volumes only. */
    fun refreshVolumes() = idle()

    /** Works with the volume [uuid] (several can be mounted). */
    fun selectVolume(uuid: String) {
        selected = uuid
        pickError = null
        idle()
    }

    /** The picker intent for the volume the screen works with, null when there is none. */
    fun pickerIntent(): Intent? = currentVolume?.let { probe.pickerIntent(it.uuid) }

    /** The picker returned the tree [tree]: store it, or say why it is refused. */
    fun onFolderPicked(tree: Uri) {
        val volume = currentVolume ?: return
        pickError =
            when (val taken = folders.take(volume.uuid, tree)) {
                is TakeResult.Taken -> null
                is TakeResult.Refused -> taken.problem
            }
        idle()
    }

    /** "Change folder": releases the grant of the volume and asks for another folder. */
    fun changeFolder() {
        val volume = currentVolume ?: return
        folders.change(volume.uuid)
        pickError = null
        idle()
    }

    /** Starts the export into the stored folder of the volume, then shows its result until [dismiss]. */
    fun start() {
        val ready = mutable.value as? ExportUiState.Ready ?: return
        val tree = folders.folder(ready.volume.uuid)
        if (tree == null) {
            idle()
            return
        }
        job?.cancel()
        job =
            viewModelScope.launch {
                mutable.value = ExportUiState.Running(exporter.progress.value)
                val watcher =
                    launch {
                        exporter.progress.collect { if (mutable.value is ExportUiState.Running) mutable.value = ExportUiState.Running(it) }
                    }
                try {
                    val outcome = exporter.run(targets.open(tree, ready.volume.uuid), ready.folderName)
                    watcher.cancel()
                    mutable.value =
                        when (outcome) {
                            is ExportOutcome.Done -> ExportUiState.Done(outcome.result)
                            is ExportOutcome.Failed -> ExportUiState.Failed(outcome.failure)
                        }
                } finally {
                    watcher.cancel()
                }
            }
    }

    /** Leaves a Done or Failed screen and checks again. */
    fun dismiss() {
        val current = mutable.value
        if (current is ExportUiState.Done || current is ExportUiState.Failed) {
            mutable.value = ExportUiState.Checking
            refresh()
        }
    }

    private val currentVolume: RemovableVolume?
        get() =
            when (val s = mutable.value) {
                is ExportUiState.NeedsFolder -> s.volume
                is ExportUiState.Ready -> s.volume
                else -> null
            }

    private fun idle() {
        when (mutable.value) {
            is ExportUiState.Running, is ExportUiState.Done, is ExportUiState.Failed -> return
            else -> Unit
        }
        val problem = access
        mutable.value =
            when {
                !accessKnown -> ExportUiState.Checking
                problem == ExportProblem.NOT_MANAGER -> ExportUiState.NotManager
                problem != null -> ExportUiState.Blocked(problem)
                else -> volumeState()
            }
    }

    private fun volumeState(): ExportUiState {
        val all = probe.mounted()
        val volume =
            all.firstOrNull { it.uuid == selected }
                ?: all.firstOrNull { folders.folder(it.uuid) != null }
                ?: all.firstOrNull()
        selected = volume?.uuid
        val tree = volume?.let { folders.folder(it.uuid) }
        return when {
            volume == null -> ExportUiState.NoStick
            tree == null -> ExportUiState.NeedsFolder(all, volume, pickError)
            else -> ExportUiState.Ready(all, volume, ExportFolderStore.displayName(tree))
        }
    }
}
