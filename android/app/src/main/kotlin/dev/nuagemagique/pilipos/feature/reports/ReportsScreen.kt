package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * The reports route (`ReportsView.vue`): builds [ReportsViewModel] from the application graph and draws
 * [ReportsContent] in the staff scaffold, with the dialogs in the scaffold's overlay slot (they stay inside the
 * window, PD214). It does not navigate when it starts and does not redirect without a user (PD205): without an
 * operator it shows an empty state and reads nothing. The root carries the tag `placeholder_reports` (PD204: the
 * tag means "feature root").
 */
@Composable
internal fun ReportsRoute(navigator: AppNavigator) {
    val entry = rememberEntryPoint<ReportsEntryPoint>()
    val model = viewModel { ReportsViewModel(entry) }
    val state by model.state.collectAsState()
    val scope = rememberCoroutineScope()
    val callbacks = remember(model) { reportsCallbacks(model, entry.delivery(), scope) }
    val mailStatus by entry.delivery().mailStatus.collectAsState()
    val request by entry.delivery().request.collectAsState()
    val canPay by entry.session().canTakePayment.collectAsState()
    val delivery = ReportsDelivery(mailStatus, request, canPay)
    // The amounts come from the engine helpers (UI-PD307): the lines of the sale once, the ticked ones on each change.
    val lines = state.refundFlow?.lines.orEmpty()
    val selected = state.refundFlow?.selected.orEmpty()
    val amounts by produceState(emptyMap<Long, LineAmount>(), lines) { value = lineAmounts(entry.helpers(), lines) }
    val preview by produceState(RefundPreview(), selected) { value = refundPreview(entry.helpers(), selected) }
    LaunchedEffect(model, state.signedIn) { if (state.signedIn) model.load() }
    PosScaffold(
        area = HeaderArea.REPORTS,
        navigator = navigator,
        modifier = Modifier.testTag("placeholder_reports"),
        overlay = { ReportsOverlay(state, delivery, RefundUi(amounts, preview), callbacks) },
    ) {
        ReportsContent(state, callbacks)
    }
}

/**
 * The reports screen, stateless: the three tabs (day, cash, tickets) and the body of the chosen one, laid out for
 * 1920 x 1080 dp. Without a user it shows an empty state instead.
 * Test tags: `reports_empty`, `reports_tab_day`, `reports_tab_cash`, `reports_tab_tickets`, `reports_body`.
 */
@Composable
internal fun ReportsContent(
    state: ReportsState,
    callbacks: ReportsCallbacks,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    if (!state.signedIn) {
        Box(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag("reports_empty"), contentAlignment = Alignment.Center) {
            Text(uiText(R.string.ui_reports_signed_out), style = PiliTheme.typography.title, color = PiliTheme.colors.textSecondary)
        }
        return
    }
    Column(
        modifier = modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).padding(horizontal = d.spaceXl, vertical = d.spaceLg),
        verticalArrangement = Arrangement.spacedBy(d.spaceLg),
    ) {
        TabBar(state.tab, callbacks.onTab)
        Box(Modifier.weight(1f).fillMaxWidth().testTag("reports_body")) {
            when (state.tab) {
                ReportsTab.DAY -> DayTab(state.day, callbacks.day)
                ReportsTab.CASH -> CashTab(state.cash, callbacks.cash)
                ReportsTab.TICKETS -> TicketsTab(state.tickets, callbacks.tickets)
            }
        }
    }
}

@Composable
private fun TabBar(
    selected: ReportsTab,
    onTab: (ReportsTab) -> Unit,
) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        ReportsTab.entries.forEach { tab ->
            val key = tabKey(tab)
            ChoiceButton(
                text = vueText(key),
                selected = tab == selected,
                onClick = { onTab(tab) },
                modifier = Modifier.weight(1f).testTag(key),
            )
        }
    }
}

private fun tabKey(tab: ReportsTab): String =
    when (tab) {
        ReportsTab.DAY -> "reports_tab_day"
        ReportsTab.CASH -> "reports_tab_cash"
        ReportsTab.TICKETS -> "reports_tab_tickets"
    }

/**
 * The dialogs of the screen, for the scaffold's overlay slot: the Z history, the two amount keypads, the copy flow
 * and the refund flow. Nothing is drawn while no dialog is open.
 */
@Composable
internal fun BoxScope.ReportsOverlay(
    state: ReportsState,
    delivery: ReportsDelivery,
    refund: RefundUi,
    callbacks: ReportsCallbacks,
) {
    if (!state.signedIn) return
    if (state.day.zListOpen) ZListDialog(state.day.zHistory, callbacks.day)
    CashDialogs(state.cash, callbacks.cash)
    state.copyFlow?.let { CopyDialog(it, delivery, callbacks) }
    state.refundFlow?.let { RefundDialog(it, state.tickets.refundable, delivery, refund, callbacks) }
}
