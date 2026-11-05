@file:Suppress("TooManyFunctions") // One cart line and its parts (row, title, chips, detail, actions): one file, like the Vue component.

package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Remove
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.nuagemagique.pilipos.ui.components.GIFT_VOUCHER_DEPARTMENT
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.bilingualName
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.format.lineCategory
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import org.json.JSONTokener
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.OrderType

private val ControlSize = 48.dp
private val QuantityWidth = 28.dp
private val LineMinHeight = 56.dp
private val PendingMark = 3.dp
private val PendingInset = 12.dp
private val Hairline = 1.dp
private val StateMarkWidth = 18.dp
private val LeadWidth = 52.dp
private val ChipRadius = 12.dp
private val DetailInset = 32.dp

/**
 * One cart line (`OrderLineItem.vue`), dense (UX audit C2): a 56 dp row with the kitchen mark (a check when the bar
 * and kitchen have the line, an amber dot while they do not), the quantity "2×" (or the voucher badge, the weight, the
 * "+" of a bowl's option), the menu code (small, secondary; bold when products are shown by code), the name, the chips
 * (options, kitchen note, course, first price change) and the amount. The actions show only under the [selected] line:
 * the stepper (the minus stops at 1), "Note" (filled when the line has one), the manager's "Offrir" and
 * "Retirer la remise", the course picker with courses on, and "Supprimer". A tap on the row selects the line, a second
 * tap folds it ([onSelect]); a bowl's option has no controls of its own and is not selectable. [pending] marks a
 * dine-in line not sent to the bar or kitchen yet (amber mark on the left). The amount is the engine's (PD208): the
 * line total from `CartAmounts.lineAmount`, or the quantity and the unit price as text for a line the VAT split leaves
 * out (code X); nothing is computed here.
 *
 * The detail of the selected line adds what the row truncates: the category (`_sub`), the price per kilo
 * (`_per_kilo`), the options (`_notes`), the whole kitchen note (`_kitchen_note_full`) and each public price change
 * (`_change_<n>`). A gift voucher line shows a badge instead of the stepper (`_voucher`) and has no note, no offer and
 * no course; a weighing shows its weight (`_weight`) and cannot change quantity: to change it the line is removed and
 * the product weighed again. Test tags: `cart_line_<id>`, `_row`, `_state`, `_lead`, `_code`, `_name`,
 * `_kitchen_note`, `_course_tag`, `_change_chip`, `_amount`, `_flash` (present for 200 ms after an add reached the
 * line) and, in the detail, `_detail`, `_minus`, `_qty`, `_plus`, `_note`, `_offer`, `_remove_discount`, `_course`,
 * `_remove`.
 */
@Suppress("LongMethod") // One row of the cart and its detail: lead, names, chips, amount and the actions.
@Composable
internal fun OrderLineItem(
    row: CartRow,
    state: OrderCartState,
    actions: OrderCartActions,
    pending: Boolean,
    modifier: Modifier = Modifier,
    selected: Boolean = false,
    onSelect: () -> Unit = {},
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val id = row.line.id
    val bowlOption = row.line.bowlLineId != null
    // The 200 ms green flash of an add (C9) on the line it went to: one pulse for each add that reaches the line.
    val flash = rememberLineFlash(state.lastAdd?.takeIf { it.lineId == id }?.seq ?: 0L)
    val flashAlpha = flash.alpha
    val flashing = flash.active
    val open = selected && !bowlOption
    // The selected line opens under its row: keep it in view in the scrolling cart (`scrollIntoView` of the Vue line).
    val inView = remember { BringIntoViewRequester() }
    LaunchedEffect(open) { if (open) inView.bringIntoView() }
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .bringIntoViewRequester(inView)
                .drawBehind {
                    if (open) drawRect(c.bgSurface)
                    drawRect(c.border, topLeft = Offset(0f, size.height - Hairline.toPx()), size = Size(size.width, Hairline.toPx()))
                    if (pending) drawRect(c.accentWarning, size = Size(PendingMark.toPx(), size.height))
                    drawFlash(c.accentSuccess, flashAlpha.value)
                }.padding(start = if (pending) PendingInset else 0.dp)
                .testTag("cart_line_$id"),
    ) {
        FlashMarker(id, flashing)
        LineRow(row, state, pending, open, onSelect)
        if (open) LineDetail(row, state, actions)
    }
}

