package dev.nuagemagique.pilipos.platform.export

import android.text.format.Formatter
import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

@Composable
internal fun NeedsFolder(
    state: ExportUiState.NeedsFolder,
    actions: ExportActions,
) {
    Panel {
        VolumeChoice(state.volumes, state.volume, actions.onSelectVolume)
        Message(R.string.plat_export_need_folder, "export_need_folder")
        state.error?.let { Message(it.textRes, "export_error", warning = true) }
        ActionButton(R.string.plat_export_pick_folder, "export_pick", actions.onPick)
    }
}

@Composable
internal fun Ready(
    state: ExportUiState.Ready,
    actions: ExportActions,
) {
    Panel {
        VolumeChoice(state.volumes, state.volume, actions.onSelectVolume)
        Text(
            text = stringResource(R.string.plat_export_folder, state.folderName),
            style = PiliTheme.typography.body,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.testTag("export_folder"),
        )
        Message(R.string.plat_export_ready_hint, "export_hint")
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
            ActionButton(R.string.plat_export_start, "export_start", actions.onStart, primary = true)
            ActionButton(R.string.plat_export_change_folder, "export_change", actions.onChangeFolder)
        }
    }
}

@Composable
private fun VolumeChoice(
    volumes: List<RemovableVolume>,
    chosen: RemovableVolume,
    onSelect: (String) -> Unit,
) {
    val colors = PiliTheme.colors
    Text(
        text = stringResource(R.string.plat_export_volume, chosen.label()),
        style = PiliTheme.typography.label,
        color = colors.accentInfo,
        modifier = Modifier.testTag("export_volume"),
    )
    if (volumes.size > 1) {
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            volumes.filter { it.uuid != chosen.uuid }.forEach { other ->
                Button(
                    onClick = { onSelect(other.uuid) },
                    colors = ButtonDefaults.buttonColors(containerColor = colors.bgSurface, contentColor = colors.textPrimary),
                    modifier = Modifier.testTag("export_volume_${other.uuid}"),
                ) { Text(other.label(), style = PiliTheme.typography.label) }
            }
        }
    }
}

private fun RemovableVolume.label(): String = description.ifBlank { uuid }

@Composable
internal fun Running(progress: ExportProgress) {
    val context = LocalContext.current
    val type = PiliTheme.typography
    val colors = PiliTheme.colors
    Panel(tag = "export_progress") {
        Text(text = stringResource(R.string.plat_export_running), style = type.body, color = colors.textPrimary)
        val fraction = if (progress.bytesTotal > 0) (progress.bytesDone.toFloat() / progress.bytesTotal).coerceIn(0f, 1f) else 0f
        LinearProgressIndicator(
            progress = { fraction },
            modifier = Modifier.fillMaxWidth(),
            color = colors.accentPrimary,
            trackColor = colors.bgSurface,
        )
        Text(
            text =
                stringResource(
                    R.string.plat_export_progress,
                    progress.filesDone,
                    progress.filesTotal,
                    Formatter.formatShortFileSize(context, progress.bytesDone),
                    Formatter.formatShortFileSize(context, progress.bytesTotal),
                ),
            style = type.figure,
            color = colors.textSecondary,
        )
        progress.current?.let {
            Text(text = stringResource(R.string.plat_export_current, it), style = type.body, color = colors.textSecondary)
        }
    }
}

@Composable
internal fun Done(
    result: ExportResult,
    actions: ExportActions,
) {
    val context = LocalContext.current
    val type = PiliTheme.typography
    val colors = PiliTheme.colors
    Panel(tag = "export_done") {
        Text(text = stringResource(R.string.plat_export_done_title), style = type.label, color = colors.accentSuccess)
        SelectionContainer {
            Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                Text(
                    text = stringResource(R.string.plat_export_done_folder, result.chosenFolder, result.exportFolder),
                    style = type.body,
                    color = colors.textPrimary,
                )
                Text(
                    text =
                        stringResource(
                            R.string.plat_export_done_files,
                            result.files,
                            Formatter.formatShortFileSize(context, result.bytes),
                        ),
                    style = type.body,
                    color = colors.textPrimary,
                )
                Text(text = stringResource(R.string.plat_export_done_manifest), style = type.body, color = colors.textSecondary)
                Text(
                    text = result.manifestSha256,
                    style = type.figure.copy(fontSize = type.sizeSm),
                    color = colors.textPrimary,
                    modifier = Modifier.testTag("export_manifest_sha"),
                )
            }
        }
        Message(R.string.plat_export_done_hint, "export_done_hint")
        ActionButton(R.string.plat_export_dismiss, "export_dismiss", actions.onDismiss)
    }
}

@Composable
internal fun Failed(
    failure: ExportFailure,
    actions: ExportActions,
) {
    val type = PiliTheme.typography
    val colors = PiliTheme.colors
    Panel(tag = "export_failed") {
        Text(text = stringResource(R.string.plat_export_failed_title), style = type.label, color = colors.accentDanger)
        Message(failure.problem.textRes, "export_failed_reason", warning = true)
        Text(
            text = stringResource(R.string.plat_export_failed_step, stringResource(failure.step.textRes)),
            style = type.body,
            color = colors.textPrimary,
        )
        failure.file?.let {
            Text(text = stringResource(R.string.plat_export_failed_file, it), style = type.body, color = colors.textPrimary)
        }
        failure.detail?.takeIf { it.isNotBlank() }?.let {
            Text(text = stringResource(R.string.plat_export_failed_detail, it), style = type.body, color = colors.textSecondary)
        }
        if (failure.step != ExportStep.PRECHECK) Message(R.string.plat_export_failed_kept, "export_failed_kept")
        ActionButton(R.string.plat_export_dismiss, "export_dismiss", actions.onDismiss)
    }
}

@Composable
private fun Panel(
    tag: String? = null,
    content: @Composable () -> Unit,
) {
    val dimens = PiliTheme.dimens
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(PiliTheme.colors.bgSecondary, RoundedCornerShape(dimens.radiusLg))
                .padding(dimens.spaceMd)
                .then(if (tag != null) Modifier.testTag(tag) else Modifier),
        verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
    ) { content() }
}

@Composable
internal fun Message(
    @StringRes textRes: Int,
    tag: String,
    warning: Boolean = false,
) {
    val colors = PiliTheme.colors
    Text(
        text = stringResource(textRes),
        style = PiliTheme.typography.body,
        color = if (warning) colors.accentWarning else colors.textSecondary,
        modifier = Modifier.testTag(tag),
    )
}

@Composable
private fun ActionButton(
    @StringRes textRes: Int,
    tag: String,
    onClick: () -> Unit,
    primary: Boolean = false,
) {
    val colors = PiliTheme.colors
    Button(
        onClick = onClick,
        colors =
            ButtonDefaults.buttonColors(
                containerColor = if (primary) colors.accentPrimary else colors.bgSurface,
                contentColor = if (primary) colors.textOnAccent else colors.textPrimary,
            ),
        modifier = Modifier.testTag(tag),
    ) { Text(stringResource(textRes), style = PiliTheme.typography.label) }
}
