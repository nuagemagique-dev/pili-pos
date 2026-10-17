@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.NumPad
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.TicketMediumPicker
import dev.nuagemagique.pilipos.ui.components.TicketMediumPickerState
import dev.nuagemagique.pilipos.ui.components.defaultTicketMediumPickerTexts
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.bilingualName
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.LocalUiLanguage
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONTokener
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.EftPayment
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest

private val MethodHeight = 80.dp
private val SplitButtonWidth = 72.dp
private val SplitHeight = 64.dp
private val ChipMinHeight = 72.dp
private val ExactHeight = 80.dp
private val QuickHeight = 64.dp
private val EntryDisplayHeight = 80.dp
private val PadWidth = 296.dp
private val ValidateHeight = 88.dp
private val ActionHeight = 88.dp
private val Hairline = 1.dp
private val CurrentBorder = 2.dp

/** The label of a till method (`method_cash`, `method_card`, `method_meal_voucher`). */
@Composable
private fun methodLabel(method: PaymentMethod): String =
    when (method) {
        PaymentMethod.CASH -> vueText("method_cash")
        PaymentMethod.CARD -> vueText("method_card")
        PaymentMethod.MEAL_VOUCHER -> vueText("method_meal_voucher")
        PaymentMethod.GIFT_VOUCHER -> vueText("method_gift_voucher")
        else -> method.name
    }

private fun methodTag(method: PaymentMethod): String =
    when (method) {
        PaymentMethod.CASH -> "checkout_method_cash"
        PaymentMethod.CARD -> "checkout_method_card"
        PaymentMethod.GIFT_VOUCHER -> "checkout_method_gift_voucher"
        else -> "checkout_method_meal_voucher"
    }

/** The payment panel: the methods, the equal split, the part row, the amount entry and the ticket medium. */
@Composable
internal fun PayPanel(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
    picker: TicketMediumPickerState,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    Column(
        modifier = modifier.padding(d.spaceLg).testTag("checkout_pay_panel"),
        verticalArrangement = Arrangement.spacedBy(d.spaceMd),
    ) {
        state.nextHeldCourse?.let { HeldWarning(it, state.sendingHeld, callbacks) }
        MethodRow(state, callbacks)
        SplitRow(state, callbacks)
        if (state.amounts.chips.size > 1) ChipRow(state)
        EntryArea(state, callbacks, Modifier.weight(1f).fillMaxWidth())
        if (state.mailStatus?.configured == true && state.taken.isEmpty()) TicketRow(state, callbacks, picker)
    }
}

/** A later course is still held at the table: say so before paying, with "Envoyer maintenant" (`held-warning`). */
@Composable
private fun HeldWarning(
    course: Int,
    sending: Boolean,
    callbacks: CheckoutCallbacks,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(c.accentWarning.copy(alpha = HELD_ALPHA), RoundedCornerShape(d.radiusMd))
                .border(1.dp, c.accentWarning, RoundedCornerShape(d.radiusMd))
                .padding(horizontal = d.spaceMd, vertical = d.spaceSm)
                .testTag("checkout_held_warning"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = vueText("checkout_held_warning", "course" to vueText("course_$course")),
            style = t.body.copy(fontWeight = t.weightBold),
            color = c.accentWarning,
            modifier = Modifier.weight(1f),
        )
        PiliButton(
            text = vueText("checkout_send_now"),
            onClick = callbacks.onSendHeld,
            variant = PiliButtonVariant.Secondary,
            enabled = !sending,
            modifier = Modifier.height(QuickHeight).testTag("checkout_held_send"),
        )
    }
}

private const val HELD_ALPHA = 0.12f

@Composable
private fun MethodRow(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
        state.methods.forEach { method ->
            PiliButton(
                text = methodLabel(method),
                onClick = { callbacks.onMethod(method) },
                variant = if (state.method == method) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                enabled = !(method == PaymentMethod.CASH && state.hasCashPart),
                modifier = Modifier.weight(1f).height(MethodHeight).testTag(methodTag(method)),
            )
        }
    }
}

