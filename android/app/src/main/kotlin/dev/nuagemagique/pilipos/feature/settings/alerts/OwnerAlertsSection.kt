@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.settings.alerts

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.feature.settings.ActionButton
import dev.nuagemagique.pilipos.feature.settings.Hairline
import dev.nuagemagique.pilipos.feature.settings.Panel
import dev.nuagemagique.pilipos.feature.settings.PanelText
import dev.nuagemagique.pilipos.feature.settings.PanelTitle
import dev.nuagemagique.pilipos.feature.settings.SectionLabel
import dev.nuagemagique.pilipos.feature.settings.SegmentedControl
import dev.nuagemagique.pilipos.feature.settings.SettingsTextField
import dev.nuagemagique.pilipos.feature.settings.StatusPill
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.AlertLogRow
import uniffi.pos_ffi.OwnerAlertsInput

/** Height of a switch row (56 in `OwnerAlertsSettings.vue`). */
private val SwitchHeight = 56.dp

/** Width and height of a switch track and the side of its thumb. */
private val TrackWidth = 64.dp
private val TrackHeight = 40.dp
private val ThumbSize = 32.dp
private val ThumbTravel = 24.dp

/** Height of the value buttons and of a log row. */
private val ValueHeight = 48.dp
private val LogRowHeight = 44.dp

/** Everything the owner can do in the alerts panel. */
internal data class OwnerAlertsCallbacks(
    val onToggleEnabled: () -> Unit = {},
    val onToggleKind: (String) -> Unit = {},
    val onLanguage: (String) -> Unit = {},
    val onLabel: (String) -> Unit = {},
    val onEdit: (AlertNumber) -> Unit = {},
    val onSave: () -> Unit = {},
    val onTest: () -> Unit = {},
)

/**
 * The owner alerts panel of the settings screen (`OwnerAlertsSettings.vue`): Telegram messages to the owner's
 * phone. A draft the manager edits and saves; the bot token is a file named by `pos.json`, so the panel only says
 * whether it is there. Test tags: `alerts_section`, `alerts_enabled`, `alerts_token`, `alerts_chat_id`,
 * `alerts_label`, `alerts_lang_<fr|zh|en>`, `alerts_kind_<kind>`, `alerts_z_hour`, `alerts_online_wait`,
 * `alerts_reminder`, `alerts_save`, `alerts_test`, `alerts_saved`, `alerts_test_sent`, `alerts_error`,
 * `alerts_log`, `alerts_log_empty`, `alerts_log_row_<id>`.
 */
@Composable
internal fun OwnerAlertsSection(
    state: OwnerAlertsUiState,
    callbacks: OwnerAlertsCallbacks,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Panel(modifier = modifier.testTag("alerts_section")) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            PanelTitle(vueText("alerts_title"), Modifier.weight(1f))
            val draft = state.draft
            if (draft != null) {
                SwitchRow(
                    on = draft.enabled,
                    text = vueText(if (draft.enabled) "alerts_state_on" else "alerts_state_off"),
                    onClick = callbacks.onToggleEnabled,
                    tag = "alerts_enabled",
                    fill = false,
                )
            }
        }
        if (state.error != null) {
            Text(state.error, style = PiliTheme.typography.body, color = colors.accentDanger, modifier = Modifier.testTag("alerts_error"))
        }
        val settings = state.settings
        val draft = state.draft
        if (settings != null && draft != null) {
            Body(state, draft, settings.tokenConfigured, callbacks)
        }
    }
}

@Composable
private fun Body(
    state: OwnerAlertsUiState,
    draft: OwnerAlertsInput,
    tokenConfigured: Boolean,
    callbacks: OwnerAlertsCallbacks,
) {
    TokenAndChat(draft, tokenConfigured, callbacks)
    LabelAndLanguage(draft, callbacks)
    KindSwitches(draft, callbacks)
    Thresholds(draft, callbacks)
    SaveAndTest(state, callbacks)
    PanelText(vueText("alerts_note"))
    LogBlock(state.log)
}

