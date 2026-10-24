package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.BadgeLogEntry

/** The latest badge log rows shown (`slice(0, 6)` in `BadgeSettings.vue`). */
private const val LOG_SHOWN = 6

private val SwitchTrackWidth = 64.dp
private val SwitchTrackHeight = 40.dp
private val SwitchThumb = 32.dp
private val SwitchTravel = 24.dp
private val FieldHeight = 48.dp
private val FieldMinWidth = 96.dp
private val SaveHeight = 56.dp

/** The label of the reader value: "Désactivé", "NFC intégré" or "Lecteur USB". */
@Composable
private fun readerLabel(reader: String): String =
    when (reader) {
        "usb" -> vueText("badge_reader_usb")
        "nfc" -> uiText(R.string.ui_settings_badge_reader_nfc)
        else -> vueText("badge_reader_off")
    }

/**
 * The badge reader panel of a manager (`BadgeSettings.vue`): folded by default behind its header, which shows the
 * reader in use. Unfolded: the reader choice ("Désactivé", "NFC intégré" only when the device has NFC, "Lecteur
 * USB"), the badge login switch, the USB timing with the numeric pad, the privacy note and the latest log rows.
 * Test tags: `settings_badges`, `badge_toggle`, `badge_relink`, `badge_reader_<off|nfc|usb>`, `badge_nfc_note`,
 * `badge_login_switch`, `badge_field_<gap|min|max>`, `badge_timing_save`, `badge_error`, `badge_log`, `badge_log_<n>`.
 */
@Composable
internal fun BadgePanel(
    panel: BadgePanelState,
    callbacks: BadgeCallbacks,
    modifier: Modifier = Modifier,
) {
    val settings = panel.settings
    Panel(modifier = modifier.testTag("settings_badges")) {
        PanelHead(panel, callbacks.onToggle)
        if (settings?.relinkNeeded == true) {
            PanelText(vueText("settings_badge_relink"), Modifier.testTag("badge_relink"), color = PiliTheme.colors.accentWarning)
        }
        if (panel.open && settings != null) {
            ReaderChoice(panel, callbacks)
            if (panel.readerOn) LoginSwitch(settings.loginEnabled, callbacks.onToggleLogin)
            if (panel.reader == "usb") UsbTiming(panel, callbacks)
            panel.error?.let {
                Text(
                    it,
                    style = PiliTheme.typography.body,
                    color = PiliTheme.colors.accentDanger,
                    modifier = Modifier.testTag("badge_error"),
                )
            }
            PanelText(vueText("settings_badge_privacy"))
            BadgeLog(panel)
        }
    }
}

@Composable
private fun PanelHead(
    panel: BadgePanelState,
    onToggle: () -> Unit,
) {
    Row(
        Modifier.fillMaxWidth().clickable(role = Role.Button, onClick = onToggle).testTag("badge_toggle"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        PanelTitle(vueText("settings_badges"), Modifier.weight(1f))
        Text(
            readerLabel(panel.reader) + "  " + if (panel.open) "▴" else "▾",
            style = PiliTheme.typography.figure.copy(fontWeight = PiliTheme.typography.weightBold),
            color = PiliTheme.colors.textSecondary,
            modifier = Modifier.testTag("badge_state"),
        )
    }
}

/** The reader buttons, and the note when the stored choice cannot work on this device. */
@Composable
private fun ReaderChoice(
    panel: BadgePanelState,
    callbacks: BadgeCallbacks,
) {
    val options =
        buildList {
            add(Triple("off", "off", vueText("badge_reader_off")))
            if (panel.hasNfc) add(Triple("nfc", "nfc", uiText(R.string.ui_settings_badge_reader_nfc)))
            add(Triple("usb", "usb", vueText("badge_reader_usb")))
        }
    SegmentedControl(options = options, selected = panel.reader, onSelect = callbacks.onReader, tag = "badge_reader")
    val note =
        when {
            panel.reader == "nfc" && !panel.hasNfc -> vueText("settings_badge_nfc_unavailable")
            panel.reader == "nfc" && !panel.nfcEnabled -> uiText(R.string.ui_settings_badge_nfc_disabled)
            else -> null
        }
    if (note != null) {
        Text(note, style = PiliTheme.typography.body, color = PiliTheme.colors.accentDanger, modifier = Modifier.testTag("badge_nfc_note"))
    }
}

/** "Connexion par badge" and its switch, then the note that managers still type their PIN. */
@Composable
private fun LoginSwitch(
    on: Boolean,
    onToggle: () -> Unit,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Text(
            vueText("settings_badge_login"),
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.weight(1f),
        )
        BadgeSwitch(on, onToggle, Modifier.testTag("badge_login_switch"))
    }
    PanelText(vueText("settings_badge_login_note"))
}

