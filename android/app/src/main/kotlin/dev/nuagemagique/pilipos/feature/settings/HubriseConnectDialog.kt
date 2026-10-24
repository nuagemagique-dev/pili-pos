@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.components.QrCode
import dev.nuagemagique.pilipos.ui.components.QrMatrices
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** What the connect dialog calls: the typed code, and the cancel. */
internal data class HubriseConnectActions(
    val onSubmit: (String) -> Unit = {},
    val onCancel: () -> Unit = {},
)

/** The widest the connect dialog grows: the QR code on the left, the steps on the right. */
private val ConnectDialogMaxWidth = 900.dp

/**
 * The HubRise connect dialog (`HubriseConnectDialog.vue`). The till has no browser, so the HubRise login [link] is
 * drawn as a QR code to open on a phone (PD218: from `EngineHelpers.qrMatrix` through [matrices], never from the
 * engine's `qr_svg`), next to the scan instructions and the link itself. The operator types the code HubRise
 * shows once the location is picked; the confirm button is off while the code is blank or [loading], and a
 * refusal ([error], already turned into text) shows under the field. A tap on the scrim or on the back button
 * calls [HubriseConnectActions.onCancel].
 * Test tags: `hubrise_dialog`, `qr_code`, `hubrise_link`, `hubrise_code`, `hubrise_dialog_error`,
 * `hubrise_cancel`, `hubrise_confirm`.
 */
@Composable
internal fun HubriseConnectDialog(
    link: String,
    matrices: QrMatrices,
    loading: Boolean,
    error: String?,
    actions: HubriseConnectActions,
    modifier: Modifier = Modifier,
) {
    var code by remember { mutableStateOf("") }
    val canSubmit = !loading && code.isNotBlank()
    val submit = { if (canSubmit) actions.onSubmit(code.trim()) }
    val qrDescription = uiText(R.string.ui_settings_qr_description)
    PiliDialog(
        title = vueText("hubrise_open_authorize_url"),
        onDismissRequest = actions.onCancel,
        modifier = modifier.testTag("hubrise_dialog"),
        maxWidth = ConnectDialogMaxWidth,
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = actions.onCancel,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("hubrise_cancel"),
            )
            PiliButton(
                text = vueText("hubrise_complete_connection"),
                onClick = submit,
                enabled = canSubmit,
                modifier = Modifier.testTag("hubrise_confirm"),
            )
        },
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceLg), verticalAlignment = Alignment.CenterVertically) {
            QrCode(link, matrices, Modifier.semantics { contentDescription = qrDescription })
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
                Text(vueText("hubrise_authorize_scan"), style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
                Text(
                    link,
                    style = PiliTheme.typography.figure,
                    color = PiliTheme.colors.textPrimary,
                    modifier = Modifier.testTag("hubrise_link"),
                )
            }
        }
        SettingsTextField(
            value = code,
            onValueChange = { code = it },
            placeholder = vueText("hubrise_code_placeholder"),
            modifier = Modifier.fillMaxWidth().testTag("hubrise_code"),
            mono = true,
            onDone = { submit() },
        )
        if (error != null) {
            Text(
                error,
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.accentDanger,
                modifier = Modifier.testTag("hubrise_dialog_error"),
            )
        }
    }
}
