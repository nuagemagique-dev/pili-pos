package dev.nuagemagique.pilipos.platform.update

import android.content.Intent
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.platform.export.VolumeProbe
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import timber.log.Timber

/** The installed application as the screen shows it. */
data class InstalledInfo(
    val versionName: String,
    val versionCode: Long,
    val apkSha1: String?,
)

/** Where the update screen is. */
sealed interface UpdatePhase {
    /** Nothing was checked yet. */
    data object Idle : UpdatePhase

    /** A source is being read and verified. */
    data class Checking(
        val source: String,
    ) : UpdatePhase

    /** "From USB stick" was pressed with no removable volume mounted. */
    data object NoStick : UpdatePhase

    /** A release passed every check; its APK is ready in app-private storage. */
    data class Verified(
        val update: VerifiedUpdate,
    ) : UpdatePhase

    /** The release was refused. */
    data class Refused(
        val refusal: UpdateRefusal,
    ) : UpdatePhase

    /** The install was committed; the till restarts. */
    data object Installing : UpdatePhase

    /** The install stopped for [reason] before the commit; the engine is open again. */
    data class InstallFailed(
        val reason: InstallRefusal,
        val detail: String?,
    ) : UpdatePhase
}

/** What the update screen shows. */
data class UpdateUiState(
    val installed: InstalledInfo? = null,
    /** True once the preconditions were read at least once. */
    val checked: Boolean = false,
    val failures: List<PreconditionFailure> = emptyList(),
    val phase: UpdatePhase = UpdatePhase.Idle,
) {
    /** The user is not an administrator. */
    val notManager: Boolean get() = PreconditionFailure.NOT_MANAGER in failures

    /** A source can be asked: nobody is busy and the user is a manager. */
    val canCheck: Boolean get() = checked && !notManager && phase !is UpdatePhase.Checking && phase !is UpdatePhase.Installing

    /** The Install button is enabled: a verified update, no precondition failure. */
    val canInstall: Boolean get() = checked && failures.isEmpty() && phase is UpdatePhase.Verified
}

/**
 * The state machine of the update screen (PD213 to PD215). [refresh] reads the installed version and the
 * preconditions (call it when the screen comes to the front); [checkOnline] and [onFolderPicked] verify a
 * release; [install] runs [UpdateInstaller] for the verified one. Created by [UpdateScreen] with the
 * collaborators of the Hilt graph.
 */
class UpdateViewModel(
    private val preconditions: UpdatePreconditions,
    private val installer: UpdateInstaller,
    private val verifier: UpdateVerifier,
    private val installedInfo: suspend () -> InstalledInfo?,
    private val sources: UpdateSources,
    private val volumes: VolumeProbe,
) : ViewModel() {
    private val mutable = MutableStateFlow(UpdateUiState())
    private var job: Job? = null

    /** The screen state. */
    val state: StateFlow<UpdateUiState> = mutable.asStateFlow()

    /** Reads the installed version and the preconditions. */
    fun refresh() {
        viewModelScope.launch {
            val info = safeInstalled()
            val failures = preconditions.check()
            mutable.update { it.copy(installed = info ?: it.installed, checked = true, failures = failures) }
        }
    }

    /** "Check online": verifies the release declared in the registry. */
    fun checkOnline() {
        if (!mutable.value.canCheck) return
        verify("ghcr") { sources.ghcr() }
    }

    /**
     * "From USB stick": the picker intent of the first mounted removable volume, to be started from the
     * activity (never with `FLAG_ACTIVITY_NEW_TASK`). With no volume mounted the phase becomes
     * [UpdatePhase.NoStick] and the result is null.
     */
    fun pickerIntent(): Intent? {
        if (!mutable.value.canCheck) return null
        val intent = volumes.mounted().firstOrNull()?.let { volumes.pickerIntent(it.uuid) }
        if (intent == null) mutable.update { it.copy(phase = UpdatePhase.NoStick) }
        return intent
    }

    /** The picker returned the folder [tree]: verify the release in it. */
    fun onFolderPicked(tree: Uri) {
        if (!mutable.value.canCheck) return
        verify("usb") { sources.usb(tree) }
    }

    /** Installs the verified release; does nothing unless [UpdateUiState.canInstall]. */
    fun install() {
        val phase = mutable.value.phase
        if (!mutable.value.canInstall || phase !is UpdatePhase.Verified) return
        job?.cancel()
        job =
            viewModelScope.launch {
                mutable.update { it.copy(phase = UpdatePhase.Installing) }
                val outcome = installer.install(phase.update)
                mutable.update {
                    when (outcome) {
                        InstallOutcome.Committed -> it
                        is InstallOutcome.Blocked -> it.copy(failures = outcome.failures, phase = phase)
                        is InstallOutcome.Refused -> it.copy(phase = UpdatePhase.InstallFailed(outcome.reason, outcome.detail))
                    }
                }
            }
    }

    private fun verify(
        label: String,
        source: () -> UpdateSource,
    ) {
        job?.cancel()
        job =
            viewModelScope.launch {
                mutable.update { it.copy(phase = UpdatePhase.Checking(label)) }
                val result =
                    try {
                        verifier.verify(source())
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: UpdateException) {
                        UpdateRefusal(e.code, e.detail)
                    }
                val failures = preconditions.check()
                mutable.update {
                    it.copy(
                        failures = failures,
                        phase =
                            when (result) {
                                is VerifiedUpdate -> UpdatePhase.Verified(result)
                                is UpdateRefusal -> UpdatePhase.Refused(result)
                            },
                    )
                }
            }
    }

    private suspend fun safeInstalled(): InstalledInfo? =
        try {
            installedInfo()
        } catch (e: CancellationException) {
            throw e
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.w(e, "UpdateViewModel: the installed version is unavailable")
            null
        }
}