/** The 56 dp row: kitchen mark, lead, code, name, chips and amount. Tapping it selects (or folds) the line. */
@Composable
private fun LineRow(
    row: CartRow,
    state: OrderCartState,
    pending: Boolean,
    open: Boolean,
    onSelect: () -> Unit,
) {
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val c = PiliTheme.colors
    val line = row.line
    val id = line.id
    val bowlOption = line.bowlLineId != null
    val selectLabel = vueText("line_select")
    val rowModifier =
        Modifier
            .fillMaxWidth()
            .defaultMinSize(minHeight = if (bowlOption) BowlRowHeight else LineMinHeight)
            // Not `clickable`: it merges the semantics of the row into one node, and the tags of the parts (quantity,
            // name, amount) would be lost to the tests and to the accessibility tree.
            .let { if (bowlOption) it else it.pointerInput(onSelect) { detectTapGestures(onTap = { onSelect() }) } }
            .semantics {
                if (!bowlOption) {
                    role = Role.Button
                    stateDescription = if (open) "open" else "closed"
                    onClick(label = selectLabel) {
                        onSelect()
                        true
                    }
                }
            }
            .padding(horizontal = d.spaceXs)
            .testTag("cart_line_${id}_row")
    Row(
        modifier = rowModifier,
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        KitchenMark(id, pending, sent = state.order?.orderType == OrderType.DINE_IN && !pending)
        RowLead(row)
        RowTitle(row, state, open, Modifier.weight(1f))
        Text(
            text = amountText(row.amount),
            style = t.figure.copy(fontSize = 17.sp, fontWeight = t.weightBold),
            color = c.textPrimary,
            maxLines = 1,
            textAlign = TextAlign.End,
            modifier = Modifier.defaultMinSize(minWidth = AmountWidth).testTag("cart_line_${id}_amount"),
        )
    }
}

/** The code, the name and the chips of the row: they share the width left by the lead and the amount. */
@Composable
private fun RowTitle(
    row: CartRow,
    state: OrderCartState,
    open: Boolean,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val line = row.line
    val id = line.id
    val voucher = line.departmentId == GIFT_VOUCHER_DEPARTMENT
    val bowlOption = line.bowlLineId != null
    val names = bilingualName(line, state.language, state.labelMode)
    val code = line.code?.trim().orEmpty()
    val name = if (names.isCode) names.secondary.orEmpty() else names.primary
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (code.isNotEmpty()) {
            Text(
                text = code,
                style =
                    if (names.isCode) {
                        t.figure.copy(
                            fontSize = t.sizeLg,
                            fontWeight = t.weightBold,
                        )
                    } else {
                        t.figure.copy(fontSize = 13.sp)
                    },
                color = if (names.isCode) c.textPrimary else c.textSecondary,
                maxLines = 1,
                modifier = Modifier.testTag("cart_line_${id}_code"),
            )
        }
        Text(
            text = name,
            style = t.body.copy(fontSize = 17.sp, fontWeight = t.weightBold),
            color = c.textPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(NAME_WEIGHT, fill = false).testTag("cart_line_${id}_name"),
        )
        RowChips(row, state, voucher, bowlOption, open)
    }
}

private val BowlRowHeight = 44.dp
private val AmountWidth = 78.dp
private const val NAME_WEIGHT = 2f

/** The kitchen mark of a dine-in line: a check once sent, an amber dot while not (`state-mark`). */
@Composable
private fun KitchenMark(
    id: Long,
    pending: Boolean,
    sent: Boolean,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val description =
        when {
            pending -> vueText("line_state_pending")
            sent -> vueText("line_state_sent")
            else -> null
        }
    Box(
        modifier =
            Modifier
                .width(StateMarkWidth)
                .let { if (description != null) it.semantics { contentDescription = description } else it }
                .testTag("cart_line_${id}_state"),
        contentAlignment = Alignment.Center,
    ) {
        when {
            pending -> Box(Modifier.size(PENDING_DOT).background(c.accentWarning, CircleShape))
            sent -> Text("✓", style = t.body.copy(fontSize = t.sizeMd, fontWeight = t.weightBold), color = c.textSecondary)
        }
    }
}

private val PENDING_DOT = 10.dp

