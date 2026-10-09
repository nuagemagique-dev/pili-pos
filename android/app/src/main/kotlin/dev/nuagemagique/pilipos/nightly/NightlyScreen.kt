package dev.nuagemagique.pilipos.nightly

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.hilt.lifecycle.viewmodel.compose.hiltViewModel
import androidx.lifecycle.compose.LifecycleStartEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * The nightly export screen: reads the status every 30 s while it is visible and offers "retry now".
 * Managers only (the engine refuses the commands to anyone else; the settings row is managerOnly).
 */
@Composable
fun NightlyScreen(
    onBack: () -> Unit,
    viewModel: NightlyViewModel = hiltViewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LifecycleStartEffect(viewModel) {
        viewModel.startPolling()
        onStopOrDispose { viewModel.stopPolling() }
    }
    NightlyContent(state = state, onRetry = viewModel::runNow, onBack = onBack)
}

/** The stateless body of [NightlyScreen]. */
@Composable
internal fun NightlyContent(
    state: NightlyUiState,
    onRetry: () -> Unit,
    onBack: () -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(colors.bgPrimary)
                .verticalScroll(rememberScrollState())
                .padding(dimens.spaceLg)
                .testTag("nightly_screen"),
        verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
    ) {
        NightlyHeader(state = state, onRetry = onRetry, onBack = onBack)
        when (state) {
            NightlyUiState.Loading ->
                Text(
                    text = stringResource(R.string.nightly_loading),
                    style = PiliTheme.typography.body,
                    color = colors.textSecondary,
                    modifier = Modifier.testTag("nightly_loading"),
                )
            is NightlyUiState.Failed ->
                Text(
                    text = state.message,
                    style = PiliTheme.typography.body,
                    color = colors.accentDanger,
                    modifier = Modifier.testTag("nightly_failed"),
                )
            is NightlyUiState.Loaded -> NightlyStatusBody(state.status)
        }
    }
}

@Composable
private fun NightlyHeader(
    state: NightlyUiState,
    onRetry: () -> Unit,
    onBack: () -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(dimens.spaceMd)) {
        TextButton(onClick = onBack, modifier = Modifier.testTag("nightly_back")) {
            Text(
                text = stringResource(R.string.nightly_back),
                style = PiliTheme.typography.label,
                color = colors.textPrimary,
            )
        }
        Text(
            text = stringResource(R.string.nightly_title),
            style = PiliTheme.typography.title,
            color = colors.textPrimary,
        )
        Spacer(Modifier.weight(1f))
        if (state is NightlyUiState.Loaded && state.status.canRetry()) {
            RetryButton(running = state.running, onRetry = onRetry)
        }
    }
}

@Composable
private fun RetryButton(
    running: Boolean,
    onRetry: () -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Button(
        onClick = onRetry,
        enabled = !running,
        shape = RoundedCornerShape(dimens.radiusSm),
        colors =
            ButtonDefaults.buttonColors(
                containerColor = colors.bgSurface,
                contentColor = colors.textPrimary,
                disabledContainerColor = colors.bgSecondary,
                disabledContentColor = colors.textSecondary,
            ),
        modifier = Modifier.defaultMinSize(minHeight = dimens.spaceXl + dimens.spaceMd).testTag("nightly_retry"),
    ) {
        Text(
            text = stringResource(if (running) R.string.nightly_retrying else R.string.nightly_retry_now),
            style = PiliTheme.typography.label,
        )
    }
}
