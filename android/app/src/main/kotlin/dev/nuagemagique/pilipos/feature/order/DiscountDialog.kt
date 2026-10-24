package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.DiscountInput
import uniffi.pos_ffi.DiscountKind
import uniffi.pos_ffi.DiscountReason
import uniffi.pos_ffi.DiscountTarget

/** The quick percents (`QUICK_PERCENTS` of `DiscountDialog.vue`). */
@Suppress("MagicNumber")
internal val QuickPercents: List<Long> = listOf(5L, 10L, 15L, 20L, 50L)

/** The longest free text of a reason, as the engine takes it. */
internal const val REASON_TEXT_MAX: Int = 120

private const val PERCENT_MAX_DIGITS = 2
private const val AMOUNT_MAX_DIGITS = 7
private const val PERCENT_MAX = 99L

private val DialogMaxWidth = 880.dp
private val TabHeight = 56.dp
private val ReasonHeight = 64.dp
private val FieldHeight = 56.dp
private val Hairline = 1.dp

/**
 * What the discount dialog holds (the refs of `DiscountDialog.vue`): the kind, the digits typed on the pad (a
 * percent, or the amount in cents like a payment terminal), the reason and its text. Pure, so the rules are
 * tested without a screen. [baseCents] is what the discount applies to: the order's total, or the line's amount.
 */
@Immutable
internal data class DiscountEntry(
    val kind: DiscountKind,
    val digits: String = "",
    val reason: DiscountReason? = null,
    val reasonText: String = "",
) {
    /** The digits read as a number; 0 while nothing is typed. */
    val value: Long get() = digits.toLongOrNull() ?: 0L

    /** A new kind clears what was typed. */
    fun withKind(next: DiscountKind): DiscountEntry = copy(kind = next, digits = "")

    fun withQuickPercent(percent: Long): DiscountEntry = copy(digits = percent.toString())

    /** The state after [key]: at most two digits for a percent, seven for an amount, no leading zero. */
    fun press(key: NumPadKey): DiscountEntry {
        val max = if (kind == DiscountKind.PERCENT) PERCENT_MAX_DIGITS else AMOUNT_MAX_DIGITS
        return when (key) {
            NumPadKey.Backspace -> copy(digits = digits.dropLast(1))
            NumPadKey.Clear -> copy(digits = "")
            is NumPadKey.Digit ->
                if (digits.length >= max || (digits.isEmpty() && key.digit == '0')) this else copy(digits = digits + key.digit)
        }
    }

    /** Whether the typed value is allowed: always for "Offert", a percent from 1 to 99, an amount up to [baseCents]. */
    fun valueOk(baseCents: Long): Boolean =
        when (kind) {
            DiscountKind.FREE -> true
            DiscountKind.PERCENT -> value in 1L..PERCENT_MAX
            DiscountKind.AMOUNT -> value in 1L..baseCents
        }

    /** The discount may be confirmed: a valid value, a reason, and something to take off. */
    fun canConfirm(baseCents: Long): Boolean = valueOk(baseCents) && reason != null && baseCents > 0L

    /** The total after the discount (the dialog preview); [baseCents] while the value is not valid. */
    fun previewCents(baseCents: Long): Long = CashDisplay.totalAfterDiscountCents(baseCents, kind, value)

    /** The input for the engine; null while it cannot be confirmed. A blank text is sent as no text. */
    fun toInput(
        lineId: Long?,
        baseCents: Long,
    ): DiscountInput? {
        val picked = reason
        if (picked == null || !canConfirm(baseCents)) return null
        val text = reasonText.trim()
        return DiscountInput(
            target = if (lineId != null) DiscountTarget.Line(lineId) else DiscountTarget.Order,
            kind = kind,
            value = if (kind == DiscountKind.FREE) null else value,
            reasonCode = picked,
            reasonText = text.ifEmpty { null },
        )
    }
}

private val ReasonKeys: List<Pair<DiscountReason, String>> =
    listOf(
        DiscountReason.COMMERCIAL_GESTURE to "discount_reason_commercial_gesture",
        DiscountReason.STAFF_MEAL to "discount_reason_staff_meal",
        DiscountReason.SERVICE_ISSUE to "discount_reason_service_issue",
        DiscountReason.OTHER to "discount_reason_other",
    )

private val KindKeys: List<Pair<DiscountKind, String>> =
    listOf(
        DiscountKind.PERCENT to "discount_tab_percent",
        DiscountKind.AMOUNT to "discount_tab_amount",
        DiscountKind.FREE to "discount_tab_free",
    )

/**
 * `DiscountDialog.vue`: a manager's discount at the till. For the whole order: "%", "Montant" or "Offert" tabs,
 * the quick percents 5, 10, 15, 20 and 50, the pad for a custom percent or amount; for one line ([lineId]): only
 * "Offert". Four reason buttons are required, and an optional text is typed with the system keyboard (the Vue
 * screen has its own keypad). The preview reads "Total old -> new". The dialog only collects the input and calls
 * [onConfirm]; the caller confirms it with the PIN and the engine applies every rule.
 *
 * Test tags: `discount_dialog`, `discount_kind_<percent|amount|free>`, `discount_quick_<n>`, `discount_typed`,
 * `discount_reason_<code>`, `discount_reason_text`, `discount_preview`, `discount_confirm`, `discount_cancel`, plus
 * the pad keys.
 */
