package dev.nuagemagique.pilipos.platform.export

import android.app.Activity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
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
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.delay

/** How often the screen looks for a stick that was inserted or removed. */
private const val VOLUME_POLL_MS = 2_000L

@Composable
private fun rememberExportViewModel(): ExportViewModel {
    val appContext = LocalContext.current.applicationContext
    val graph = remember(appContext) { EntryPointAccessors.fromApplication(appContext, ExportEntryPoint::class.java) }
    return viewModel { ExportViewModel(graph.exporter(), graph.volumeProbe(), graph.folderStore(), graph.targets()) }
}

/**
 * The SPF export screen (route `platform/export`, manager only). The folder picker is started from the
 * activity with the volume's own intent (never with `FLAG_ACTIVITY_NEW_TASK`, which Lock Task refuses).
 */
@Composable
fun ExportScreen(viewModel: ExportViewModel = rememberExportViewModel()) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val picker =
        rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            if (result.resultCode == Activity.RESULT_OK) result.data?.data?.let(viewModel::onFolderPicked)
        }
    val owner = LocalLifecycleOwner.current
    LaunchedEffect(owner) {
        owner.lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            viewModel.refresh()
            while (true) {
                delay(VOLUME_POLL_MS)
                viewModel.refreshVolumes()
            }
        }
    }
    ExportContent(
        state = state,
        actions =
            ExportActions(
                onPick = { viewModel.pickerIntent()?.let(picker::launch) },
                onChangeFolder = viewModel::changeFolder,
                onStart = viewModel::start,
                onDismiss = viewModel::dismiss,
                onSelectVolume = viewModel::selectVolume,
            ),
    )
}

/** What the buttons of [ExportContent] do. */
internal class ExportActions(
    val onPick: () -> Unit = {},
    val onChangeFolder: () -> Unit = {},
    val onStart: () -> Unit = {},
    val onDismiss: () -> Unit = {},
    val onSelectVolume: (String) -> Unit = {},
)

/** The stateless body of [ExportScreen]. */
@Composable
internal fun ExportContent(
    state: ExportUiState,
    actions: ExportActions,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Box(modifier = Modifier.fillMaxSize().background(colors.bgPrimary).testTag("export_root")) {
        Column(
            modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(dimens.spaceLg),
            verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
        ) {
            Text(text = stringResource(R.string.plat_export_title), style = PiliTheme.typography.title, color = colors.textPrimary)
            when (state) {
                ExportUiState.Checking -> Message(R.string.plat_export_checking, "export_checking")
                ExportUiState.NotManager -> Message(R.string.plat_export_not_manager, "export_not_manager", warning = true)
                is ExportUiState.Blocked -> Message(state.problem.textRes, "export_blocked", warning = true)
                ExportUiState.NoStick -> Message(R.string.plat_export_no_stick, "export_no_stick")
                is ExportUiState.NeedsFolder -> NeedsFolder(state, actions)
                is ExportUiState.Ready -> Ready(state, actions)
                is ExportUiState.Running -> Running(state.progress)
                is ExportUiState.Done -> Done(state.result, actions)
                is ExportUiState.Failed -> Failed(state.failure, actions)
            }
        }
    }
}