/** The quantity "2×", or the voucher badge, the weight of a weighing, the "+" of a bowl's option. */
@Composable
private fun RowLead(row: CartRow) {
    val id = row.line.id
    val weight = row.line.weightGrams
    when {
        row.line.departmentId == GIFT_VOUCHER_DEPARTMENT -> VoucherBadge(id)
        row.line.bowlLineId != null -> BowlLink(id)
        weight != null -> WeightBadge(id, weight)
        else ->
            Text(
                text = "${row.line.quantity}×",
                style =
                    PiliTheme.typography.figure.copy(
                        fontSize = PiliTheme.typography.sizeLg,
                        fontWeight = PiliTheme.typography.weightBold,
                    ),
                color = PiliTheme.colors.textPrimary,
                maxLines = 1,
                modifier = Modifier.width(LeadWidth).testTag("cart_line_${id}_lead"),
            )
    }
}

/** The chips of the row, inline and truncated: the options, the kitchen note, the course and the first price change. */
@Composable
private fun RowScope.RowChips(
    row: CartRow,
    state: OrderCartState,
    voucher: Boolean,
    bowlOption: Boolean,
    open: Boolean,
) {
    val line = row.line
    val id = line.id
    val c = PiliTheme.colors
    val options = line.bowlOptions.joinToString(" · ").ifEmpty { lineNotes(line.modifiersJson) }
    if (options.isNotEmpty()) RowChip(options, c.accentWarning, open, "cart_line_${id}_options")
    if (!line.kitchenNote.isNullOrEmpty()) {
        RowChip(
            line.kitchenNote.orEmpty(),
            c.textSecondary,
            open,
            "cart_line_${id}_kitchen_note",
            italic = true,
        )
    }
    if (state.coursesEnabled && !voucher && !bowlOption) {
        val held = line.kitchenState == "held"
        val label = vueText("course_${line.course}") + if (held) " · " + vueText("course_held_tag") else ""
        RowChip(
            label,
            if (held) c.accentWarning else c.textPrimary,
            open,
            "cart_line_${id}_course_tag",
            bold = true,
            faded = line.kitchenState == "printed",
        )
    }
    line.priceChanges.firstOrNull()?.let { RowChip(it.name, c.accentWarning, open, "cart_line_${id}_change_chip", bold = true) }
}

/** One chip of the row: a pill that shrinks and ends in an ellipsis before the amount is pushed. */
@Composable
private fun RowScope.RowChip(
    text: String,
    color: Color,
    open: Boolean,
    tag: String,
    italic: Boolean = false,
    bold: Boolean = false,
    faded: Boolean = false,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Text(
        text = text,
        style =
            t.body.copy(
                fontSize = 13.sp,
                fontStyle = if (italic) FontStyle.Italic else FontStyle.Normal,
                fontWeight = if (bold) t.weightBold else t.weightNormal,
            ),
        color = color,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier =
            Modifier
                .weight(1f, fill = false)
                .alpha(if (faded) CHIP_LOCKED_ALPHA else 1f)
                .background(if (open) c.bgPrimary else c.bgSurface, RoundedCornerShape(ChipRadius))
                .padding(horizontal = 10.dp, vertical = 2.dp)
                .testTag(tag),
    )
}

/** The selected line: what the row truncates, then the actions. */
@Composable
private fun LineDetail(
    row: CartRow,
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val d = PiliTheme.dimens
    val id = row.line.id
    Column(
        modifier =
            Modifier.fillMaxWidth().padding(
                start = d.spaceXs,
                end = d.spaceXs,
                bottom = d.spaceSm,
            ).testTag("cart_line_${id}_detail"),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        DetailInfo(row, state)
        LineActions(row, state, actions)
    }
}

