package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.ZReportSummary

private val ReportCardHeight = 160.dp
private val ZButtonHeight = 96.dp
private val ZButtonWidth = 520.dp

/**
 * The day tab (`day` section of `ReportsView.vue`): the reminder that a Z is due, the X, user X and Z history
 * cards, the answer of the last report, and the Z bar. The Z button needs two taps: the first arms it (the label
 * becomes the confirmation) and the second one, within five seconds, generates the report.
 * Test tags: `reports_z_notice`, `report_x`, `report_user_x`, `report_z_history`, `report_result`,
 * `report_result_failure`, `report_z_explain`, `report_z`, `report_spinner`.
 */
@Composable
internal fun DayTab(
    day: DayState,
    callbacks: DayCallbacks,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    Column(modifier.fillMaxSize().verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(d.spaceLg)) {
        if (day.zPending) ZNotice()
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceLg)) {
            ReportCard(
                title = vueText("report_x"),
                subtitle = vueText("report_x_desc"),
                loading = day.loadingKind == ReportKind.X,
                enabled = !day.busy,
                onClick = callbacks.onX,
                modifier = Modifier.weight(1f).testTag("report_x"),
            )
            ReportCard(
                title = vueText("report_user_x"),
                subtitle = vueText("report_user_x_desc"),
                loading = day.loadingKind == ReportKind.USER_X,
                enabled = !day.busy,
                onClick = callbacks.onUserX,
                modifier = Modifier.weight(1f).testTag("report_user_x"),
            )
            ReportCard(
                title = vueText("reports_reprint_z"),
                subtitle = lastZText(day.lastZ),
                loading = false,
                enabled = true,
                onClick = callbacks.onOpenZList,
                modifier = Modifier.weight(1f).testTag("report_z_history"),
            )
        }
        day.lastReport?.let { ReportResultPanel(it) }
        ZBar(day, callbacks.onZ)
    }
}

/** The line under the reprint card: the newest Z of the history, or the empty history text. */
@Composable
private fun lastZText(last: ZReportSummary?): String =
    if (last != null) {
        vueText("reports_last_z", "no" to last.sequenceNumber, "date" to dateOf(last.createdAt))
    } else {
        vueText("report_no_history")
    }

/** A backend timestamp as a Belgian date, or `-` when there is none (`formatDate` of `ReportsView.vue`). */
internal fun dateOf(iso: String): String = if (iso.isEmpty()) "-" else PosDateTime.formatTimestamp(iso, PosDateTime::formatDate)

@Composable
private fun ZNotice() {
    val c = PiliTheme.colors
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusMd)
    Text(
        text = vueText("reports_z_pending"),
        style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
        color = c.accentWarning,
        modifier =
            Modifier
                .fillMaxWidth()
                .background(c.bgSecondary, shape)
                .border(1.dp, c.accentWarning, shape)
                .padding(PiliTheme.dimens.spaceMd)
                .testTag("reports_z_notice"),
    )
}

@Composable
private fun ReportCard(
    title: String,
    subtitle: String,
    loading: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    PiliButton(
        onClick = onClick,
        modifier = modifier.height(ReportCardHeight),
        variant = PiliButtonVariant.Secondary,
        enabled = enabled,
    ) {
        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm, Alignment.CenterVertically),
        ) {
            if (loading) Spinner(Modifier.testTag("report_spinner"))
            Text(title, style = t.title.copy(fontSize = t.sizeLg), color = c.textPrimary, textAlign = TextAlign.Center)
            Text(subtitle, style = t.body, color = c.textSecondary, textAlign = TextAlign.Center)
        }
    }
}

/** What the last report answered: the generated report, or the text of the refusal. */
@Composable
private fun ReportResultPanel(outcome: ReportOutcome) {
    val c = PiliTheme.colors
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusMd)
    val tone = if (outcome.generated) c.accentSuccess else c.accentDanger
    val text = outcome.failure ?: (vueText("report_generated") + " (" + reportLabel(outcome.kind, outcome.result?.reportNo) + ")")
    Text(
        text = text,
        style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
        color = tone,
        modifier =
            Modifier
                .fillMaxWidth()
                .background(c.bgSecondary, shape)
                .border(1.dp, tone, shape)
                .padding(PiliTheme.dimens.spaceMd)
                .testTag(if (outcome.generated) "report_result" else "report_result_failure"),
    )
}

@Composable
private fun ZBar(
    day: DayState,
    onZ: () -> Unit,
) {
    val d = PiliTheme.dimens
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceLg), verticalAlignment = Alignment.CenterVertically) {
        Text(
            text = vueText("reports_z_explain"),
            style = PiliTheme.typography.body,
            color = PiliTheme.colors.textSecondary,
            modifier = Modifier.weight(1f).testTag("report_z_explain"),
        )
        PiliButton(
            onClick = onZ,
            modifier = Modifier.width(ZButtonWidth).height(ZButtonHeight).testTag("report_z"),
            variant = if (day.zArmed) PiliButtonVariant.Danger else PiliButtonVariant.Secondary,
            enabled = !day.busy,
        ) {
            if (day.loadingKind == ReportKind.Z) {
                Spinner()
            } else {
                Text(
                    text = if (day.zArmed) vueText("confirm_z") else vueText("reports_z_close"),
                    style = PiliTheme.typography.title.copy(fontSize = PiliTheme.typography.sizeLg),
                    textAlign = TextAlign.Center,
                )
            }
        }
    }
}
