@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The selection of a cart line has not been chosen yet: the newest line is open. */
private const val NO_CHOICE: Long = Long.MIN_VALUE

private val PendingDot = 10.dp
private val ActionHeight = 64.dp
private val EmptyHeight = 120.dp

/**
 * The lines of the cart column. A dine-in line not yet sent goes last under an amber label; a takeaway's lines all go
 * out with the sale. With courses on the lines are grouped by course, lowest first, under a header (a takeaway groups
 * too) and the amber label gives way to the groups. Test tags: `cart_pending_label`, `cart_course_<n>`, `cart_empty`.
 */
@Composable
internal fun CartLines(
    state: OrderCartState,
    actions: OrderCartActions,
    dineIn: Boolean,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    // Dine-in lines not yet sent go last under an amber label; a takeaway's lines all go out with the sale.
    val sent = if (dineIn) state.rows.filter { it.line.unsentQuantity <= 0 } else state.rows
    val pending = if (dineIn) state.rows.filter { it.line.unsentQuantity > 0 } else emptyList()
    val selection = rememberCartSelection(state)
    val item: @Composable (CartRow, Boolean) -> Unit = { row, isPending ->
        OrderLineItem(
            row,
            state,
            actions,
            pending = isPending,
            selected = selection.selectedId == row.line.id,
            onSelect = { selection.toggle(row.line.id) },
        )
    }
    Column(
        modifier = modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = d.spaceMd),
    ) {
        if (state.coursesEnabled) {
            state.rows.groupBy { it.line.course }.toSortedMap().forEach { (course, rows) ->
                CourseHeader(course)
                rows.forEach { item(it, dineIn && it.line.unsentQuantity > 0) }
            }
        } else {
            sent.forEach { item(it, false) }
            if (pending.isNotEmpty()) {
                PendingLabel()
                pending.forEach { item(it, true) }
            }
        }
        if (state.rows.isEmpty()) {
            Box(Modifier.fillMaxWidth().height(EmptyHeight), contentAlignment = Alignment.Center) {
                Text(text = vueText("no_items"), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("cart_empty"))
            }
        }
    }
}

/** The selected line of the cart and what a tap on a line does to it. */
private class CartSelection(
    val selectedId: Long?,
    val toggle: (Long) -> Unit,
)

/**
 * C2: one line is selected, its actions show under it. By default it is the newest line of the cart; a tap on the
 * selected line folds it, a tap on another one moves the selection, and an add selects the line it went to
 * (`selectedLineId` of `CartSidebar.vue`). [NO_CHOICE] is "no choice yet": the newest line.
 */
@Composable
private fun rememberCartSelection(state: OrderCartState): CartSelection {
    var choice by rememberSaveable { mutableStateOf<Long?>(NO_CHOICE) }
    val newest = state.rows.lastOrNull { it.line.bowlLineId == null }?.line?.id
    val selectedId = if (choice == NO_CHOICE) newest else choice
    val lastAdd = state.lastAdd
    LaunchedEffect(lastAdd?.seq) { if (lastAdd != null) choice = lastAdd.lineId }
    // Another order, or a line that left the cart (removed, paid): back to the newest line.
    LaunchedEffect(state.order?.id) { choice = NO_CHOICE }
    LaunchedEffect(state.rows) {
        val picked = choice
        if (picked != null && picked != NO_CHOICE && state.rows.none { it.line.id == picked }) choice = NO_CHOICE
    }
    return CartSelection(selectedId) { id -> choice = if (selectedId == id) null else id }
}

/** The amber label over the lines that were not sent to the bar or kitchen yet. */
@Composable
private fun PendingLabel() {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = PiliTheme.dimens.spaceMd, bottom = PiliTheme.dimens.spaceXs)
                .semantics(mergeDescendants = true) {}
                .testTag("cart_pending_label"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(PendingDot).background(c.accentWarning, CircleShape))
        Text(
            text = vueText("not_sent_to_kitchen").uppercase(),
            style = t.figure.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
            color = c.accentWarning,
        )
    }
}

/** The header of a course group of the cart (`course-label` of `CartSidebar.vue`). */
@Composable
private fun CourseHeader(course: Int) {
    val t = PiliTheme.typography
    Text(
        text = vueText("course_$course").uppercase(),
        style = t.figure.copy(fontSize = t.sizeMd, fontWeight = t.weightBold),
        color = PiliTheme.colors.textSecondary,
        modifier = Modifier.padding(top = PiliTheme.dimens.spaceMd, bottom = PiliTheme.dimens.spaceXs).testTag("cart_course_$course"),
    )
}

/** "Envoyer la suite (Plats)": signs what is unsent, releases the next held course and prints it. */
@Composable
internal fun NextCourseButton(
    course: Int,
    sending: Boolean,
    actions: OrderCartActions,
) {
    val t = PiliTheme.typography
    PiliButton(
        onClick = actions.onSendNext,
        variant = PiliButtonVariant.Secondary,
        enabled = !sending,
        modifier = Modifier.fillMaxWidth().height(ActionHeight).testTag("cart_send_next"),
    ) {
        Text(
            vueText("send_next_course", "course" to vueText("course_$course")),
            style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
        )
    }
}