/** Split in 1 to 5 equal parts, only before the first part is taken (UX guide section 7.4). */
@Composable
private fun SplitRow(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
) {
    val d = PiliTheme.dimens
    Row(horizontalArrangement = Arrangement.spacedBy(d.spaceSm), verticalAlignment = Alignment.CenterVertically) {
        Text(vueText("split_into"), style = PiliTheme.typography.label, color = PiliTheme.colors.textSecondary)
        for (n in 1..SPLIT_MAX) {
            PiliButton(
                text = n.toString(),
                onClick = { callbacks.onSplit(n) },
                variant = if (state.splitCount == n) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                enabled = state.splitCount == n || state.splitAllowed(n),
                modifier = Modifier.width(SplitButtonWidth).height(SplitHeight).testTag("checkout_split_$n"),
            )
        }
        Text(vueText("equal_parts"), style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
    }
}

/** Every part: taken, the current one, and the ones to come. */
@Composable
private fun ChipRow(state: CheckoutUiState) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
        state.amounts.chips.forEachIndexed { index, chip ->
            val fill =
                when (chip.state) {
                    ChipState.PAID -> c.selectedBg
                    ChipState.CURRENT -> c.bgSurface
                    ChipState.NEXT -> c.bgSecondary
                }
            val shape = RoundedCornerShape(d.radiusMd)
            val current = chip.state == ChipState.CURRENT
            val borderWidth = if (current) CurrentBorder else Hairline
            val borderColor = if (current) c.accentPrimary else c.border
            Column(
                modifier =
                    Modifier
                        .weight(1f)
                        .height(ChipMinHeight)
                        .background(fill, shape)
                        .border(borderWidth, borderColor, shape)
                        .padding(horizontal = d.spaceSm, vertical = d.spaceXs)
                        .semantics(mergeDescendants = true) {}
                        .testTag("checkout_chip_$index"),
                verticalArrangement = Arrangement.Center,
            ) {
                Text(vueText("part_n", "n" to (index + 1)), style = t.label, color = c.textSecondary)
                Text(
                    text = chipLabel(chip),
                    style = t.figure.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
                    color = if (chip.state == ChipState.PAID) c.selectedFg else c.textPrimary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
private fun chipLabel(chip: PartChip): String {
    val amount = formatCents(chip.amountCents)
    val method = chip.method?.let { methodLabel(it) } ?: ""
    return when (chip.state) {
        ChipState.PAID -> uiText(R.string.ui_checkout_chip_paid, "method" to method, "amount" to amount)
        ChipState.CURRENT -> uiText(R.string.ui_checkout_chip_current, "method" to method, "amount" to amount)
        ChipState.NEXT -> uiText(R.string.ui_checkout_chip_next, "amount" to amount)
    }
}

/** The shortcuts and the hints on the left, the entry display and the pad on the right. */
@Composable
private fun EntryArea(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
    modifier: Modifier,
) {
    val d = PiliTheme.dimens
    val cash = state.method == PaymentMethod.CASH
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(d.spaceLg)) {
        Column(Modifier.weight(1f).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(d.spaceMd)) {
            Text(
                text = if (cash) vueText("amount_received") else vueText("amount"),
                style = PiliTheme.typography.label,
                color = PiliTheme.colors.textSecondary,
            )
            ExactButton(state, callbacks.onExact)
            if (cash) {
                QuickAmounts(state, callbacks.onQuick)
                ChangeBox(state)
            } else {
                CardHints(state, callbacks)
            }
        }
        Column(Modifier.width(PadWidth), verticalArrangement = Arrangement.spacedBy(d.spaceMd)) {
            EntryDisplay(state, cash)
            NumPad(
                onKey = callbacks.onKey,
                clearLabel = uiText(R.string.ui_common_numpad_clear),
                backspaceLabel = uiText(R.string.ui_common_numpad_backspace),
            )
        }
    }
}

/** The exact amount: the rounded cash due for cash (with the exact amount under it), the part for card. */
@Composable
private fun ExactButton(
    state: CheckoutUiState,
    onExact: () -> Unit,
) {
    val cash = state.method == PaymentMethod.CASH
    val shown = if (cash) state.amounts.roundedDueCents else state.amounts.dueCents
    PiliButton(
        onClick = onExact,
        variant = if (state.exactActive) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
        modifier = Modifier.fillMaxWidth().height(ExactHeight).testTag("checkout_exact"),
    ) {
        Text(vueText("exact"), style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold))
        Column(Modifier.weight(1f), horizontalAlignment = Alignment.End) {
            Text(
                text = formatEuro(shown),
                style = PiliTheme.typography.price,
                modifier = Modifier.testTag("checkout_exact_amount"),
            )
            if (cash && state.amounts.roundedDueCents != state.amounts.dueCents) {
                Text(
                    text = vueText("exact_rounded", "exact" to formatCents(state.amounts.dueCents)),
                    style = PiliTheme.typography.label,
                    modifier = Modifier.testTag("checkout_exact_rounded"),
                )
            }
        }
    }
}

@Composable
private fun QuickAmounts(
    state: CheckoutUiState,
    onQuick: (Long) -> Unit,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        state.amounts.quickAmountsCents.forEach { quick ->
            PiliButton(
                text = formatEuro(quick),
                onClick = { onQuick(quick) },
                variant = if (state.quickActive(quick)) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(QuickHeight).testTag("checkout_quick_$quick"),
            )
        }
    }
}

/** The change to give back, or a dash while the cash handed over does not cover the rounded due. */
@Composable
private fun ChangeBox(state: CheckoutUiState) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val covered = state.canPayCash
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(c.bgSecondary, RoundedCornerShape(d.radiusMd))
                .padding(d.spaceMd)
                .testTag("checkout_change"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(vueText("change"), style = t.label, color = c.textSecondary)
        Text(
            text = if (covered) formatEuro(state.changeCents) else "-",
            style = t.price.copy(fontSize = t.sizeXl),
            color = if (covered) c.accentSuccess else c.textSecondary,
            modifier = Modifier.testTag("checkout_change_value"),
        )
    }
}

