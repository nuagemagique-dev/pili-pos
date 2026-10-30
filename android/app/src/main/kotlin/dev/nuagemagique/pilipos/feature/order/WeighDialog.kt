package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.format.formatWeight
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.ScaleWeighInput

/** As wide as the stepper it replaces. */
private val WeightBadgeWidth = 96.dp

/**
 * The weighing in progress (`WeighDialog.vue`): the product, where the weighing stands, and the only way out. The
 * engine holds the weighing, so a tap outside does nothing. Test tags: `weigh_dialog`, `weigh_stage`, `weigh_cancel`.
 */
@Composable
internal fun WeighDialog(
    dialog: OrderDialog.Weighing,
    onCancel: () -> Unit,
) {
    PiliDialog(
        title = vueText("scale_title"),
        onDismissRequest = null,
        modifier = Modifier.testTag("weigh_dialog"),
        actions = {
            PiliButton(
                text = vueText("back"),
                onClick = onCancel,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("weigh_cancel"),
            )
        },
    ) {
        Text(dialog.item.name, style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
        Text(
            vueText(stageKey(dialog.stage)),
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = PiliTheme.colors.accentPrimary,
            modifier = Modifier.fillMaxWidth().testTag("weigh_stage"),
        )
    }
}

private fun stageKey(stage: String): String =
    when (stage) {
        "stabilizing" -> "scale_stage_stabilizing"
        "checking" -> "scale_stage_checking"
        else -> "scale_stage_waiting"
    }

/** The weight of a weighed cart line, where another line has its stepper (test tag `cart_line_<id>_weight`). */
@Composable
internal fun WeightBadge(
    id: Long,
    grams: Long,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Text(
        text = formatWeight(grams),
        style = t.figure.copy(fontWeight = t.weightBold),
        color = c.textPrimary,
        maxLines = 1,
        textAlign = TextAlign.Center,
        modifier =
            Modifier
                .width(WeightBadgeWidth)
                .background(c.bgSurface, RoundedCornerShape(d.radiusMd))
                .padding(vertical = 12.dp)
                .testTag("cart_line_${id}_weight"),
    )
}

/**
 * A tap on a product sold by weight (`weigh` of `OrderView.vue`): the dialog stays up until the engine has the line or a
 * refusal. The store's lock is held during the weighing, so the taps behind it wait; the dialog covers the menu anyway.
 * Without a scale the product stays off sale. [dialog] changes the open dialog of the order screen.
 */
internal class WeighFlow(
    private val entry: OrderEntryPoint,
    private val scope: CoroutineScope,
    private val dialog: ((OrderDialog) -> OrderDialog) -> Unit,
) {
    private var running = false

    fun start(item: MenuItem) {
        val language = entry.settings().language.value
        if (entry.settings().settings.value?.scaleConfigured != true) {
            entry.toasts().show(entry.catalog().text("err_scale_not_configured", language), ToastVariant.Error)
            return
        }
        if (running) return
        running = true
        dialog { OrderDialog.Weighing(item) }
        scope.launch {
            entry.order().addLineWith(item, 1) { orderId ->
                entry.gateway().call { engine -> engine.scaleStartWeighing(ScaleWeighInput(orderId, item.id)) }
            }
            running = false
            dialog { if (it is OrderDialog.Weighing) OrderDialog.None else it }
        }
    }

    /** Where the weighing stands, from the `scale-progress` event. */
    fun progress(stage: String) {
        dialog { if (it is OrderDialog.Weighing) it.copy(stage = stage) else it }
    }

    /** The engine ends the weighing and its refusal closes the dialog. */
    @Suppress("TooGenericExceptionCaught")
    fun cancel() {
        scope.launch {
            try {
                entry.gateway().call { engine -> engine.scaleCancelWeighing() }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                entry.errors().show(e, entry.settings().language.value)
            }
        }
    }
}
