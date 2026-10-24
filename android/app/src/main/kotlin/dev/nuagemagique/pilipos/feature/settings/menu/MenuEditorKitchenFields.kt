package dev.nuagemagique.pilipos.feature.settings.menu

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.feature.settings.Hairline
import dev.nuagemagique.pilipos.feature.settings.PanelText
import dev.nuagemagique.pilipos.feature.settings.PanelTitle
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.AllergenEntry

private val AllergenChipHeight = 52.dp
private val NumberSize = 24.dp
private val TracesBorder = 2.dp

/**
 * The kitchen course of a category (`menu-ed-course` of `MenuEditorView.vue`): four chips, "Direct" to "Desserts",
 * written with `set_course_settings` by Enregistrer. Hidden when the setting could not be read; with courses off a
 * note says the choice waits for the switch.
 */
@Composable
internal fun CourseField(
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
) {
    val settings = state.courseSettings ?: return
    FieldLabel(vueText("menu_ed_course"))
    ChipRow {
        COURSES.forEach { n -> Chip(vueText("course_$n"), state.courseDraft == n, "menu_ed_course_$n") { callbacks.onCourse(n) } }
    }
    if (!settings.enabled) PanelText(vueText("menu_ed_course_off"), Modifier.testTag("menu_ed_course_off"))
}

/**
 * The allergen list of a saved product or option (`menu-ed-allergens` of `MenuEditorView.vue`): the "Liste déclarée"
 * switch, then the 14 allergens in two columns; a tap cycles contains, traces, nothing. Written with `set_allergens`
 * by Enregistrer.
 */
@Composable
internal fun AllergenSection(
    state: MenuEditorUiState,
    form: MenuForm,
    callbacks: MenuEditorCallbacks,
) {
    if (form.kind != FormKind.Product && form.kind != FormKind.Option) return
    Hairline()
    Column(Modifier.fillMaxWidth().testTag("menu_ed_allergens"), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        PanelTitle(vueText("menu_ed_allergens"))
        if (form.isNew) {
            PanelText(vueText("menu_ed_allergens_save_first"), Modifier.testTag("menu_ed_allergens_save_first"))
            return@Column
        }
        val draft = state.allergenDraft
        ToggleRow(vueText("menu_ed_allergens_declared"), draft.declared, "menu_ed_allergens_declared", callbacks.onAllergensDeclared)
        if (!draft.declared) {
            PanelText(vueText("menu_ed_allergens_unknown"), Modifier.testTag("menu_ed_allergens_unknown"))
            return@Column
        }
        PanelText(vueText("menu_ed_allergens_hint"))
        state.legend.chunked(2).forEach { pair ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
                pair.forEach { a -> AllergenChip(a, draft.levelOf(a.code), { callbacks.onAllergen(a.code) }, Modifier.weight(1f)) }
                if (pair.size == 1) Box(Modifier.weight(1f))
            }
        }
        if (draft.contains.isEmpty() && draft.mayContain.isEmpty()) {
            PanelText(vueText("allergens_none"), Modifier.testTag("menu_ed_allergens_none"))
        }
    }
}

@Composable
private fun AllergenChip(
    entry: AllergenEntry,
    level: AllergenLevel,
    onClick: () -> Unit,
    modifier: Modifier,
) {
    val colors = PiliTheme.colors
    val shape = RoundedCornerShape(PiliTheme.dimens.radiusMd)
    val contains = level == AllergenLevel.CONTAINS
    val levelText =
        when (level) {
            AllergenLevel.CONTAINS -> vueText("allergens_contains")
            AllergenLevel.TRACES -> vueText("allergens_traces")
            AllergenLevel.NONE -> ""
        }
    Row(
        modifier
            .heightIn(min = AllergenChipHeight)
            .background(if (contains) colors.selectedBg else colors.bgSurface, shape)
            .border(TracesBorder, if (level == AllergenLevel.TRACES) colors.accentWarning else Color.Transparent, shape)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { stateDescription = level.name.lowercase() }
            .padding(horizontal = PiliTheme.dimens.spaceSm)
            .testTag("menu_ed_allergen_${entry.code}"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(NumberSize).background(colors.accentInfo, CircleShape), contentAlignment = Alignment.Center) {
            Text(entry.number.toString(), style = PiliTheme.typography.label, color = colors.textOnAccent)
        }
        val fg = if (contains) colors.selectedFg else colors.textPrimary
        Text(
            vueText("allergen_${entry.code}"),
            style = PiliTheme.typography.label.copy(fontWeight = PiliTheme.typography.weightBold),
            color = fg,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        if (levelText.isNotEmpty()) Text(levelText, style = PiliTheme.typography.label, color = fg)
    }
}
