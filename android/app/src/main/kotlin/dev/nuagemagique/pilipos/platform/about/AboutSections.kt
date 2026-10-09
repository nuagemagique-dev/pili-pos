package dev.nuagemagique.pilipos.platform.about

import android.text.format.Formatter
import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextStyle
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

// The four sections of the About screen and their field rows.

@Composable
internal fun Sections(report: IdentityReport) {
    val dimens = PiliTheme.dimens
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(dimens.spaceMd)) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(dimens.spaceMd)) {
            VersionSection(report)
            IntegritySection(report)
        }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(dimens.spaceMd)) {
            DeviceSection(report)
            StorageSection(report)
        }
    }
}

@Composable
private fun VersionSection(report: IdentityReport) {
    Section(R.string.plat_about_section_version) {
        Field(R.string.plat_about_version_name, report.versionName)
        Field(R.string.plat_about_version_code, report.versionCode.toString())
        Field(R.string.plat_about_pos_sw_version, report.posSwVersion)
        Field(R.string.plat_about_engine_ref_label, report.engineRef)
    }
}

@Composable
private fun IntegritySection(report: IdentityReport) {
    val unavailable = stringResource(R.string.plat_about_unavailable)
    val soText = report.soSha1 ?: report.soProblem?.let { "$unavailable ($it)" }
    Section(R.string.plat_about_section_integrity) {
        Field(R.string.plat_about_apk_sha1, report.apkSha1, tag = "about_apk_sha1", mono = true)
        Field(R.string.plat_about_apk_sha256, report.apkSha256, mono = true)
        Field(R.string.plat_about_so_sha1, soText, tag = "about_so_sha1", mono = true)
        Field(
            R.string.plat_about_signer_sha256,
            report.signerSha256.takeIf { it.isNotEmpty() }?.joinToString("\n"),
            mono = true,
        )
    }
}

@Composable
private fun DeviceSection(report: IdentityReport) {
    Section(R.string.plat_about_section_device) {
        Field(R.string.plat_about_device_id, report.deviceId, tag = "about_device_id", mono = true)
        Field(R.string.plat_about_pos_id, report.posId, mono = true)
        Field(R.string.plat_about_terminal_id, report.terminalId, mono = true)
        Field(R.string.plat_about_android, stringResource(R.string.plat_about_android_value, report.androidRelease, report.sdkInt))
        Field(R.string.plat_about_model, "${report.manufacturer} ${report.model}".trim())
    }
}

@Composable
private fun StorageSection(report: IdentityReport) {
    val context = LocalContext.current
    val dimens = PiliTheme.dimens
    Section(R.string.plat_about_section_storage) {
        Field(R.string.plat_about_free_space, Formatter.formatShortFileSize(context, report.freeInternalBytes), tag = "about_free_space")
        Field(
            R.string.plat_about_backups,
            stringResource(
                R.string.plat_about_backups_value,
                report.backupsCount,
                Formatter.formatShortFileSize(context, report.backupsBytes),
            ),
            tag = "about_backups",
        )
        Column(verticalArrangement = Arrangement.spacedBy(dimens.spaceXs)) {
            if (report.lowFreeSpace) Warning(R.string.plat_about_warning_low_space, "about_warning_low_space")
            if (report.backupsOverHalfOfFree) Warning(R.string.plat_about_warning_backups, "about_warning_backups")
        }
    }
}

@Composable
private fun Warning(
    @StringRes messageRes: Int,
    tag: String,
) {
    Text(
        text = stringResource(messageRes),
        style = PiliTheme.typography.body,
        color = PiliTheme.colors.accentWarning,
        modifier = Modifier.testTag(tag),
    )
}

@Composable
private fun Section(
    @StringRes titleRes: Int,
    content: @Composable () -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(colors.bgSecondary, RoundedCornerShape(dimens.radiusLg))
                .padding(dimens.spaceMd),
        verticalArrangement = Arrangement.spacedBy(dimens.spaceSm),
    ) {
        Text(text = stringResource(titleRes), style = PiliTheme.typography.label, color = colors.accentInfo)
        content()
    }
}

@Composable
private fun Field(
    @StringRes labelRes: Int,
    value: String?,
    tag: String? = null,
    mono: Boolean = false,
) {
    val colors = PiliTheme.colors
    val type = PiliTheme.typography
    val shown = value ?: stringResource(R.string.plat_about_unavailable)
    val style: TextStyle = if (mono) type.figure.copy(fontSize = type.sizeSm) else type.body
    Column {
        Text(text = stringResource(labelRes), style = type.body.copy(fontSize = type.sizeXs), color = colors.textSecondary)
        Text(
            text = shown,
            style = style,
            color = if (value == null) colors.textSecondary else colors.textPrimary,
            modifier = if (tag != null) Modifier.testTag(tag) else Modifier,
        )
    }
}
