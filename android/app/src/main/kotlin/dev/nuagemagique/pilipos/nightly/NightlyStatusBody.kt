package dev.nuagemagique.pilipos.nightly

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.ExportStatus

/** The state pill, the problem line and, when the export runs, the rows of the status. */
@Composable
internal fun NightlyStatusBody(status: ExportStatus) {
    val kind = status.kind()
    Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
        StatePill(kind = kind, pending = status.pending)
        status.problem?.let { problem ->
            Text(
                text = problem,
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.accentDanger,
                modifier = Modifier.testTag("nightly_problem"),
            )
        }
        if (kind == NightlyKind.Pending || kind == NightlyKind.Ok) StatusRows(status)
    }
}

@Composable
private fun StatePill(
    kind: NightlyKind,
    pending: Long,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    val tint: Color =
        when (kind) {
            NightlyKind.Invalid -> colors.accentDanger
            NightlyKind.Disabled -> colors.textSecondary
            NightlyKind.Pending -> colors.accentWarning
            NightlyKind.Ok -> colors.accentSuccess
        }
    val text =
        when (kind) {
            NightlyKind.Invalid -> stringResource(R.string.nightly_state_invalid)
            NightlyKind.Disabled -> stringResource(R.string.nightly_state_disabled)
            NightlyKind.Pending -> {
                val count = pending.coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
                pluralStringResource(R.plurals.nightly_state_pending, count, pending)
            }
            NightlyKind.Ok -> stringResource(R.string.nightly_state_ok)
        }
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(dimens.spaceSm),
        modifier =
            Modifier
                .border(dimens.spaceXs / 4, colors.border, RoundedCornerShape(dimens.radiusLg))
                .padding(horizontal = dimens.spaceMd, vertical = dimens.spaceSm),
    ) {
        Box(Modifier.size(dimens.spaceSm).background(tint, CircleShape))
        Text(
            text = text,
            style =
                PiliTheme.typography.figure.copy(
                    fontSize = PiliTheme.typography.sizeSm,
                    fontWeight = PiliTheme.typography.weightBold,
                ),
            color = tint,
            modifier = Modifier.testTag("nightly_state"),
        )
    }
}

@Composable
private fun StatusRows(status: ExportStatus) {
    val never = stringResource(R.string.nightly_never)
    Column {
        status.endpoint?.let { StatusRow(stringResource(R.string.nightly_endpoint), it, "nightly_endpoint") }
        StatusRow(
            label = stringResource(R.string.nightly_last_export),
            value =
                listOfNotNull(status.lastExportId, NightlyTime.formatOrNever(status.lastPreparedAt, never))
                    .joinToString(SEPARATOR),
            tag = "nightly_last_export",
        )
        StatusRow(
            label = stringResource(R.string.nightly_last_upload),
            value = NightlyTime.formatOrNever(status.lastUploadedAt, never),
            tag = "nightly_last_upload",
        )
        status.nextAttemptAt?.takeIf { status.pending > 0 }?.let {
            StatusRow(stringResource(R.string.nightly_next_attempt), NightlyTime.format(it), "nightly_next_attempt")
        }
        status.lastError?.let { error ->
            val shown = listOfNotNull(error, status.lastErrorAt?.let(NightlyTime::format)).joinToString(SEPARATOR)
            StatusRow(stringResource(R.string.nightly_last_error), shown, "nightly_last_error", error = true)
        }
        StatusRow(
            label = stringResource(R.string.nightly_kept, status.kept, status.keepDays),
            value = null,
            tag = "nightly_kept",
        )
        if (status.dropped > 0) StatusRow(stringResource(R.string.nightly_dropped), status.dropped.toString(), "nightly_dropped")
    }
}

@Composable
private fun StatusRow(
    label: String,
    value: String?,
    tag: String,
    error: Boolean = false,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(dimens.spaceMd),
        modifier =
            Modifier
                .fillMaxWidth()
                .defaultMinSize(minHeight = dimens.spaceXl + dimens.spaceSm)
                .testTag(tag),
    ) {
        Text(
            text = label,
            style = PiliTheme.typography.body,
            color = if (error) colors.accentDanger else colors.textSecondary,
        )
        if (value != null) {
            Text(
                text = value,
                style = PiliTheme.typography.figure,
                color = if (error) colors.accentDanger else colors.textPrimary,
                textAlign = TextAlign.End,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

private const val SEPARATOR = " · "