@Composable
private fun CardHints(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    if (state.exactTooHigh) {
        Text(
            text = vueText("amount_over_rest"),
            style = t.body.copy(fontWeight = t.weightBold),
            color = c.accentDanger,
            modifier = Modifier.testTag("checkout_too_high"),
        )
    }
    if (state.method == PaymentMethod.CARD && state.eftActive) CardModeRow(state, callbacks)
    Text(
        text = if (state.viaTerminal) uiText(R.string.ui_eft_hint_terminal) else vueText("card_wait"),
        style = t.body,
        color = c.textSecondary,
        modifier = Modifier.testTag("checkout_card_hint"),
    )
    if (state.hasCashPart) {
        Text(vueText("cash_part_taken"), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("checkout_cash_taken"))
    }
}

/** "Terminal" or "Saisie manuelle": the card part is charged on the linked terminal, or typed by hand (terminal down). */
@Composable
private fun CardModeRow(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
) {
    Row(Modifier.fillMaxWidth().testTag("checkout_eft_mode"), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        PiliButton(
            text = uiText(R.string.ui_eft_mode_terminal),
            onClick = { callbacks.onCardManual(false) },
            variant = if (!state.cardManual) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
            modifier = Modifier.weight(1f).height(QuickHeight).testTag("checkout_eft_mode_terminal"),
        )
        PiliButton(
            text = uiText(R.string.ui_eft_mode_manual),
            onClick = { callbacks.onCardManual(true) },
            variant = if (state.cardManual) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
            modifier = Modifier.weight(1f).height(QuickHeight).testTag("checkout_eft_mode_manual"),
        )
    }
}

/** What is typed, read as cents like a payment terminal and shown with two decimals. */
@Composable
private fun EntryDisplay(
    state: CheckoutUiState,
    cash: Boolean,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(EntryDisplayHeight)
                .background(c.bgSecondary, RoundedCornerShape(d.radiusMd))
                .padding(horizontal = d.spaceMd),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(if (cash) vueText("received") else vueText("amount"), style = t.label, color = c.textSecondary)
        Text(
            text = formatCents(state.entryCents),
            style = t.price.copy(fontSize = t.sizeXl),
            color = if (cash) c.textPrimary else c.accentPrimary,
            maxLines = 1,
            modifier = Modifier.testTag("checkout_entry"),
        )
    }
}

/** The ticket medium (note 05): shown only when e-mail is configured and before the first part is taken. */
@Composable
private fun TicketRow(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
    picker: TicketMediumPickerState,
) {
    Row(
        modifier = Modifier.fillMaxWidth().testTag("checkout_ticket_row"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(vueText("ticket_medium"), style = PiliTheme.typography.label, color = PiliTheme.colors.textSecondary)
        Box(Modifier.weight(1f)) {
            TicketMediumPicker(
                mailStatus = state.mailStatus,
                request = state.deliveryRequest ?: TicketDeliveryRequest(RequestedMedium.PAPER, null),
                state = picker,
                onPaper = callbacks.onPaper,
                texts = defaultTicketMediumPickerTexts(),
            )
        }
    }
}

// ── The recap ─────────────────────────────────────────────────────

/** The recap of the order, the total, the parts taken, what is left and the actions. */
@Composable
internal fun SummaryPanel(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    Column(modifier.background(c.bgSecondary).testTag("checkout_summary")) {
        SummaryHead(state)
        Column(
            modifier =
                Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = PiliTheme.dimens.spaceMd),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        ) {
            state.summary.lines.forEachIndexed { index, line ->
                SummaryLine(line, state.summary.amounts.getOrNull(index), state)
            }
        }
        SummaryFoot(state, callbacks)
    }
}

