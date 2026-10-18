package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Remove
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.GIFT_VOUCHER_DEPARTMENT
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.BilingualName
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.bilingualName
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import org.json.JSONTokener
import uniffi.pos_ffi.CartLine

private val ControlSize = 48.dp
private val QuantityWidth = 28.dp
private val LineMinHeight = 64.dp
private val PendingMark = 3.dp
private val PendingInset = 12.dp
private val Hairline = 1.dp

/**
 * One cart line (`OrderLineItem.vue`): the quantity with its minus and plus, the bilingual name with the notes of
 * the options, the amount and the delete control. The minus stops at 1 (UX guide 4.3: removing a line is its own
 * control, set apart on the right). [pending] marks a dine-in line that was not sent to the bar or kitchen yet
 * (amber mark on the left). The amount is the engine's (PD208): the line total from `CartAmounts.lineAmount`, or
 * the quantity and the unit price as text for a line the VAT split leaves out (code X); nothing is computed here.
 *
 * Under the line each public price change is a row (`_change_<n>`), with the manager's `_offer` and
 * `_remove_discount` buttons. The "Note" button (`_note`, filled when the line has one) opens the kitchen note
 * dialog; the note shows under the name in italics (`_kitchen_note`). A gift voucher line shows a badge instead of
 * the stepper (`_voucher`) and has no note, no offer and no course; with courses on, the course chip (`_course`) sits
 * under the name. Test tags: `cart_line_<id>`, `cart_line_<id>_minus`, `_qty`, `_plus`, `_name`, `_sub`, `_notes`,
 * `_kitchen_note`, `_course`, `_voucher`, `_amount`, `_note`, `_remove`.
 */
@Suppress("LongMethod") // One row of the cart: stepper or voucher badge, names, notes, course chip and actions.
@Composable
internal fun OrderLineItem(
    row: CartRow,
    state: OrderCartState,
    actions: OrderCartActions,
    pending: Boolean,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val id = row.line.id
    val names = bilingualName(row.line, state.language, state.labelMode)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .drawBehind {
                    drawRect(c.border, topLeft = Offset(0f, size.height - Hairline.toPx()), size = Size(size.width, Hairline.toPx()))
                    if (pending) drawRect(c.accentWarning, size = Size(PendingMark.toPx(), size.height))
                }.padding(vertical = d.spaceXs)
                .padding(start = if (pending) PendingInset else 0.dp)
                .testTag("cart_line_$id"),
    ) {
        Row(
            // The line is 64 dp high with its vertical padding, as before the change rows existed.
            modifier = Modifier.fillMaxWidth().defaultMinSize(minHeight = LineMinHeight - d.spaceXs * 2),
            horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val voucher = row.line.departmentId == GIFT_VOUCHER_DEPARTMENT
            if (voucher) VoucherBadge(id) else QuantityStepper(id, row.line.quantity, actions)
            Column(Modifier.weight(1f)) {
                LineNames(id, names, lineNotes(row.line.modifiersJson), row.line.kitchenNote)
                if (state.coursesEnabled && !voucher) CourseChip(row.line, actions)
            }
            Text(
                text = amountText(row.amount),
                style = t.figure.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
                color = c.textPrimary,
                maxLines = 1,
                modifier = Modifier.testTag("cart_line_${id}_amount"),
            )
            if (row.canNote && !voucher) {
                PiliButton(
                    text = vueText("note_button"),
                    onClick = { actions.onNote(id) },
                    variant = if (row.line.kitchenNote.isNullOrEmpty()) PiliButtonVariant.Secondary else PiliButtonVariant.Primary,
                    modifier = Modifier.testTag("cart_line_${id}_note"),
                )
            }
            // Destructive: set apart by a rule, never where the minus is.
            Box(
                modifier =
                    Modifier
                        .padding(start = d.spaceSm)
                        .drawBehind { drawRect(c.border, size = Size(Hairline.toPx(), size.height)) },
            ) {
                LineControl(
                    icon = Icons.Filled.Close,
                    description = vueText("line_remove"),
                    enabled = true,
                    modifier = Modifier.testTag("cart_line_${id}_remove"),
                ) { actions.onRemove(id) }
            }
        }
        LineChanges(row, actions)
    }
}

