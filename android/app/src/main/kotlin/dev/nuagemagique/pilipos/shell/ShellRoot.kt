package dev.nuagemagique.pilipos.shell

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import dev.nuagemagique.pilipos.R

/**
 * Root of the Compose UI.
 *
 * Placeholder: P4-T11 replaces it with the shell (navigation host, status line, kiosk surface).
 */
@Composable
fun ShellRoot() {
    Box(modifier = Modifier.fillMaxSize().testTag("shell_root"), contentAlignment = Alignment.Center) {
        Text(text = stringResource(R.string.shell_app_name))
    }
}