@Composable
internal fun DiscountDialog(
    baseCents: Long,
    lineId: Long?,
    onConfirm: (DiscountInput) -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var digits by rememberSaveable { mutableStateOf("") }
    var kindName by rememberSaveable { mutableStateOf(if (lineId != null) DiscountKind.FREE.name else DiscountKind.PERCENT.name) }
    var reasonName by rememberSaveable { mutableStateOf<String?>(null) }
    var text by rememberSaveable { mutableStateOf("") }
    val entry =
        DiscountEntry(DiscountKind.valueOf(kindName), digits, reasonName?.let { DiscountReason.valueOf(it) }, text)
    val isLine = lineId != null
    Box(modifier = modifier.testTag("discount_dialog")) {
        PiliDialog(
            title = vueText(if (isLine) "discount_offer_title" else "discount_title"),
            onDismissRequest = onCancel,
            maxWidth = DialogMaxWidth,
            actions = {
                PiliButton(
                    text = vueText("back"),
                    onClick = onCancel,
                    variant = PiliButtonVariant.Secondary,
                    modifier = Modifier.testTag("discount_cancel"),
                )
                PiliButton(
                    text = vueText("discount_apply"),
                    onClick = { entry.toInput(lineId, baseCents)?.let(onConfirm) },
                    enabled = entry.canConfirm(baseCents),
                    modifier = Modifier.testTag("discount_confirm"),
                )
            },
        ) {
            if (!isLine) {
                KindTabs(entry.kind) { next ->
                    val changed = entry.withKind(next)
                    kindName = changed.kind.name
                    digits = changed.digits
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceLg), verticalAlignment = Alignment.Top) {
                if (!isLine) {
                    Entry(entry, Modifier.weight(1f)) { digits = it.digits }
                }
                Reasons(
                    entry = entry,
                    modifier = Modifier.weight(1f),
                    onReason = { reasonName = it.name },
                    onText = { text = it.take(REASON_TEXT_MAX) },
                )
            }
            val preview = vueText("discount_preview", "from" to formatCents(baseCents), "to" to formatCents(entry.previewCents(baseCents)))
            Text(
                text = preview,
                style = PiliTheme.typography.price.copy(fontSize = PiliTheme.typography.sizeLg),
                color = PiliTheme.colors.textPrimary,
                modifier = Modifier.fillMaxWidth().testTag("discount_preview"),
            )
        }
    }
}

@Composable
private fun KindTabs(
    selected: DiscountKind,
    onKind: (DiscountKind) -> Unit,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        KindKeys.forEach { (kind, key) ->
            PiliButton(
                text = vueText(key),
                onClick = { onKind(kind) },
                variant = if (selected == kind) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(TabHeight).testTag("discount_kind_${kind.name.lowercase()}"),
            )
        }
    }
}

/** The quick percents, the typed value and the pad, or the "everything is free" note. */
@Composable
private fun Entry(
    entry: DiscountEntry,
    modifier: Modifier,
    onEntry: (DiscountEntry) -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(
        modifier,
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        if (entry.kind == DiscountKind.FREE) {
            Text(vueText("discount_free_note"), style = t.body, color = c.textSecondary)
            return@Column
        }
        if (entry.kind == DiscountKind.PERCENT) {
            Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
                QuickPercents.forEach { percent ->
                    PiliButton(
                        text = "$percent %",
                        onClick = { onEntry(entry.withQuickPercent(percent)) },
                        variant = if (entry.digits == percent.toString()) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                        modifier = Modifier.height(TabHeight).testTag("discount_quick_$percent"),
                    )
                }
            }
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(
                vueText(if (entry.kind == DiscountKind.PERCENT) "discount_custom_percent" else "discount_custom_amount"),
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
            )
            Text(
                text = if (entry.kind == DiscountKind.PERCENT) "${entry.value} %" else formatEuro(entry.value),
                style = t.price.copy(fontSize = t.sizeXl),
                color = c.accentPrimary,
                modifier = Modifier.testTag("discount_typed"),
            )
        }
        NumPad(
            onKey = { key -> onEntry(entry.press(key)) },
            clearLabel = uiText(R.string.ui_common_numpad_clear),
            backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
        )
    }
}

@Composable
private fun Reasons(
    entry: DiscountEntry,
    modifier: Modifier,
    onReason: (DiscountReason) -> Unit,
    onText: (String) -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(modifier, verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        Text(vueText("discount_reason"), style = t.body, color = c.textSecondary)
        ReasonKeys.chunked(2).forEach { pair ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
                pair.forEach { (reason, key) ->
                    PiliButton(
                        text = vueText(key),
                        onClick = { onReason(reason) },
                        variant = if (entry.reason == reason) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                        modifier = Modifier.weight(1f).height(ReasonHeight).testTag("discount_reason_${reason.name.lowercase()}"),
                    )
                }
            }
        }
        ReasonField(entry.reasonText, onText)
    }
}

/** The optional detail of the reason, on the system keyboard. */
@Composable
private fun ReasonField(
    value: String,
    onValueChange: (String) -> Unit,
) {
    val c = PiliTheme.colors
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        singleLine = true,
        textStyle = PiliTheme.typography.body.copy(color = c.textPrimary),
        cursorBrush = SolidColor(c.textPrimary),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
        modifier = Modifier.fillMaxWidth().testTag("discount_reason_text"),
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .defaultMinSize(minHeight = FieldHeight)
                    .border(BorderStroke(Hairline, c.border), RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .padding(horizontal = PiliTheme.dimens.spaceMd),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.isEmpty()) {
                    Text(
                        vueText("discount_reason_text_add"),
                        style = PiliTheme.typography.body,
                        color = c.textSecondary,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                inner()
            }
        },
    )
}
