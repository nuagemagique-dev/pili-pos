package dev.nuagemagique.pilipos.feature.webshop

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.WebshopOrderSummary

private val DotSize = 12.dp
private val ColumnHeadHeight = 56.dp

/** Everything the operator can do on the webshop screen. */
@Suppress("LongParameterList") // One callback per button of the board; a holder class would only rename them.
internal data class WebshopCallbacks(
    val onPrimary: (WebshopOrderSummary) -> Unit = {},
    val onReject: (WebshopOrderSummary) -> Unit = {},
    val onResync: (WebshopOrderSummary) -> Unit = {},
    val onCollectCard: (WebshopOrderSummary) -> Unit = {},
    val onDelay: (WebshopOrderSummary) -> Unit = {},
    val onToggleReprint: (WebshopOrderSummary) -> Unit = {},
    val onPrintKitchen: (WebshopOrderSummary) -> Unit = {},
    val onPrintCustomer: (WebshopOrderSummary) -> Unit = {},
    val onToggleDone: () -> Unit = {},
    val onConfirmReject: () -> Unit = {},
    val onConfirmRefund: () -> Unit = {},
    val onDelayMinutes: (Int) -> Unit = {},
    val onDismissDialog: () -> Unit = {},
)

/**
 * The online orders route (`WebshopOrdersView.vue`): builds [WebshopViewModel] from the application graph and draws
 * [WebshopContent] in the staff scaffold, with the dialogs in its overlay slot (inside the window, PD214). It does
 * not navigate when it starts and does not redirect without a user (PD205). The list is read each time the screen
 * comes to the front. The root carries the tag `placeholder_webshop` (PD204: the tag means "feature root").
 */
@Composable
internal fun WebshopRoute(navigator: AppNavigator) {
    val entry = rememberEntryPoint<WebshopEntryPoint>()
    val model = viewModel { WebshopViewModel(entry) }
    val state by model.state.collectAsState()
    LaunchedEffect(model) { model.runClock() }
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { model.refresh() }
    val callbacks =
        WebshopCallbacks(
            onPrimary = model::primary,
            onReject = model::askReject,
            onResync = model::resync,
            onCollectCard = model::collectCard,
            onDelay = model::askDelay,
            onToggleReprint = model::toggleReprint,
            onPrintKitchen = model::printKitchen,
            onPrintCustomer = model::printCustomer,
            onToggleDone = model::toggleDone,
            onConfirmReject = model::confirmReject,
            onConfirmRefund = model::confirmRefund,
            onDelayMinutes = model::delay,
            onDismissDialog = model::dismissDialog,
        )
    PosScaffold(
        area = HeaderArea.WEBSHOP,
        navigator = navigator,
        modifier = Modifier.testTag("placeholder_webshop"),
        overlay = { WebshopDialogs(state, callbacks) },
    ) {
        WebshopContent(state, callbacks)
    }
}

/**
 * The online orders screen, stateless: the stale alarm, the loading or error line and the board of three columns
 * (new, in the kitchen, ready or finished), laid out for 1920 x 1080 dp. Without a user, and while HubRise is off, it
 * shows an empty state instead. Test tags: `webshop_empty` (no user), `webshop_off`, `webshop_alarm`, `webshop_message`,
 * `webshop_board`, `webshop_col_<stage>` (`new`, `kitchen`, `ready`, `done`), `_count`, `_empty`, `webshop_done_toggle`.
 */
@Composable
internal fun WebshopContent(
    state: WebshopUiState,
    callbacks: WebshopCallbacks,
    modifier: Modifier = Modifier,
) {
    when {
        !state.signedIn -> EmptyState(uiText(R.string.ui_webshop_signed_out), "webshop_empty", modifier)
        !state.hubriseVisible -> EmptyState(uiText(R.string.ui_webshop_hubrise_off), "webshop_off", modifier)
        else -> Board(state, callbacks, modifier)
    }
}

@Composable
private fun EmptyState(
    text: String,
    tag: String,
    modifier: Modifier,
) {
    Box(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag(tag), contentAlignment = Alignment.Center) {
        Text(text, style = PiliTheme.typography.title, color = PiliTheme.colors.textSecondary, textAlign = TextAlign.Center)
    }
}

