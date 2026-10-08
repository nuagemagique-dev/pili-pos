package dev.nuagemagique.pilipos.shell

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem

/** Tag of the debug-only button importing from the inbox directory. */
internal const val TAG_IMPORT_INBOX = "prov_import_inbox"

/**
 * First screen of an unprovisioned till: instructions, the folder picker (Storage Access Framework tree, it
 * works in Lock Task and is launched from the activity, S4 Q17) and, in debug builds only, the inbox import.
 */
@Composable
internal fun ProvisioningScreen(
    importing: Boolean,
    problem: ProvisioningProblem?,
    onImportTree: (Uri) -> Unit,
    onImportInbox: (() -> Unit)?,
) {
    ShellPanel(tag = "shell_provisioning") {
        PanelTitle(stringResource(R.string.prov_title))
        PanelBody(stringResource(R.string.prov_instructions))
        ImportControls(importing, problem, onImportTree, onImportInbox)
    }
}

/** The installed bundle belongs to another device: same actions as [ProvisioningScreen], with the identifiers. */
@Composable
internal fun DeviceMismatchScreen(
    stored: String,
    actual: String,
    importing: Boolean,
    problem: ProvisioningProblem?,
    onImportTree: (Uri) -> Unit,
    onImportInbox: (() -> Unit)?,
) {
    ShellPanel(tag = "shell_mismatch") {
        PanelTitle(stringResource(R.string.prov_mismatch_title))
        PanelBody(
            fillPlaceholders(stringResource(R.string.prov_mismatch_body), "stored" to stored, "actual" to actual),
        )
        ImportControls(importing, problem, onImportTree, onImportInbox)
    }
}

@Composable
private fun ImportControls(
    importing: Boolean,
    problem: ProvisioningProblem?,
    onImportTree: (Uri) -> Unit,
    onImportInbox: (() -> Unit)?,
) {
    val picker =
        rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
            if (uri != null) onImportTree(uri)
        }
    if (problem != null) ProblemText(problem)
    if (importing) {
        PanelProgress(label = stringResource(R.string.prov_importing), tag = "prov_importing")
    } else {
        PrimaryButton(
            text = stringResource(R.string.prov_choose_folder),
            onClick = { picker.launch(null) },
            modifier = Modifier.testTag("prov_choose_folder"),
        )
        if (BuildConfig.DEBUG && onImportInbox != null) {
            SecondaryButton(
                text = stringResource(R.string.prov_import_inbox),
                onClick = onImportInbox,
                modifier = Modifier.testTag(TAG_IMPORT_INBOX),
            )
        }
    }
}