@Composable
private fun SummaryHead(state: CheckoutUiState) {
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val where =
        if (state.orderType == OrderType.DINE_IN && state.tableNumber != null) {
            uiText(R.string.ui_checkout_table, "table" to vueText("table"), "number" to state.tableNumber)
        } else {
            vueText("takeaway_badge")
        }
    Row(
        modifier = Modifier.fillMaxWidth().padding(d.spaceMd),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = uiText(R.string.ui_checkout_title, "order" to vueText("order"), "where" to where),
            style = t.title,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.testTag("checkout_title"),
        )
        Text(
            text = vueText("items_count", "n" to state.itemCount),
            style = t.body,
            color = PiliTheme.colors.textSecondary,
            modifier = Modifier.testTag("checkout_item_count"),
        )
    }
}

@Composable
private fun SummaryLine(
    line: CartLine,
    amount: LineAmount?,
    state: CheckoutUiState,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val name = bilingualName(line, LocalUiLanguage.current, state.labelMode)
    val notes = lineNotes(line.modifiersJson)
    Row(
        modifier = Modifier.fillMaxWidth().semantics(mergeDescendants = true) {}.testTag("checkout_line_${line.id}"),
        horizontalArrangement = Arrangement.spacedBy(d.spaceSm),
        verticalAlignment = Alignment.Top,
    ) {
        Text(uiText(R.string.ui_checkout_line_qty, "quantity" to line.quantity), style = t.figure, color = c.textSecondary)
        Column(Modifier.weight(1f)) {
            Text(name.primary, style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
            name.secondary?.let { Text(it, style = t.label, color = c.textSecondary) }
            notes?.let { Text(it, style = t.label, color = c.accentWarning) }
        }
        Text(
            text =
                when (amount) {
                    is LineAmount.Total -> formatCents(amount.cents)
                    is LineAmount.PerUnit ->
                        uiText(R.string.ui_checkout_per_unit, "quantity" to amount.quantity, "unit" to formatCents(amount.unitCents))
                    null -> ""
                },
            style = t.figure.copy(fontWeight = t.weightBold),
            color = c.textPrimary,
            textAlign = TextAlign.End,
        )
    }
}

/** The notes or modifiers of a line, when it has some: a JSON list of strings, or a plain string (Vue `lineNotes`). */
internal fun lineNotes(raw: String?): String? {
    if (raw.isNullOrEmpty()) return null
    return try {
        when (raw.first()) {
            '[' -> JSONArray(raw).let { list -> (0 until list.length()).joinToString(" · ") { list.opt(it).toString() } }.ifEmpty { null }
            '"' -> (JSONTokener(raw).nextValue() as? String)?.ifEmpty { null }
            // Anything else is shown as is, as Vue does for text that is not a list or a string.
            else -> raw
        }
    } catch (_: JSONException) {
        // Not JSON: shown as is.
        raw
    }
}

@Composable
private fun SummaryFoot(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Column(
        modifier = Modifier.fillMaxWidth().background(c.bgPrimary).padding(d.spaceMd),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        FootRow(vueText("total"), uiText(R.string.ui_checkout_amount_eur, "amount" to formatCents(state.totalCents)), "checkout_total")
        state.taken.forEachIndexed { index, part -> TakenRow(index, part) }
        state.eftOpen.forEach { OpenPaymentRow(it, callbacks.onVerify) }
        FootRow(
            vueText("rest_to_pay"),
            uiText(R.string.ui_checkout_amount_eur, "amount" to formatCents(state.amounts.restShownCents)),
            "checkout_rest",
            emphasised = true,
        )
        if (state.eftBlocked) {
            Text(
                text = uiText(R.string.ui_eft_blocking),
                style = t.body.copy(fontWeight = t.weightBold),
                color = c.accentDanger,
                modifier = Modifier.fillMaxWidth().testTag("checkout_eft_blocking"),
            )
        }
        state.error?.let {
            Text(
                text = it,
                style = t.body.copy(fontWeight = t.weightBold),
                color = c.accentDanger,
                modifier = Modifier.fillMaxWidth().testTag("checkout_error"),
            )
        }
        if (!state.canTakePayment) {
            Text(
                text = uiText(R.string.ui_checkout_no_payment),
                style = t.body,
                color = c.accentWarning,
                modifier = Modifier.testTag("checkout_no_rights"),
            )
        }
        FootActions(state, callbacks)
    }
}

/** Back (or clear the parts once one is taken) and validate. */
@Composable
private fun FootActions(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
        // A card the terminal charged stays: only the parts typed by hand can be cleared.
        if (state.taken.any { !it.locked }) {
            PiliButton(
                text = vueText("clear_payments"),
                onClick = callbacks.onClearParts,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("checkout_clear_parts"),
            )
        } else {
            PiliButton(
                text = vueText("back"),
                onClick = callbacks.onBack,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(ActionHeight).testTag("checkout_back"),
            )
        }
        ValidateButton(state, callbacks.onValidate, Modifier.weight(2f))
    }
}