/** The public price changes of the line ("Remise 50 %  -20,00"), then the manager's "Offrir" or "Retirer la remise". */
@Composable
private fun LineChanges(
    row: CartRow,
    actions: OrderCartActions,
) {
    val changes = row.line.priceChanges
    // A gift voucher line is never discounted (`canOfferHere` of `OrderLineItem.vue`).
    val canOffer = row.canOffer && row.line.departmentId != GIFT_VOUCHER_DEPARTMENT
    if (changes.isEmpty() && !canOffer && !row.canRemoveDiscount) return
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val id = row.line.id
    Row(
        modifier = Modifier.fillMaxWidth().padding(start = PiliTheme.dimens.spaceXs),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            changes.forEachIndexed { index, change ->
                Row(Modifier.fillMaxWidth().testTag("cart_line_${id}_change_$index"), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text(change.name, style = t.body.copy(fontSize = t.sizeSm), color = c.accentWarning)
                    Text(formatCents(change.amountCents), style = t.figure.copy(fontSize = t.sizeSm), color = c.accentWarning)
                }
            }
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
    }
}

/** A gift voucher sold has a fixed quantity (one line per voucher): a badge replaces the stepper. Tag `cart_line_<id>_voucher`. */
@Composable
private fun VoucherBadge(id: Long) {
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
                .padding(vertical = 12.dp)
                .testTag("cart_line_${id}_voucher"),
    )
}

private val VoucherBadgeWidth = 96.dp

/** The minus, the quantity and the plus; the minus stops at 1. */
@Composable
private fun QuantityStepper(
    id: Long,
    quantity: Int,
    actions: OrderCartActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(
        modifier = Modifier.background(c.bgSurface, RoundedCornerShape(d.radiusMd)).padding(2.dp),
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

/** The bilingual name and the notes of the options. */
@Composable
private fun LineNames(
    id: Long,
    names: BilingualName,
    notes: String,
    kitchenNote: String?,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(modifier) {
        Text(
            text = names.primary,
            style = (if (names.isCode) t.figure else t.body).copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
            color = c.textPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.testTag("cart_line_${id}_name"),
        )
        names.secondary?.let {
            Text(
                text = it,
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.testTag("cart_line_${id}_sub"),
            )
        }
        if (notes.isNotEmpty()) {
            Text(
                text = notes,
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.accentWarning,
                modifier = Modifier.testTag("cart_line_${id}_notes"),
            )
        }
        if (!kitchenNote.isNullOrEmpty()) {
            Text(
                text = kitchenNote,
                style = t.body.copy(fontSize = t.sizeSm, fontStyle = FontStyle.Italic),
                color = c.textSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.testTag("cart_line_${id}_kitchen_note"),
            )
        }
    }
}

/**
 * The course of a line as a small pill under the name (`course-chip` of `OrderLineItem.vue`): a tap opens the
 * picker; once a slip carried the line the chip is locked, and a line held for "Envoyer la suite" says so.
 * Test tag: `cart_line_<id>_course`.
 */
@Composable
private fun CourseChip(
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
                .padding(top = d.spaceXs)
                .defaultMinSize(minHeight = ChipHeight)
                .alpha(if (printed) CHIP_LOCKED_ALPHA else 1f)
                .background(c.bgSurface, RoundedCornerShape(ChipHeight / 2))
                .clickable(enabled = !printed, role = Role.Button) { actions.onCourse(line.id) }
                .padding(horizontal = d.spaceMd)
                .semantics { contentDescription = description }
                .testTag("cart_line_${line.id}_course"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = t.figure.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
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

private val ChipHeight = 32.dp
private const val CHIP_LOCKED_ALPHA = 0.55f

/** A 48 dp touch target with an icon; [enabled] false draws it faded and not clickable. */
@Composable
private fun LineControl(
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
