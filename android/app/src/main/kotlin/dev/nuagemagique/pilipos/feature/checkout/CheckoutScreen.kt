package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.order.OrderRoutes
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.DeliveryOutcomePanel
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.TicketMediumKeypad
import dev.nuagemagique.pilipos.ui.components.TicketMediumPickerState
import dev.nuagemagique.pilipos.ui.components.defaultDeliveryOutcomeTexts
import dev.nuagemagique.pilipos.ui.components.defaultEmailKeypadTexts
import dev.nuagemagique.pilipos.ui.components.rememberTicketMediumPickerState
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.PaymentMethod

private val DoneBadge = 96.dp
private val DoneCardMaxWidth = 720.dp
private val DoneButtonHeight = 72.dp
private val SpinnerSize = 64.dp

/** Everything the operator can do on the checkout screen. */
internal data class CheckoutCallbacks(
    val onMethod: (PaymentMethod) -> Unit = {},
    val onSplit: (Int) -> Unit = {},
    val onKey: (NumPadKey) -> Unit = {},
    val onExact: () -> Unit = {},
    val onQuick: (Long) -> Unit = {},
    val onClearParts: () -> Unit = {},
    val onBack: () -> Unit = {},
    val onValidate: () -> Unit = {},
    val onFinish: () -> Unit = {},
    val onPaper: () -> Unit = {},
    val onPrintMissing: suspend (fiscalEventId: Long) -> Boolean = { false },
)

/**
 * The checkout route (`CheckoutView.vue`): builds [CheckoutViewModel] from the application graph and draws
 * [CheckoutContent] in the staff scaffold. It does not navigate when it starts and does not redirect without an
 * order (PD205): with no matching order it asks the order store for the route's order once the engine is ready,
 * and shows an empty state meanwhile. It leaves only after the operator pressed back or finish. The root carries
 * the tag `placeholder_checkout` (PD204: the tag means "feature root"). Dialogs (the e-mail keypad) are drawn in
 * the scaffold's overlay, inside the window (PD214).
 */
@Composable
internal fun CheckoutRoute(
    navigator: AppNavigator,
    orderId: Long,
) {
    val entry = rememberEntryPoint<CheckoutEntryPoint>()
    val model = viewModel(key = "checkout_$orderId") { CheckoutViewModel(entry, orderId) }
    val state by model.state.collectAsState()
    val picker = rememberTicketMediumPickerState()
    LaunchedEffect(model) {
        model.effects.collect {
            when (it) {
                is CheckoutEffect.BackToOrder -> navigator.navigate(OrderRoutes.order(it.orderId))
                CheckoutEffect.NewOrder -> navigator.navigate(TablesRoutes.TABLES)
            }
        }
    }
    val callbacks =
        CheckoutCallbacks(
            onMethod = model::selectMethod,
            onSplit = model::selectSplit,
            onKey = model::pressKey,
            onExact = model::setExact,
            onQuick = model::setQuickAmount,
            onClearParts = model::clearParts,
            onBack = model::back,
            onValidate = model::validate,
            onFinish = model::finish,
            onPaper = model::chooseDeliveryPaper,
            onPrintMissing = model::printMissingOriginal,
        )
    PosScaffold(
        area = HeaderArea.CHECKOUT,
        navigator = navigator,
        modifier = Modifier.testTag("placeholder_checkout"),
        overlay = {
            TicketMediumKeypad(
                state = picker,
                texts = defaultEmailKeypadTexts(),
                onEmail = model::chooseDeliveryEmail,
                onPaper = model::chooseDeliveryPaper,
            )
        },
    ) {
        CheckoutContent(state, callbacks, picker)
    }
}

/**
 * The checkout screen, stateless, laid out for 1920 x 1080 dp: the payment panel (methods, split, entry, ticket
 * medium) on the left and the recap of the order with the total, the parts taken, what is left and the validate
 * button on the right, as `CheckoutView.vue`. The processing step shows a spinner and the done step the sale
 * summary with the delivery outcome. Without an order in the pay step it shows an empty state. Test tags:
 * `checkout_empty`, `checkout_pay`, `checkout_processing`, `checkout_done`, and those of the panels.
 */