/** What the row truncates: the category, the price per kilo, the options, the whole note and each price change. */
@Composable
private fun DetailInfo(
    row: CartRow,
    state: OrderCartState,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val line = row.line
    val id = line.id
    val voucher = line.departmentId == GIFT_VOUCHER_DEPARTMENT
    val weight = line.weightGrams
    val category = if (voucher) null else lineCategory(line, state.language)
    val options = line.bowlOptions.joinToString(" · ").ifEmpty { lineNotes(line.modifiersJson) }
    Column(Modifier.padding(start = DetailInset)) {
        category?.let {
            Text(
                it,
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
                modifier = Modifier.testTag("cart_line_${id}_sub"),
            )
        }
        if (weight != null) {
            Text(
                text = formatCents(line.unitPriceCents) + " €/kg",
                style = t.figure.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
                modifier = Modifier.testTag("cart_line_${id}_per_kilo"),
            )
        }
        if (options.isNotEmpty()) {
            Text(
                options,
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.accentWarning,
                modifier = Modifier.testTag("cart_line_${id}_notes"),
            )
        }
        if (!line.kitchenNote.isNullOrEmpty()) {
            Text(
                text = line.kitchenNote.orEmpty(),
                style = t.body.copy(fontSize = t.sizeSm, fontStyle = FontStyle.Italic),
                color = c.textSecondary,
                modifier = Modifier.testTag("cart_line_${id}_kitchen_note_full"),
            )
        }
        line.priceChanges.forEachIndexed { index, change ->
            Row(Modifier.fillMaxWidth().testTag("cart_line_${id}_change_$index"), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(change.name, style = t.body.copy(fontSize = t.sizeSm), color = c.accentWarning)
                Text(formatCents(change.amountCents), style = t.figure.copy(fontSize = t.sizeSm), color = c.accentWarning)
            }
        }
    }
}

/** The actions of the selected line, in the order of the Vue detail; "Supprimer" last, set apart on the right. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun LineActions(
    row: CartRow,
    state: OrderCartState,
    actions: OrderCartActions,
) {
    val d = PiliTheme.dimens
    val line = row.line
    val id = line.id
    val voucher = line.departmentId == GIFT_VOUCHER_DEPARTMENT
    val bowlOption = line.bowlLineId != null
    val canOffer = row.canOffer && !voucher && !bowlOption
    FlowRow(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
        itemVerticalAlignment = Alignment.CenterVertically,
    ) {
        if (!voucher && line.weightGrams == null) QuantityStepper(id, line.quantity, actions)
        if (row.canNote && !voucher) {
            PiliButton(
                text = vueText("note_button"),
                onClick = { actions.onNote(id) },
                variant = if (line.kitchenNote.isNullOrEmpty()) PiliButtonVariant.Secondary else PiliButtonVariant.Primary,
                modifier = Modifier.testTag("cart_line_${id}_note"),
            )
        }
        if (canOffer) {
            PiliButton(
                text = vueText("discount_offer"),
                onClick = { actions.onOffer(id) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("cart_line_${id}_offer"),
            )
        }
        if (row.canRemoveDiscount) {
            PiliButton(
                text = vueText("discount_remove"),
                onClick = { actions.onRemoveLineDiscount(id) },
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("cart_line_${id}_remove_discount"),
            )
        }
        if (state.coursesEnabled && !voucher) CourseButton(line, actions)
        RemoveButton(id, actions)
    }
}

/**
 * The course picker of the selected line (the `course-chip` button of `OrderLineItem.vue`): a tap opens the picker;
 * once a slip carried the line the button is locked, and a line held for "Envoyer la suite" says so. Test tag:
 * `cart_line_<id>_course`.
 */
@Composable
private fun CourseButton(
    line: CartLine,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val printed = line.kitchenState == "printed"
    val held = line.kitchenState == "held"
    val label = vueText("course_${line.course}")
    val description = vueText("course_chip_label", "course" to label)
    Row(
        modifier =
            Modifier
                .defaultMinSize(minHeight = ControlSize)
                .alpha(if (printed) CHIP_LOCKED_ALPHA else 1f)
                .background(c.bgPrimary, RoundedCornerShape(d.radiusMd))
                .clickable(enabled = !printed, role = Role.Button) { actions.onCourse(line.id) }
                .padding(horizontal = d.spaceMd)
                .semantics { contentDescription = description }
                .testTag("cart_line_${line.id}_course"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = t.figure.copy(fontSize = t.sizeMd, fontWeight = t.weightBold),
            color = if (held) c.accentWarning else c.textPrimary,
        )
        if (held) {
            Text(
                text = vueText("course_held_tag").uppercase(),
                style = t.figure.copy(fontSize = t.sizeSm),
                color = c.accentWarning,
            )
        }
    }
}

private const val CHIP_LOCKED_ALPHA = 0.55f