/** The on/off switch of the panel: a track and a thumb, in the primary accent when on (`.toggle` of the Vue panel). */
@Composable
private fun BadgeSwitch(
    on: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Box(
        modifier
            .height(SaveHeight)
            .background(colors.bgSurface, RoundedCornerShape(SaveHeight / 2))
            .clickable(role = Role.Switch, onClick = onToggle)
            .semantics { stateDescription = if (on) "on" else "off" }
            .padding(horizontal = PiliTheme.dimens.spaceSm),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier.size(
                SwitchTrackWidth,
                SwitchTrackHeight,
            ).background(if (on) colors.accentPrimary else colors.border, RoundedCornerShape(SwitchTrackHeight / 2)),
        ) {
            Box(
                Modifier
                    .padding(start = PiliTheme.dimens.spaceXs + if (on) SwitchTravel else 0.dp, top = PiliTheme.dimens.spaceXs)
                    .size(SwitchThumb)
                    .background(if (on) colors.textPrimary else colors.textSecondary, CircleShape),
            )
        }
    }
}

/** The three timing fields, the pad that edits the one tapped, and the save button while something changed. */
@Composable
private fun UsbTiming(
    panel: BadgePanelState,
    callbacks: BadgeCallbacks,
) {
    val draft = panel.draft ?: return
    SectionLabel(vueText("settings_badge_usb_timing"))
    TimingField.entries.forEach { field ->
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(
                vueText(field.labelKey),
                style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
                color = PiliTheme.colors.textPrimary,
                modifier = Modifier.weight(1f),
            )
            val editing = panel.editing == field
            Box(
                Modifier
                    .heightIn(min = FieldHeight)
                    .widthIn(min = FieldMinWidth)
                    .border(
                        1.dp,
                        if (editing) PiliTheme.colors.accentPrimary else PiliTheme.colors.border,
                        RoundedCornerShape(PiliTheme.dimens.radiusMd),
                    )
                    .clickable(role = Role.Button) { callbacks.onEditField(field) }
                    .semantics { selected = editing }
                    .padding(horizontal = PiliTheme.dimens.spaceMd)
                    .testTag("badge_field_" + field.tag),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    draft.of(field).ifEmpty { "·" },
                    style = PiliTheme.typography.figure.copy(fontWeight = PiliTheme.typography.weightBold),
                    color = PiliTheme.colors.textPrimary,
                    textAlign = TextAlign.Center,
                )
            }
        }
    }
    if (panel.editing != null) {
        Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
            NumPad(
                onKey = callbacks.onKey,
                clearLabel = uiText(R.string.ui_common_numpad_clear),
                backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
            )
        }
    }
    if (panel.dirty) {
        ActionButton(
            vueText("settings_tables_save"),
            callbacks.onSaveTiming,
            Modifier.fillMaxWidth().heightIn(min = SaveHeight).testTag("badge_timing_save"),
        )
    }
}

/** The latest rows of the badge log, newest first: what happened, to whom, by whom and when. */
@Composable
private fun BadgeLog(panel: BadgePanelState) {
    SectionLabel(vueText("settings_badge_log"))
    val log = panel.overview?.log.orEmpty()
    if (log.isEmpty()) {
        PanelText(vueText("settings_badge_log_empty"), Modifier.testTag("badge_log_empty"))
        return
    }
    Column(Modifier.fillMaxWidth().testTag("badge_log"), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        log.take(LOG_SHOWN).forEachIndexed { index, entry -> LogRow(index, entry) }
    }
}

@Composable
private fun LogRow(
    index: Int,
    entry: BadgeLogEntry,
) {
    val what = vueText("badge_log_" + entry.action) + (entry.name?.let { " · $it" } ?: "")
    val meta = (entry.byName ?: "") + " " + PosDateTime.formatTimestamp(entry.at, PosDateTime::formatDayTime)
    Column(Modifier.fillMaxWidth().padding(vertical = 8.dp).semantics(mergeDescendants = true) {}.testTag("badge_log_$index")) {
        Text(
            what,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = PiliTheme.colors.textPrimary,
        )
        Text(
            meta.trim(),
            style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeSm),
            color = PiliTheme.colors.textSecondary,
        )
        Hairline()
    }
}

/**
 * The badge state of an employee row and its button (`worker-badge` of `SettingsView.vue`): "Badge lié le 17/10" or
 * "Aucun badge", and "Lier un badge" or "Retirer le badge". Shown by the caller only to a manager with a reader on.
 */
@Composable
internal fun WorkerBadgeLine(
    insz: String,
    linkedAt: String?,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val colors = PiliTheme.colors
    GapRow(modifier.fillMaxWidth()) {
        Text(
            if (linkedAt == null) {
                vueText("badge_none_label")
            } else {
                vueText("badge_linked_on", "date" to PosDateTime.formatTimestamp(linkedAt, PosDateTime::formatDate))
            },
            style = PiliTheme.typography.body,
            color = if (linkedAt == null) colors.textSecondary else colors.textPrimary,
            modifier = Modifier.weight(1f, fill = false).testTag("worker_" + insz + "_badge"),
        )
        ActionButton(
            vueText(if (linkedAt == null) "badge_link" else "badge_revoke"),
            onClick,
            Modifier.testTag("worker_" + insz + "_badge_button"),
        )
    }
}
