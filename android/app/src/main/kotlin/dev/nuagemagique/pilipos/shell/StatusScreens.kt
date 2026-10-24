package dev.nuagemagique.pilipos.shell

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The app is not the device owner (PD108): nothing can start. Debug builds also show the adb command. */
@Composable
internal fun OwnerRequiredScreen() {
    ShellPanel(tag = "shell_owner_required") {
        PanelTitle(stringResource(R.string.prov_owner_required_title))
        PanelBody(stringResource(R.string.prov_owner_required_body))
        if (BuildConfig.DEBUG) {
            val command = "adb shell dpm set-device-owner ${LocalContext.current.packageName}/$ADMIN_RECEIVER"
            PanelBody(text = stringResource(R.string.shell_owner_adb_hint), secondary = true)
            Text(
                text = command,
                style = PiliTheme.typography.figure,
                color = PiliTheme.colors.textPrimary,
                textAlign = TextAlign.Center,
                modifier = Modifier.testTag("shell_owner_adb"),
            )
        }
    }
}

/** The device serial cannot be read: the till does not start without it. */
@Composable
internal fun DeviceIdScreen(detail: String) {
    ShellPanel(tag = "shell_device_id") {
        PanelTitle(stringResource(R.string.prov_device_id_title))
        PanelBody(fillPlaceholders(stringResource(R.string.prov_device_id_body), "detail" to detail))
    }
}

/** The engine is starting. */
@Composable
internal fun OpeningScreen() {
    ShellPanel(tag = "shell_opening") {
        PanelProgress(label = stringResource(R.string.shell_opening), tag = "shell_opening")
    }
}

/** The engine failed for good: the till must restart; the technical [reason] is for the technician. */
@Composable
internal fun FatalScreen(reason: String) {
    ShellPanel(tag = "shell_fatal") {
        PanelTitle(stringResource(R.string.shell_fatal_title))
        PanelBody(stringResource(R.string.shell_fatal_body))
        Text(
            text = reason,
            style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeXs),
            color = PiliTheme.colors.textSecondary,
            textAlign = TextAlign.Center,
            modifier = Modifier.testTag("shell_fatal_reason"),
        )
    }
}

private const val ADMIN_RECEIVER = "dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver"