/** "Supprimer": the destructive action, last and in red (`cart_line_<id>_remove`). */
@Composable
private fun RemoveButton(
    id: Long,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val description = vueText("line_remove")
    Row(
        modifier =
            Modifier
                .defaultMinSize(minHeight = ControlSize)
                .background(c.bgPrimary, RoundedCornerShape(d.radiusMd))
                .clickable(role = Role.Button) { actions.onRemove(id) }
                .padding(horizontal = d.spaceMd)
                .semantics { contentDescription = description }
                .testTag("cart_line_${id}_remove"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(Icons.Filled.Close, contentDescription = null, tint = c.accentDanger, modifier = Modifier.size(REMOVE_ICON))
        Text(vueText("line_delete"), style = t.body.copy(fontWeight = t.weightBold), color = c.accentDanger)
    }
}

private val REMOVE_ICON = 16.dp

/** A gift voucher sold has a fixed quantity (one line per voucher): a badge replaces the stepper. Tag `cart_line_<id>_voucher`. */
@Composable
internal fun VoucherBadge(id: Long) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Text(
        text = vueText("gv_line_badge"),
        style = t.body.copy(fontWeight = t.weightBold),
        color = c.selectedFg,
        textAlign = TextAlign.Center,
        modifier =
            Modifier
                .width(VoucherBadgeWidth)
                .background(c.selectedBg, RoundedCornerShape(d.radiusMd))
                .padding(vertical = 6.dp)
                .testTag("cart_line_${id}_voucher"),
    )
}

private val VoucherBadgeWidth = 64.dp

/** The minus, the quantity and the plus; the minus stops at 1. */
@Composable
internal fun QuantityStepper(
    id: Long,
    quantity: Int,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(
        modifier = Modifier.background(c.bgPrimary, RoundedCornerShape(d.radiusMd)).padding(2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        LineControl(
            icon = Icons.Filled.Remove,
            description = vueText("qty_less"),
            enabled = quantity > 1,
            modifier = Modifier.testTag("cart_line_${id}_minus"),
        ) { actions.onDecrement(id) }
        Text(
            text = "$quantity",
            style = t.figure.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
            color = c.textPrimary,
            textAlign = TextAlign.Center,
            modifier = Modifier.width(QuantityWidth).testTag("cart_line_${id}_qty"),
        )
        LineControl(
            icon = Icons.Filled.Add,
            description = vueText("qty_more"),
            enabled = true,
            modifier = Modifier.testTag("cart_line_${id}_plus"),
        ) { actions.onIncrement(id) }
    }
}

/** A 48 dp touch target with an icon; [enabled] false draws it faded and not clickable. */
@Composable
internal fun LineControl(
    icon: ImageVector,
    description: String,
    enabled: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    Box(
        modifier =
            modifier
                .size(ControlSize)
                .alpha(if (enabled) 1f else DISABLED_ALPHA)
                .clickable(enabled = enabled, role = Role.Button, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, contentDescription = description, tint = c.textPrimary)
    }
}

private const val DISABLED_ALPHA = 0.35f

/** The amount of a line: its total, or quantity and unit price for a line the VAT split leaves out (PD208). */
internal fun amountText(amount: LineAmount?): String =
    when (amount) {
        is LineAmount.Total -> formatEuro(amount.cents)
        is LineAmount.PerUnit -> "${amount.quantity} × ${formatEuro(amount.unitCents)}"
        null -> ""
    }

/**
 * The notes of a line ("Sans sésame"): `modifiers_json` holds a string or a list of strings, shown joined by a
 * dot; any other JSON (an object, a number) shows nothing, and text that is not JSON is shown as it is (`notes` of
 * `OrderLineItem.vue`, where `JSON.parse` is strict).
 */
internal fun lineNotes(raw: String?): String {
    val text = raw?.trim().orEmpty()
    if (text.isEmpty()) return ""
    return try {
        when {
            text.startsWith("[") -> JSONArray(text).strings().joinToString(" · ")
            text.startsWith("\"") -> JSONTokener(text).nextValue() as? String ?: ""
            text.startsWith("{") -> JSONObject(text).let { "" }
            text.toDoubleOrNull() != null || text in JSON_LITERALS -> ""
            else -> raw.orEmpty()
        }
    } catch (_: JSONException) {
        raw.orEmpty()
    }
}

private val JSON_LITERALS = setOf("null", "true", "false")

private fun JSONArray.strings(): List<String> = (0 until length()).mapNotNull { opt(it) as? String }