/** A part taken: for a card the terminal charged, its brand and the "Accepte" badge (the part is locked). */
@Composable
private fun TakenRow(
    index: Int,
    part: TakenPart,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val partName = vueText("part_n", "n" to (index + 1))
    val value = uiText(R.string.ui_checkout_paid_part, "amount" to formatCents(part.cents))
    if (!part.locked) {
        val method = methodLabel(part.method) + (part.codeMasked?.let { " · $it" } ?: "")
        val label = uiText(R.string.ui_checkout_part_label, "part" to partName, "method" to method)
        FootRow(label, value, "checkout_part_$index")
        return
    }
    val label =
        part.brand?.let { uiText(R.string.ui_eft_locked_part, "part" to partName, "method" to methodLabel(part.method), "brand" to it) }
            ?: uiText(R.string.ui_checkout_part_label, "part" to partName, "method" to methodLabel(part.method))
    Row(
        Modifier.fillMaxWidth().testTag("checkout_part_locked_$index"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Row(Modifier.weight(1f), horizontalArrangement = Arrangement.spacedBy(d.spaceSm), verticalAlignment = Alignment.CenterVertically) {
            Text(label, style = t.body.copy(fontWeight = t.weightBold), color = c.textSecondary)
            Text(
                text = uiText(R.string.ui_eft_approved),
                style = t.label.copy(fontWeight = t.weightBold),
                color = c.textOnAccent,
                modifier =
                    Modifier
                        .background(c.accentSuccess, RoundedCornerShape(d.radiusSm))
                        .padding(horizontal = d.spaceSm, vertical = d.spaceXs)
                        .testTag("checkout_part_eft_$index"),
            )
        }
        Text(value, style = t.price, color = c.textPrimary, modifier = Modifier.testTag("checkout_part_$index"))
    }
}

/** A card payment still waiting for the terminal's answer, with "Verifier" (the sale is blocked until it is checked). */
@Composable
private fun OpenPaymentRow(
    payment: EftPayment,
    onVerify: (EftPayment) -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Row(
        Modifier.fillMaxWidth().testTag("checkout_eft_open_${payment.id}"),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "${methodLabel(PaymentMethod.CARD)} · ${formatEuro(payment.amountCents)} · ${uiText(R.string.ui_eft_unknown_row)}",
            style = t.body.copy(fontWeight = t.weightBold),
            color = c.accentWarning,
            modifier = Modifier.weight(1f),
        )
        PiliButton(
            text = uiText(R.string.ui_eft_check),
            onClick = { onVerify(payment) },
            modifier = Modifier.height(QuickHeight).testTag("checkout_eft_verify_${payment.id}"),
        )
    }
}

@Composable
private fun FootRow(
    label: String,
    value: String,
    tag: String,
    emphasised: Boolean = false,
) {
    val t = PiliTheme.typography
    val style = if (emphasised) t.price.copy(fontSize = t.sizeXl) else t.price
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = t.body.copy(fontWeight = t.weightBold), color = PiliTheme.colors.textSecondary)
        Text(value, style = style, color = PiliTheme.colors.textPrimary, modifier = Modifier.testTag(tag))
    }
}

@Composable
private fun ValidateButton(
    state: CheckoutUiState,
    onValidate: () -> Unit,
    modifier: Modifier,
) {
    val cash = state.method == PaymentMethod.CASH
    PiliButton(
        onClick = onValidate,
        enabled = state.canValidate,
        modifier = modifier.height(ValidateHeight).testTag("checkout_validate"),
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text =
                    if (state.splitCount > 1) {
                        vueText("validate_part", "n" to (state.partIndex + 1))
                    } else {
                        vueText("validate")
                    },
                style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            )
            if (cash && state.entry.digits.isNotEmpty()) {
                Text(
                    text = vueText("received_amount", "amount" to formatCents(state.entryCents)),
                    style = PiliTheme.typography.label,
                    modifier = Modifier.testTag("checkout_validate_amount"),
                )
            } else if (!cash && state.entryCents > 0L) {
                Text(
                    text = formatEuro(state.entryCents),
                    style = PiliTheme.typography.label,
                    modifier = Modifier.testTag("checkout_validate_amount"),
                )
            }
        }
    }
}
