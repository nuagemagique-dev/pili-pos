package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The smallest first takeaway number (`TAKEAWAY_FIRST_MIN` of `utils/takeaway.ts`). */
internal const val TAKEAWAY_FIRST_MIN: Int = 1

/** The largest first takeaway number (`TAKEAWAY_FIRST_MAX`); the engine checks it again. */
internal const val TAKEAWAY_FIRST_MAX: Int = 999

/** The digits the pad takes for it. */
internal const val TAKEAWAY_FIRST_DIGITS: Int = 3

private val DialogMaxWidth = 420.dp

/** What the first takeaway number dialog calls. */
internal data class TakeawayFirstActions(
    val onEdit: () -> Unit = {},
    val onKey: (NumPadKey) -> Unit = {},
    val onConfirm: () -> Unit = {},
    val onCancel: () -> Unit = {},
)

/** The typed digits are a number the engine takes. */
internal fun takeawayFirstValid(typed: String?): Boolean =
    typed?.toIntOrNull()?.let { it in TAKEAWAY_FIRST_MIN..TAKEAWAY_FIRST_MAX } == true

/**
 * The "Premier numero a emporter" row of the table plan panel (`.takeaway-first` of `SettingsView.vue`): the number
 * the first takeaway of a day gets ("E-61") and the button that edits it. Test tags: `settings_takeaway_first`,
 * `settings_takeaway_first_value`, `settings_takeaway_first_edit`.
 */
@Composable
internal fun TakeawayFirstRow(
    first: Int?,
    onEdit: () -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(
        Modifier.fillMaxWidth().testTag("settings_takeaway_first"),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                vueText("settings_takeaway_first"),
                style = t.body.copy(fontWeight = t.weightBold),
                color = c.textPrimary,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = if (first == null) "" else "E-$first",
                style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
                color = c.textPrimary,
                modifier = Modifier.testTag("settings_takeaway_first_value"),
            )
            ActionButton(
                vueText("settings_takeaway_first_edit"),
                onEdit,
                Modifier.testTag("settings_takeaway_first_edit"),
                enabled = first != null,
            )
        }
        Text(vueText("settings_takeaway_first_note"), style = t.body.copy(fontSize = t.sizeSm), color = c.textSecondary)
    }
}

/**
 * The number pad of the first takeaway number (`NumberEntryDialog.vue` with min 1 and max 999). Test tags:
 * `takeaway_first_dialog`, `takeaway_first_value`, `takeaway_first_ok`, `takeaway_first_cancel`, plus the pad keys.
 */
@Composable
internal fun TakeawayFirstDialog(
    typed: String,
    actions: TakeawayFirstActions,
    modifier: Modifier = Modifier,
) {
    Box(modifier.fillMaxSize().testTag("takeaway_first_dialog")) {
        PiliDialog(
            title = vueText("settings_takeaway_first"),
            onDismissRequest = actions.onCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = actions.onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("takeaway_first_cancel"),
                )
                PiliButton(
                    text = vueText("validate"),
                    onClick = actions.onConfirm,
                    enabled = takeawayFirstValid(typed),
                    modifier = Modifier.testTag("takeaway_first_ok"),
                )
            },
        ) {
            Column(
                Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
            ) {
                Text(
                    text = typed.ifEmpty { "0" },
                    style = PiliTheme.typography.figure.copy(fontSize = PiliTheme.typography.sizeXl),
                    color = PiliTheme.colors.accentPrimary,
                    modifier = Modifier.testTag("takeaway_first_value"),
                )
                NumPad(
                    onKey = actions.onKey,
                    clearLabel = uiText(R.string.ui_common_numpad_clear),
                    backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
                )
            }
        }
    }
}
