package dev.nuagemagique.pilipos.platform.update

import android.app.Activity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import dagger.hilt.android.EntryPointAccessors
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.platform.PlatformBackButton
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

@Composable
private fun rememberUpdateViewModel(): UpdateViewModel {
    val appContext = LocalContext.current.applicationContext
    val graph = remember(appContext) { EntryPointAccessors.fromApplication(appContext, UpdateScreenEntryPoint::class.java) }
    return viewModel {
        val identity = graph.identity()
        UpdateViewModel(
            preconditions = graph.preconditions(),
            installer = graph.installer(),
            verifier = graph.verifier(),
            installedInfo = { identity.report().let { InstalledInfo(it.versionName, it.versionCode, it.apkSha1) } },
            sources = graph.sources(),
            volumes = graph.volumeProbe(),
        )
    }
}

/**
 * The update screen (route `platform/update`, manager only). The folder picker is started from the activity
 * with the volume's own intent (never with `FLAG_ACTIVITY_NEW_TASK`, which Lock Task refuses).
 */
@Composable
fun UpdateScreen(
    onBack: () -> Unit = {},
    viewModel: UpdateViewModel = rememberUpdateViewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val picker =
        rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            if (result.resultCode == Activity.RESULT_OK) result.data?.data?.let(viewModel::onFolderPicked)
        }
    val owner = LocalLifecycleOwner.current
    LaunchedEffect(owner) {
        owner.lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) { viewModel.refresh() }
    }
    UpdateContent(
        state = state,
        onBack = onBack,
        actions =
            UpdateActions(
                onCheckOnline = viewModel::checkOnline,
                onFromUsb = { viewModel.pickerIntent()?.let(picker::launch) },
                onInstall = viewModel::install,
            ),
    )
}

/** What the buttons of [UpdateContent] do. */
internal class UpdateActions(
    val onCheckOnline: () -> Unit = {},
    val onFromUsb: () -> Unit = {},
    val onInstall: () -> Unit = {},
)

/** The stateless body of [UpdateScreen]. */
@Composable
internal fun UpdateContent(
    state: UpdateUiState,
    actions: UpdateActions,
    onBack: () -> Unit = {},
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Box(modifier = Modifier.fillMaxSize().background(colors.bgPrimary).testTag("update_root")) {
        Column(
            modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(dimens.spaceLg),
            verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(dimens.spaceMd)) {
                PlatformBackButton(onBack = onBack, tag = "update_back")
                Text(text = stringResource(R.string.plat_update_title), style = PiliTheme.typography.title, color = colors.textPrimary)
            }
            state.installed?.let { InstalledPanel(it) }
            if (state.notManager) {
                Message(R.string.plat_update_not_manager, "update_not_manager", warning = true)
            } else {
                Body(state, actions)
            }
        }
    }
}

@Composable
private fun Body(
    state: UpdateUiState,
    actions: UpdateActions,
) {
    if (state.phase == UpdatePhase.Installing) {
        InstallingPanel()
        return
    }
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
        ActionButton(R.string.plat_update_check_online, "update_check_online", actions.onCheckOnline, enabled = state.canCheck)
        ActionButton(R.string.plat_update_from_usb, "update_from_usb", actions.onFromUsb, enabled = state.canCheck)
    }
    when (val phase = state.phase) {
        is UpdatePhase.Checking -> Message(R.string.plat_update_checking, "update_checking")
        UpdatePhase.NoStick -> Message(R.string.plat_update_no_stick, "update_no_stick")
        is UpdatePhase.Verified -> ResultPanel(phase.update)
        is UpdatePhase.Refused -> RefusalPanel(phase.refusal)
        is UpdatePhase.InstallFailed -> InstallFailedPanel(phase)
        UpdatePhase.Idle, UpdatePhase.Installing -> Unit
    }
    if (state.failures.isNotEmpty()) FailuresPanel(state.failures)
    if (state.phase is UpdatePhase.Verified) {
        ActionButton(R.string.plat_update_install, "update_install", actions.onInstall, primary = true, enabled = state.canInstall)
    }
}
