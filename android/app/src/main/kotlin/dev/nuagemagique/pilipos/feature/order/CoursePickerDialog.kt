package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val ChoiceHeight = 72.dp
private const val MAX_COURSE = 3
private const val PER_ROW = 2

/** The four courses, 0 "Direct" to 3 "Desserts". */
private val Courses: List<Int> = (0..MAX_COURSE).toList()

/**
 * The four courses of a line (0 "Direct" to 3 "Desserts") as big buttons (`CoursePickerDialog.vue`). The current
 * one is the primary button, the others are secondary; a tap on any calls [onPick] and the screen closes the
 * dialog. The engine refuses a change once a kitchen slip carried the line (the screen shows the refusal).
 *
 * Test tags: `course_picker` (root), `course_pick_<n>`, `course_picker_cancel`.
 */
@Composable
internal fun CoursePickerDialog(
    current: Int,
    onPick: (Int) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    PiliDialog(
        title = vueText("course_picker_title"),
        onDismissRequest = onCancel,
        modifier = modifier.testTag("course_picker"),
        actions = {
            PiliButton(
                text = vueText("course_cancel"),
                onClick = onCancel,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("course_picker_cancel"),
            )
        },
    ) {
        Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            Courses.chunked(PER_ROW).forEach { row ->
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
                    row.forEach { course ->
                        PiliButton(
                            onClick = { onPick(course) },
                            variant = if (course == current) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                            modifier = Modifier.weight(1f).height(ChoiceHeight).testTag("course_pick_$course"),
                        ) {
                            val t = PiliTheme.typography
                            Text(vueText("course_$course"), style = t.body.copy(fontWeight = t.weightBold))
                        }
                    }
                }
            }
        }
    }
}