@Composable
private fun TokenAndChat(
    draft: OwnerAlertsInput,
    tokenConfigured: Boolean,
    callbacks: OwnerAlertsCallbacks,
) {
    val colors = PiliTheme.colors
    StatusPill(
        vueText(if (tokenConfigured) "alerts_token_ok" else "alerts_token_missing_pill"),
        if (tokenConfigured) colors.accentSuccess else colors.accentWarning,
        Modifier.testTag("alerts_token"),
    )
    if (!tokenConfigured) PanelText(vueText("alerts_token_missing_note"))
    ValueRow(vueText("alerts_chat_id"), draft.chatId.ifEmpty { vueText("alerts_chat_id_unset") }, "alerts_chat_id") {
        callbacks.onEdit(AlertNumber.ChatId)
    }
}

@Composable
private fun LabelAndLanguage(
    draft: OwnerAlertsInput,
    callbacks: OwnerAlertsCallbacks,
) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(vueText("alerts_label"), style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
        SettingsTextField(
            value = draft.label,
            onValueChange = callbacks.onLabel,
            placeholder = vueText("alerts_label_placeholder"),
            modifier = Modifier.weight(1f).testTag("alerts_label"),
        )
    }
    SectionLabel(vueText("alerts_language"))
    SegmentedControl(
        options = ALERT_LANGUAGES.map { Triple(it, it, vueText("settings_lang_name_$it")) },
        selected = draft.language,
        onSelect = callbacks.onLanguage,
        tag = "alerts_lang",
    )
}

@Composable
private fun KindSwitches(
    draft: OwnerAlertsInput,
    callbacks: OwnerAlertsCallbacks,
) {
    SectionLabel(vueText("alerts_kinds"))
    ALERT_KINDS.forEach { kind ->
        SwitchRow(
            on = draft.kinds.isOn(kind),
            text = vueText("alerts_kind_$kind"),
            onClick = { callbacks.onToggleKind(kind) },
            tag = "alerts_kind_$kind",
            fill = true,
        )
    }
}

@Composable
private fun Thresholds(
    draft: OwnerAlertsInput,
    callbacks: OwnerAlertsCallbacks,
) {
    ValueRow(vueText("alerts_z_deadline"), vueText("alerts_value_hour", "n" to draft.zDeadlineHour), "alerts_z_hour") {
        callbacks.onEdit(AlertNumber.ZDeadlineHour)
    }
    ValueRow(vueText("alerts_online_wait"), vueText("alerts_value_minutes", "n" to draft.onlineWaitMinutes), "alerts_online_wait") {
        callbacks.onEdit(AlertNumber.OnlineWait)
    }
    val reminder =
        if (draft.reminderMinutes == 0L) vueText("alerts_value_never") else vueText("alerts_value_minutes", "n" to draft.reminderMinutes)
    ValueRow(vueText("alerts_reminder"), reminder, "alerts_reminder") { callbacks.onEdit(AlertNumber.Reminder) }
}

@Composable
private fun SaveAndTest(
    state: OwnerAlertsUiState,
    callbacks: OwnerAlertsCallbacks,
) {
    val colors = PiliTheme.colors
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        ActionButton(
            vueText("alerts_save"),
            callbacks.onSave,
            Modifier.weight(1f).heightIn(min = SwitchHeight).testTag("alerts_save"),
            enabled = !state.busy && state.dirty,
            variant = PiliButtonVariant.Primary,
        )
        ActionButton(
            vueText("alerts_test"),
            callbacks.onTest,
            Modifier.weight(1f).heightIn(min = SwitchHeight).testTag("alerts_test"),
            enabled = state.canTest,
        )
    }
    if (state.saved && !state.dirty) {
        Text(
            vueText("alerts_saved"),
            style = PiliTheme.typography.body,
            color = colors.accentSuccess,
            modifier = Modifier.testTag("alerts_saved"),
        )
    }
    if (state.testSent) {
        Text(
            vueText("alerts_test_sent"),
            style = PiliTheme.typography.body,
            color = colors.accentSuccess,
            modifier = Modifier.testTag("alerts_test_sent"),
        )
    }
}

