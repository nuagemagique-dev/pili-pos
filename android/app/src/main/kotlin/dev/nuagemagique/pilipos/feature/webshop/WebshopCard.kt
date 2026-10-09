@file:OptIn(ExperimentalLayoutApi::class)
@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.webshop

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.BilingualName
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.format.bilingualName
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.LocalUiLanguage
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.WebshopOrderItem
import uniffi.pos_ffi.WebshopOrderSummary
import java.time.Instant

private val ChipShape = 8.dp
private val ChipBorder = 1.dp
private val ChipMaxWidth = 280.dp

/** What a card needs besides its order. */
internal data class CardContext(
    val now: Instant,
    val isManager: Boolean,
    val busy: Boolean,
    val reprintOpen: Boolean,
)

/**
 * One online order (`<li class="card">` of `WebshopOrdersView.vue`): the channel chip, the key time, the service and
 * the customer, the notices, the items, the total and the buttons. Test tags are prefixed `webshop_card_<id>`:
 * `_channel`, `_time`, `_meta`, `_pay`, `_banner`, `_changed`, `_review`, `_address`, `_total`, `_reject`, `_resync`,
 * `_collect_card`, `_delay`, `_reprint`, `_primary`, `_print_kitchen`, `_print_customer`.
 */
@Composable
internal fun WebshopCard(
    order: WebshopOrderSummary,
    context: CardContext,
    callbacks: WebshopCallbacks,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val tag = "webshop_card_${order.id}"
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(c.bgSecondary, RoundedCornerShape(d.radiusLg))
                .border(1.dp, c.border, RoundedCornerShape(d.radiusLg))
                .padding(d.spaceMd)
                .testTag(tag),
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        CardHead(order, context, tag)
        CardMeta(order, tag)
        CardNotices(order, context, tag)
        CardItems(order)
        order.customerNotes?.takeIf { it.isNotBlank() }?.let {
            Text("“$it”", style = PiliTheme.typography.body, color = c.textSecondary)
        }
        CardActions(order, context, callbacks, tag)
        if (context.reprintOpen && WebshopRules.canReprint(order)) PrintRow(order, callbacks, tag)
    }
}

@Composable
private fun CardHead(
    order: WebshopOrderSummary,
    context: CardContext,
    tag: String,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        ChannelChip(order, Modifier.testTag("${tag}_channel"))
        WebshopRules.keyTime(order, context.now)?.let { KeyTimeText(it, Modifier.testTag("${tag}_time")) }
    }
}

/** The channel as a text chip (UI-PD309: no platform logo); the own webshop is told apart by a Volt outline. */
@Composable
private fun ChannelChip(
    order: WebshopOrderSummary,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val own = WebshopRules.isOwnWebshop(order)
    val text = if (own) vueText("webshop_own_label") else order.source
    val outline = if (own) c.accentPrimary else c.border
    Text(
        text = text,
        style = t.label.copy(fontSize = t.sizeMd),
        color = if (own) c.accentPrimary else c.textPrimary,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier =
            modifier
                .widthIn(max = ChipMaxWidth)
                .background(c.bgSurface, RoundedCornerShape(ChipShape))
                .border(ChipBorder, outline, RoundedCornerShape(ChipShape))
                .padding(horizontal = 12.dp, vertical = 6.dp),
    )
}

@Composable
private fun KeyTimeText(
    keyTime: KeyTime,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val (text, color) =
        when (keyTime) {
            is KeyTime.Waiting ->
                vueText("webshop_time_ago", "n" to keyTime.minutes) to if (keyTime.late) c.accentDanger else c.accentWarning
            is KeyTime.ReadyAt -> vueText("webshop_time_ready_at", "time" to keyTime.time) to c.textPrimary
            is KeyTime.Ready -> vueText("webshop_time_ready", "time" to keyTime.time) to c.textPrimary
            KeyTime.InDelivery -> vueText("webshop_status_in_delivery") to c.accentInfo
            is KeyTime.Finished -> vueText(keyTime.labelKey) to c.textSecondary
        }
    Text(text, style = t.label.copy(fontSize = t.sizeMd), color = color, modifier = modifier)
}

@Composable
private fun CardMeta(
    order: WebshopOrderSummary,
    tag: String,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val meta =
        listOfNotNull(
            vueText(WebshopRules.serviceLabelKey(order)),
            order.customerName?.takeIf { it.isNotBlank() },
        ).joinToString(" · ")
    val badge = WebshopRules.paymentBadge(order)
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            meta,
            style = t.body,
            color = c.textSecondary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.testTag("${tag}_meta"),
        )
        if (badge != null) {
            val paid = badge == PaymentBadge.PAID
            Text(
                text = vueText(badge.labelKey),
                style = t.label,
                color = c.textOnAccent,
                modifier =
                    Modifier
                        .background(if (paid) c.accentSuccess else c.accentWarning, RoundedCornerShape(6.dp))
                        .padding(horizontal = 8.dp, vertical = 2.dp)
                        .testTag("${tag}_pay"),
            )
        }
    }
}

@Composable
private fun CardNotices(
    order: WebshopOrderSummary,
    context: CardContext,
    tag: String,
) {
    WebshopRules.cancelledBannerKey(order, context.isManager)?.let { Banner(vueText(it), "${tag}_banner") }
    if (WebshopRules.changedAfterSigning(order)) Banner(vueText("webshop_changed_after_signing"), "${tag}_changed")
    if (WebshopRules.needsReview(order)) ReviewBlock(order, "${tag}_review")
    WebshopRules.deliveryAddress(order)?.let {
        Text(it, style = PiliTheme.typography.body, color = PiliTheme.colors.textPrimary, modifier = Modifier.testTag("${tag}_address"))
    }
}