@Composable
internal fun CheckoutContent(
    state: CheckoutUiState,
    callbacks: CheckoutCallbacks,
    picker: TicketMediumPickerState = rememberTicketMediumPickerState(),
    modifier: Modifier = Modifier,
) {
    when {
        state.step == CheckoutStep.PROCESSING -> ProcessingStep(modifier)
        state.step == CheckoutStep.DONE && state.done != null -> DoneStep(state, state.done, callbacks, modifier)
        !state.hasOrder -> EmptyStep(modifier)
        else ->
            Row(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag("checkout_pay")) {
                PayPanel(state, callbacks, picker, Modifier.weight(1f).fillMaxHeight())
                SummaryPanel(state, callbacks, Modifier.width(PiliTheme.dimens.cartWidth).fillMaxHeight())
            }
    }
}

@Composable
private fun EmptyStep(modifier: Modifier) {
    Box(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag("checkout_empty"), contentAlignment = Alignment.Center) {
        Text(
            text = uiText(R.string.ui_checkout_no_order),
            style = PiliTheme.typography.title,
            color = PiliTheme.colors.textSecondary,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable
private fun ProcessingStep(modifier: Modifier) {
    val d = PiliTheme.dimens
    Box(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag("checkout_processing"), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(d.spaceMd)) {
            CircularProgressIndicator(color = PiliTheme.colors.accentPrimary, modifier = Modifier.size(SpinnerSize))
            Text(vueText("processing_fdm"), style = PiliTheme.typography.title, color = PiliTheme.colors.textPrimary)
        }
    }
}

/** The done step: what was sold, the change, the FDM reference, how the ticket reached the customer, and finish. */
@Composable
private fun DoneStep(
    state: CheckoutUiState,
    done: CheckoutDone,
    callbacks: CheckoutCallbacks,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Box(modifier.fillMaxSize().background(c.bgPrimary).testTag("checkout_done"), contentAlignment = Alignment.Center) {
        Column(
            modifier = Modifier.widthIn(max = DoneCardMaxWidth).fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(d.spaceMd),
        ) {
            Box(Modifier.size(DoneBadge).background(c.accentSuccess, CircleShape), contentAlignment = Alignment.Center) {
                Text(
                    text = uiText(R.string.ui_checkout_done_ok),
                    style = t.title,
                    color = c.textOnAccent,
                    modifier = Modifier.testTag("checkout_done_ok"),
                )
            }
            Receipt(done)
            if (!done.fdmConnected) {
                Text(
                    text = vueText("fdm_no_sig"),
                    style = t.body.copy(fontWeight = t.weightBold),
                    color = c.accentWarning,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.testTag("checkout_fdm_warning"),
                )
            }
            DeliveryOutcomePanel(
                outcome = done.delivery,
                texts = defaultDeliveryOutcomeTexts(done.delivery),
                canPrintMissing = state.canTakePayment,
                onPrintMissing = callbacks.onPrintMissing,
                fiscalEventId = done.fiscalEventId,
            )
            PiliButton(
                text = vueText("finish"),
                onClick = callbacks.onFinish,
                modifier = Modifier.fillMaxWidth().height(DoneButtonHeight).testTag("checkout_finish"),
            )
        }
    }
}

/** The mini receipt of the done step (`receipt-mini`). */
@Composable
private fun Receipt(done: CheckoutDone) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(c.bgSecondary, RoundedCornerShape(d.radiusLg))
                .border(1.dp, c.border, RoundedCornerShape(d.radiusLg))
                .padding(d.spaceLg)
                .testTag("checkout_receipt"),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(d.spaceSm),
    ) {
        done.ticketLabel?.let {
            Text(it, style = t.body, color = c.textSecondary, modifier = Modifier.testTag("checkout_receipt_label"))
        }
        Text(
            text = formatEuro(done.totalCents),
            style = t.price.copy(fontSize = t.sizeXl),
            color = c.textPrimary,
            modifier = Modifier.testTag("checkout_done_total"),
        )
        if (done.changeCents > 0L) {
            Row(horizontalArrangement = Arrangement.spacedBy(d.spaceMd), verticalAlignment = Alignment.CenterVertically) {
                Text(vueText("change"), style = t.body, color = c.textSecondary)
                Text(
                    text = formatEuro(done.changeCents),
                    style = t.price.copy(fontSize = t.sizeLg),
                    color = c.accentSuccess,
                    modifier = Modifier.testTag("checkout_done_change"),
                )
            }
        }
        done.fdmId?.let {
            Text(
                text = uiText(R.string.ui_checkout_fdm_ref, "id" to it),
                style = t.figure.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
                modifier = Modifier.testTag("checkout_done_fdm"),
            )
        }
    }
}