@Composable
private fun LogBlock(log: List<AlertLogRow>) {
    SectionLabel(vueText("alerts_log_title"))
    if (log.isEmpty()) {
        PanelText(vueText("alerts_log_empty"), Modifier.testTag("alerts_log_empty"))
    } else {
        Column(Modifier.fillMaxWidth().testTag("alerts_log")) {
            log.forEach { row -> LogRow(row) }
        }
    }
}

/** A label on the left and, on the right, a button showing the value that opens its dialog (`.row` and `.value-btn`). */
@Composable
private fun ValueRow(
    label: String,
    value: String,
    tag: String,
    onClick: () -> Unit,
) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = SwitchHeight),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary, modifier = Modifier.weight(1f))
        Box(
            Modifier
                .heightIn(min = ValueHeight)
                .background(PiliTheme.colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = PiliTheme.dimens.spaceMd)
                .testTag(tag),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                value,
                style = PiliTheme.typography.figure,
                color = PiliTheme.colors.textPrimary,
            )
        }
    }
    Hairline()
}

/** An on/off switch (`.toggle`, `.kind`): the text and a track with a thumb; [fill] spreads the text and the track over the row. */
@Composable
private fun SwitchRow(
    on: Boolean,
    text: String,
    onClick: () -> Unit,
    tag: String,
    fill: Boolean,
) {
    val colors = PiliTheme.colors
    Row(
        modifier =
            (if (fill) Modifier.fillMaxWidth() else Modifier)
                .heightIn(min = SwitchHeight)
                .background(colors.bgSurface, RoundedCornerShape(SwitchHeight / 2))
                .clickable(role = Role.Switch, onClick = onClick)
                .semantics { stateDescription = if (on) "on" else "off" }
                .padding(start = PiliTheme.dimens.spaceMd, end = PiliTheme.dimens.spaceSm)
                .testTag(tag),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd, if (fill) Alignment.Start else Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = if (on) colors.textPrimary else colors.textSecondary,
            modifier = if (fill) Modifier.weight(1f) else Modifier,
        )
        Box(
            Modifier.size(
                TrackWidth,
                TrackHeight,
            ).background(if (on) colors.accentSuccess else colors.border, RoundedCornerShape(TrackHeight / 2)),
        ) {
            Box(
                Modifier
                    .padding(start = PiliTheme.dimens.spaceXs + if (on) ThumbTravel else 0.dp, top = PiliTheme.dimens.spaceXs)
                    .size(ThumbSize)
                    .background(if (on) colors.textPrimary else colors.textSecondary, CircleShape),
            )
        }
    }
}

/** One message of the log: what it was and what became of it (`.log-row`). */
@Composable
private fun LogRow(row: AlertLogRow) {
    val colors = PiliTheme.colors
    val what =
        vueText("alerts_transition_${row.transition}") + if (row.transition != "test") " · " + vueText("alerts_kind_${row.kind}") else ""
    val time = PosDateTime.formatTimestamp(row.createdAt, PosDateTime::formatDayTime)
    Column(Modifier.fillMaxWidth().semantics(mergeDescendants = true) {}.testTag("alerts_log_row_${row.id}")) {
        Row(
            Modifier.fillMaxWidth().heightIn(min = LogRowHeight),
            horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(what, style = PiliTheme.typography.body, color = colors.textPrimary, modifier = Modifier.weight(1f))
            Text(
                vueText("alerts_outcome_${row.outcome}") + " · " + time,
                style = PiliTheme.typography.figure,
                color = if (row.outcome == "sent") colors.accentSuccess else colors.accentWarning,
            )
        }
        Hairline()
    }
}