/** An amber notice about the fiscal state of the order. */
@Composable
private fun Banner(
    text: String,
    tag: String,
) {
    Text(
        text = text,
        style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeSm),
        color = PiliTheme.colors.textOnAccent,
        modifier =
            Modifier
                .fillMaxWidth()
                .background(PiliTheme.colors.accentWarning, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .padding(horizontal = 10.dp, vertical = 8.dp)
                .testTag(tag),
    )
}

/** An order that does not match the menu: never signed, the reasons are listed. */
@Composable
private fun ReviewBlock(
    order: WebshopOrderSummary,
    tag: String,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .border(1.dp, c.accentDanger, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .padding(horizontal = 10.dp, vertical = 8.dp)
                .semantics(mergeDescendants = true) {}
                .testTag(tag),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(vueText("webshop_review_title"), style = t.label, color = c.accentDanger)
        WebshopRules.reviewReasonKeys(order).forEach {
            Text("• " + vueText(it), style = t.body.copy(fontSize = t.sizeSm), color = c.textPrimary)
        }
    }
}

@Composable
private fun CardItems(order: WebshopOrderSummary) {
    val language = LocalUiLanguage.current
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        order.items.forEachIndexed { index, item -> ItemRow(item, language, Modifier.testTag("webshop_card_${order.id}_item_$index")) }
    }
}

@Composable
private fun ItemRow(
    item: WebshopOrderItem,
    language: AppLanguage,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val name = bilingualName(item.productName, item.productName, item.productNameZh, null, language, ProductLabelMode.NAME)
    val zh = language == AppLanguage.ZH
    Column(modifier.fillMaxWidth().semantics(mergeDescendants = true) {}, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            Text("${item.quantity}×", style = t.figure.copy(fontWeight = t.weightBold), color = c.textPrimary)
            ItemNames(name, zh)
        }
        if (item.options.isNotEmpty()) {
            Text(item.options.joinToString(" · "), style = t.body.copy(fontSize = t.sizeSm), color = c.textSecondary)
        }
        item.customerNotes?.takeIf { it.isNotBlank() }?.let {
            Text("“$it”", style = t.body.copy(fontSize = t.sizeSm), color = c.accentWarning)
        }
    }
}

/** The two lines of an item name; the Chinese one gets the Chinese family whichever line it is on. */
@Composable
private fun ItemNames(
    name: BilingualName,
    uiIsZh: Boolean,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column {
        Text(name.primary, style = t.body.copy(fontWeight = t.weightBold).family(uiIsZh), color = c.textPrimary)
        name.secondary?.let { Text(it, style = t.body.copy(fontSize = t.sizeSm).family(!uiIsZh), color = c.textSecondary) }
    }
}

@Composable
private fun TextStyle.family(zh: Boolean): TextStyle = if (zh) copy(fontFamily = PiliTheme.typography.familyZh) else this

@Composable
private fun CardActions(
    order: WebshopOrderSummary,
    context: CardContext,
    callbacks: WebshopCallbacks,
    tag: String,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val enabled = !context.busy
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            formatEuro(order.totalCents),
            style = t.price,
            color = c.textPrimary,
            maxLines = 1,
            modifier = Modifier.testTag("${tag}_total"),
        )
        FlowRow(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm, Alignment.End),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        ) {
            if (WebshopRules.canReject(order)) {
                CardButton(vueText("webshop_action_reject"), { callbacks.onReject(order) }, "${tag}_reject", enabled)
            }
            if (WebshopRules.needsReview(order)) {
                CardButton(vueText("webshop_action_resync"), { callbacks.onResync(order) }, "${tag}_resync", enabled)
            }
            if (WebshopRules.canCollectByCard(order)) {
                CardButton(vueText("webshop_action_collect_card"), { callbacks.onCollectCard(order) }, "${tag}_collect_card", enabled)
            }
            if (WebshopRules.canDelay(order)) {
                CardButton(vueText("webshop_delay_label"), { callbacks.onDelay(order) }, "${tag}_delay", true)
            }
            if (WebshopRules.canReprint(order)) {
                val variant = if (context.reprintOpen) PiliButtonVariant.Secondary else PiliButtonVariant.Ghost
                CardButton(vueText("webshop_action_reprint"), { callbacks.onToggleReprint(order) }, "${tag}_reprint", true, variant)
            }
            WebshopRules.nextAction(order, context.isManager)?.let {
                CardButton(vueText(it.labelKey), { callbacks.onPrimary(order) }, "${tag}_primary", enabled, PiliButtonVariant.Primary)
            }
        }
    }
}

@Composable
private fun CardButton(
    text: String,
    onClick: () -> Unit,
    tag: String,
    enabled: Boolean,
    variant: PiliButtonVariant = PiliButtonVariant.Secondary,
) {
    PiliButton(text = text, onClick = onClick, variant = variant, enabled = enabled, modifier = Modifier.testTag(tag))
}

/** The two tickets a "Reimprimer" tap reveals (2 taps, a rare action). */
@Composable
private fun PrintRow(
    order: WebshopOrderSummary,
    callbacks: WebshopCallbacks,
    tag: String,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm, Alignment.End)) {
        CardButton(vueText("webshop_action_print_kitchen"), {
            callbacks.onPrintKitchen(order)
        }, "${tag}_print_kitchen", true, PiliButtonVariant.Secondary)
        CardButton(vueText("webshop_action_print_customer"), {
            callbacks.onPrintCustomer(order)
        }, "${tag}_print_customer", true, PiliButtonVariant.Secondary)
    }
}
