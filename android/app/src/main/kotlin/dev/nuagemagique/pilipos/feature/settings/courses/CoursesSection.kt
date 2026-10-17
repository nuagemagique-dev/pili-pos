package dev.nuagemagique.pilipos.feature.settings.courses

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import dev.nuagemagique.pilipos.feature.settings.ActionButton
import dev.nuagemagique.pilipos.feature.settings.Panel
import dev.nuagemagique.pilipos.feature.settings.PanelText
import dev.nuagemagique.pilipos.feature.settings.PanelTitle
import dev.nuagemagique.pilipos.feature.settings.SectionLabel
import dev.nuagemagique.pilipos.feature.settings.SegmentedControl
import dev.nuagemagique.pilipos.feature.settings.alerts.SwitchRow
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * The courses panel of the settings screen (`CoursesSettings.vue`, "Service en plusieurs temps"): a switch and, per
 * category, the course its products take (0 "Direct" to 3 "Desserts"). Kitchen printing only: nothing here reaches
 * the signed data. Test tags: `courses_section`, `courses_enabled`, `courses_cat_<id>` (a row) and
 * `courses_cat_<id>_<course>` (its buttons), `courses_save`, `courses_saved`, `courses_error`.
 */
@Composable
internal fun CoursesSection(
    state: CoursesUiState,
    callbacks: CoursesCallbacks,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Panel(modifier = modifier.testTag("courses_section")) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            PanelTitle(vueText("courses_title"), Modifier.weight(1f))
            if (state.stored != null) {
                SwitchRow(
                    on = state.enabled,
                    text = vueText(if (state.enabled) "courses_state_on" else "courses_state_off"),
                    onClick = callbacks.onToggle,
                    tag = "courses_enabled",
                    fill = false,
                )
            }
        }
        state.error?.let {
            Text(it, style = PiliTheme.typography.body, color = colors.accentDanger, modifier = Modifier.testTag("courses_error"))
        }
        PanelText(vueText("courses_note"))
        if (state.stored != null) {
            SectionLabel(vueText("courses_categories"))
            if (state.categories.isEmpty()) PanelText(vueText("courses_no_categories"))
            state.categories.forEach { category ->
                CategoryRow(category.id, category.name, state.mapping[category.id] ?: 0, callbacks)
            }
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                ActionButton(
                    text = vueText("courses_save"),
                    onClick = callbacks.onSave,
                    enabled = !state.busy && state.dirty,
                    variant = PiliButtonVariant.Primary,
                    modifier = Modifier.testTag("courses_save"),
                )
                if (state.saved && !state.dirty) {
                    Text(
                        text = vueText("courses_saved"),
                        style = PiliTheme.typography.body,
                        color = colors.accentSuccess,
                        modifier = Modifier.testTag("courses_saved"),
                    )
                }
            }
        }
    }
}

/** A category and the four buttons of its course. */
@Composable
private fun CategoryRow(
    id: Long,
    name: String,
    course: Int,
    callbacks: CoursesCallbacks,
) {
    Row(
        Modifier.fillMaxWidth().testTag("courses_cat_$id"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(name, style = PiliTheme.typography.body, color = PiliTheme.colors.textPrimary, modifier = Modifier.weight(1f))
        SegmentedControl(
            options = COURSE_CHOICES.map { Triple(it, it.toString(), vueText("course_$it")) },
            selected = course,
            onSelect = { callbacks.onCourse(id, it) },
            tag = "courses_cat_$id",
            modifier = Modifier.weight(2f),
        )
    }
}