@Composable
private fun Board(
    state: WebshopUiState,
    callbacks: WebshopCallbacks,
    modifier: Modifier,
) {
    val d = PiliTheme.dimens
    val grouped = WebshopRules.byStage(state.orders)
    val late = state.orders.count { WebshopRules.isLate(it, state.now) }
    Column(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
        if (late > 0) Alarm(vueText("webshop_stale_alarm", "count" to late))
        Message(state)
        Row(
            modifier = Modifier.weight(1f).fillMaxWidth().padding(d.spaceMd).testTag("webshop_board"),
            horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
        ) {
            val third = if (state.showDone) Stage.DONE else Stage.READY
            listOf(Stage.NEW, Stage.KITCHEN, third).forEach { stage ->
                StageColumn(stage, grouped[stage].orEmpty(), grouped[Stage.DONE].orEmpty().size, state, callbacks, Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun Alarm(text: String) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    Text(
        text = text,
        style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeLg, fontWeight = PiliTheme.typography.weightBold),
        color = c.textOnAccent,
        modifier =
            Modifier
                .padding(start = d.spaceMd, end = d.spaceMd, top = d.spaceMd)
                .fillMaxWidth()
                .background(c.accentDanger, RoundedCornerShape(d.radiusSm))
                .padding(horizontal = d.spaceMd, vertical = 12.dp)
                .testTag("webshop_alarm"),
    )
}

/** The loading line (first load) or the last failure. */
@Composable
private fun Message(state: WebshopUiState) {
    val c = PiliTheme.colors
    val text: String
    val color: Color
    when {
        state.loading && state.orders.isEmpty() -> {
            text = vueText("webshop_loading")
            color = c.textSecondary
        }
        state.error != null -> {
            text = state.error
            color = c.accentDanger
        }
        else -> return
    }
    Text(
        text = text,
        style = PiliTheme.typography.body,
        color = color,
        modifier =
            Modifier.padding(
                start = PiliTheme.dimens.spaceMd,
                end = PiliTheme.dimens.spaceMd,
                top = PiliTheme.dimens.spaceMd,
            ).testTag("webshop_message"),
    )
}

@Composable
private fun StageColumn(
    stage: Stage,
    orders: List<WebshopOrderSummary>,
    doneCount: Int,
    state: WebshopUiState,
    callbacks: WebshopCallbacks,
    modifier: Modifier,
) {
    val tag = "webshop_col_" + stage.name.lowercase()
    Column(modifier.testTag(tag), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        ColumnHead(stage, orders.size, doneCount, state.showDone, callbacks.onToggleDone, tag)
        LazyColumn(
            modifier = Modifier.weight(1f).fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm + 4.dp),
        ) {
            if (orders.isEmpty()) {
                item {
                    Text(
                        text = vueText("webshop_empty"),
                        style = PiliTheme.typography.body,
                        color = PiliTheme.colors.textSecondary,
                        modifier = Modifier.padding(PiliTheme.dimens.spaceSm).testTag("${tag}_empty"),
                    )
                }
            }
            items(orders, key = { it.id }) { order ->
                val context = CardContext(state.now, state.isManager, order.id in state.busy, state.reprintOpenId == order.id)
                WebshopCard(order, context, callbacks)
            }
        }
    }
}

@Composable
private fun ColumnHead(
    stage: Stage,
    count: Int,
    doneCount: Int,
    showDone: Boolean,
    onToggleDone: () -> Unit,
    tag: String,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val (titleKey, dot) =
        when (stage) {
            Stage.NEW -> "webshop_col_new" to c.accentWarning
            Stage.KITCHEN -> "webshop_col_kitchen" to c.accentInfo
            Stage.READY -> "webshop_col_ready" to c.accentSuccess
            Stage.DONE -> "webshop_filter_done" to c.textSecondary
        }
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp).height(ColumnHeadHeight),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(DotSize).background(dot, CircleShape))
        Text(vueText(titleKey), style = t.title.copy(fontSize = t.sizeLg), color = c.textPrimary)
        Text(count.toString(), style = t.figure, color = c.textSecondary, modifier = Modifier.testTag("${tag}_count"))
        Box(Modifier.weight(1f))
        if (stage == Stage.READY || stage == Stage.DONE) DoneToggle(doneCount, showDone, onToggleDone)
    }
}

/** The "Terminees" button of the third column head: shows the finished orders in place of the ready ones. */
@Composable
private fun DoneToggle(
    doneCount: Int,
    active: Boolean,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Text(
        text = vueText("webshop_filter_done") + " · " + doneCount,
        style = t.label,
        color = if (active) c.selectedFg else c.textPrimary,
        modifier =
            Modifier
                .background(if (active) c.selectedBg else c.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                .clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = 14.dp, vertical = 10.dp)
                .testTag("webshop_done_toggle"),
    )
}
