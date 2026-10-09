package dev.nuagemagique.pilipos.platform.about

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import dagger.hilt.android.EntryPointAccessors
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

@Composable
private fun rememberAboutViewModel(): AboutViewModel {
    val appContext = LocalContext.current.applicationContext
    val identity = remember(appContext) { EntryPointAccessors.fromApplication(appContext, AboutEntryPoint::class.java).identityProvider() }
    return viewModel { AboutViewModel(identity) }
}

/** The About screen (route `platform/about`): version, file hashes, device and storage. */
@Composable
fun AboutScreen(viewModel: AboutViewModel = rememberAboutViewModel()) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    AboutContent(state = state, onRefresh = viewModel::refresh)
}

/** The stateless body of [AboutScreen]. */
@Composable
internal fun AboutContent(
    state: AboutUiState,
    onRefresh: () -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    val type = PiliTheme.typography
    Box(
        modifier = Modifier.fillMaxSize().background(colors.bgPrimary).testTag("about_root"),
    ) {
        when (state) {
            AboutUiState.Loading ->
                Text(
                    text = stringResource(R.string.plat_about_loading),
                    style = type.body,
                    color = colors.textSecondary,
                    modifier = Modifier.align(Alignment.Center),
                )
            is AboutUiState.Loaded ->
                Column(
                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(dimens.spaceLg),
                    verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
                ) {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = stringResource(R.string.plat_about_title),
                            style = type.title,
                            color = colors.textPrimary,
                            modifier = Modifier.weight(1f),
                        )
                        Button(
                            onClick = onRefresh,
                            colors = ButtonDefaults.buttonColors(containerColor = colors.bgSurface, contentColor = colors.textPrimary),
                            modifier = Modifier.testTag("about_refresh"),
                        ) { Text(stringResource(R.string.plat_about_refresh), style = type.label) }
                    }
                    SelectionContainer { Sections(state.report) }
                }
        }
    }
}
