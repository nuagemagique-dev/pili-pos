package dev.nuagemagique.pilipos.shell

import android.net.Uri
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.hilt.lifecycle.viewmodel.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.nav.AppNavHost
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.provisioning.BundleSource
import dev.nuagemagique.pilipos.ui.theme.PiliDensity
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * Root of the Compose UI: switches between the provisioning, device check, opening and fatal screens and
 * the navigation host (with the registry overlays above it) once the engine is ready.
 */
@Composable
fun ShellRoot(viewModel: ShellViewModel = hiltViewModel()) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    var navigator by remember { mutableStateOf<AppNavigator?>(null) }
    val currentNavigator by rememberUpdatedState(navigator)

    LaunchedEffect(viewModel) {
        viewModel.effects.collect { effect ->
            when (effect) {
                ShellEffect.NavigateToLogin -> currentNavigator?.toLogin()
            }
        }
    }

    PiliTheme {
        PiliDensity {
            Box(modifier = Modifier.fillMaxSize().testTag("shell_root")) {
                ShellContent(
                    state = state,
                    onImportTree = { uri: Uri -> viewModel.importFrom(BundleSource.TreeUri(uri)) },
                    onImportInbox = if (BuildConfig.DEBUG) viewModel::importFromInbox else null,
                ) {
                    ReadyContent(registry = viewModel.registry, onNavigator = { navigator = it }, onGone = { navigator = null })
                }
            }
        }
    }
}

/** The navigation host and, above it, every overlay of the registry. */
@Composable
private fun ReadyContent(
    registry: NavRegistry,
    onNavigator: (AppNavigator) -> Unit,
    onGone: () -> Unit,
) {
    DisposableEffect(Unit) { onDispose(onGone) }
    Box(modifier = Modifier.fillMaxSize()) {
        AppNavHost(registry = registry, onNavigator = onNavigator)
        registry.overlays.forEach { it.Content() }
    }
}

/**
 * The screen of [state]. Stateless: [readyContent] is shown when the engine is ready, the import callbacks
 * start an import ([onImportInbox] is null outside debug builds).
 */
@Composable
fun ShellContent(
    state: ShellState,
    onImportTree: (Uri) -> Unit,
    onImportInbox: (() -> Unit)?,
    readyContent: @Composable () -> Unit,
) {
    when (state) {
        ShellState.NeedsDeviceOwner -> OwnerRequiredScreen()
        is ShellState.NotProvisioned -> ProvisioningScreen(state.importing, state.problem, onImportTree, onImportInbox)
        is ShellState.DeviceMismatch ->
            DeviceMismatchScreen(state.stored, state.actual, state.importing, state.problem, onImportTree, onImportInbox)
        is ShellState.DeviceIdUnavailable -> DeviceIdScreen(state.detail)
        ShellState.Opening -> OpeningScreen()
        is ShellState.Ready -> readyContent()
        is ShellState.Fatal -> FatalScreen(state.reason)
    }
}
