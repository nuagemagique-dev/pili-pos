package dev.nuagemagique.pilipos.platform.update

import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

@Composable
internal fun InstalledPanel(info: InstalledInfo) {
    val type = PiliTheme.typography
    val colors = PiliTheme.colors
    Panel(tag = "update_installed") {
        Text(
            text = stringResource(R.string.plat_update_installed, info.versionName, info.versionCode),
            style = type.label,
            color = colors.textPrimary,
        )
        SelectionContainer {
            Text(
                text = stringResource(R.string.plat_update_installed_sha1, info.apkSha1 ?: stringResource(R.string.plat_update_unknown)),
                style = type.figure.copy(fontSize = type.sizeSm),
                color = colors.textSecondary,
                modifier = Modifier.testTag("update_installed_sha1"),
            )
        }
    }
}

@Composable
internal fun ResultPanel(update: VerifiedUpdate) {
    val type = PiliTheme.typography
    val colors = PiliTheme.colors
    Panel(tag = "update_result") {
        Text(text = stringResource(R.string.plat_update_result_title), style = type.label, color = colors.accentSuccess)
        SelectionContainer {
            Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                Text(
                    text = stringResource(R.string.plat_update_result_version, update.release.version, update.release.versionCode),
                    style = type.body,
                    color = colors.textPrimary,
                    modifier = Modifier.testTag("update_result_version"),
                )
                Text(
                    text = stringResource(R.string.plat_update_result_sha1, update.sha1),
                    style = type.figure.copy(fontSize = type.sizeSm),
                    color = colors.textPrimary,
                    modifier = Modifier.testTag("update_result_sha1"),
                )
                Text(
                    text = stringResource(R.string.plat_update_result_signer, update.release.signerSha256),
                    style = type.figure.copy(fontSize = type.sizeSm),
                    color = colors.textPrimary,
                    modifier = Modifier.testTag("update_result_signer"),
                )
            }
        }
        Message(R.string.plat_update_install_hint, "update_install_hint")
    }
}

@Composable
internal fun RefusalPanel(refusal: UpdateRefusal) {
    val type = PiliTheme.typography
    val colors = PiliTheme.colors
    Panel(tag = "update_refusal") {
        Text(text = stringResource(R.string.plat_update_refused_title), style = type.label, color = colors.accentDanger)
        Message(refusal.code.messageRes, "update_refusal_text", warning = true)
        Text(
            text = stringResource(R.string.plat_update_refused_detail, refusal.detail),
            style = type.body,
            color = colors.textSecondary,
            modifier = Modifier.testTag("update_refusal_detail"),
        )
    }
}

@Composable
internal fun InstallFailedPanel(phase: UpdatePhase.InstallFailed) {
    val type = PiliTheme.typography
    val colors = PiliTheme.colors
    Panel(tag = "update_install_failed") {
        Text(text = stringResource(R.string.plat_update_install_failed_title), style = type.label, color = colors.accentDanger)
        Message(phase.reason.messageRes, "update_install_failed_text", warning = true)
        phase.detail?.takeIf { it.isNotBlank() }?.let {
            Text(text = stringResource(R.string.plat_update_refused_detail, it), style = type.body, color = colors.textSecondary)
        }
    }
}

@Composable
internal fun FailuresPanel(failures: List<PreconditionFailure>) {
    val colors = PiliTheme.colors
    Panel(tag = "update_failures") {
        Text(text = stringResource(R.string.plat_update_blocked_title), style = PiliTheme.typography.label, color = colors.accentWarning)
        failures.forEach { Message(it.messageRes, "update_failure_${it.id}", warning = true) }
    }
}

@Composable
internal fun InstallingPanel() {
    val colors = PiliTheme.colors
    Panel(tag = "update_installing") {
        Text(text = stringResource(R.string.plat_update_installing), style = PiliTheme.typography.label, color = colors.textPrimary)
        CircularProgressIndicator(color = colors.accentPrimary)
        Message(R.string.plat_update_installing_hint, "update_installing_hint")
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
internal fun ActionButton(
    @StringRes textRes: Int,
    tag: String,
    onClick: () -> Unit,
    primary: Boolean = false,
    enabled: Boolean = true,
) {
    val colors = PiliTheme.colors
    Button(
        onClick = onClick,
        enabled = enabled,
        colors =
            ButtonDefaults.buttonColors(
                containerColor = if (primary) colors.accentPrimary else colors.bgSurface,
                contentColor = if (primary) colors.textOnAccent else colors.textPrimary,
            ),
        modifier = Modifier.testTag(tag),
    ) { Text(stringResource(textRes), style = PiliTheme.typography.label) }
}
