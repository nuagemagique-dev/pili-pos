@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.tables

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.feature.order.OrderRoutes
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.TableInfo
import uniffi.pos_ffi.TakeawayInfo

/** Tables per row of the grid (`repeat(5, ...)` in `TablesView.vue`). */
private const val GRID_COLUMNS = 5

/** Rows the grid divides the height into (`repeat(4, ...)`); more tables scroll. */
private const val GRID_ROWS = 4

/** The `status` the engine gives a table with an open order. */
private const val STATUS_OCCUPIED = "occupied"

private const val DASH_ON = 10f
private const val DASH_OFF = 7f

private val BandHeight = 6.dp
private val TakeawayBand = 4.dp
private val TakeawayMinHeight = 80.dp
private val NewTakeawayHeight = 80.dp
private val DashStroke = 2.dp

/** Everything the operator can do on the tables screen. */
internal data class TablesCallbacks(
    val onTable: (TableInfo) -> Unit = {},
    val onTakeaway: (TakeawayInfo) -> Unit = {},
    val onNewTakeaway: () -> Unit = {},
)

/**
 * The tables route (`TablesView.vue`): builds [TablesViewModel] from the application graph and draws
 * [TablesContent] in the staff scaffold. It does not navigate when it starts and does not redirect without a user
 * (PD205); it leaves for the order screen only after the operator opened an order. The lists are read again each
 * time the screen comes back to the front. The root carries the tag `placeholder_tables` (PD204: the tag means
 * "feature root").
 */
@Composable
internal fun TablesRoute(navigator: AppNavigator) {
    val entry = rememberEntryPoint<TablesEntryPoint>()
    val model = viewModel { TablesViewModel(entry) }
    val state by model.state.collectAsState()
    LaunchedEffect(model) {
        model.effects.collect {
            when (it) {
                is TablesEffect.OpenOrder -> navigator.navigate(OrderRoutes.order(it.orderId))
            }
        }
    }
    LaunchedEffect(model) { model.runClock() }
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { model.refresh() }
    val callbacks =
        TablesCallbacks(
            onTable = model::selectTable,
            onTakeaway = model::resumeTakeaway,
            onNewTakeaway = model::newTakeaway,
        )
    PosScaffold(HeaderArea.TABLES, navigator, modifier = Modifier.testTag("placeholder_tables")) {
        TablesContent(state, callbacks)
    }
}

/**
 * The tables screen, stateless: the table grid (5 columns, occupied tables filled with their total, free ones
 * dashed) and the open takeaways on the right with the new takeaway button, laid out for 1920 x 1080 dp. Without a
 * user it shows an empty state instead. Test tags: `tables_grid`, `table_<number>`, `table_<number>_total`,
 * `tables_none`, `takeaways_panel`, `takeaway_<orderId>`, `takeaway_new`, `tables_empty`.
 */
@Composable
internal fun TablesContent(
    state: TablesUiState,
    callbacks: TablesCallbacks,
    modifier: Modifier = Modifier,
) {
    if (!state.signedIn) {
        Box(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).testTag("tables_empty"), contentAlignment = Alignment.Center) {
            Text(uiText(R.string.ui_tables_signed_out), style = PiliTheme.typography.title, color = PiliTheme.colors.textSecondary)
        }
        return
    }
    Row(modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) {
        TableGrid(state, callbacks.onTable, Modifier.weight(1f).fillMaxHeight())
        TakeawayPanel(state, callbacks, Modifier.width(PiliTheme.dimens.cartWidth).fillMaxHeight())
    }
}

@Composable
private fun TableGrid(
    state: TablesUiState,
    onTable: (TableInfo) -> Unit,
    modifier: Modifier,
) {
    val d = PiliTheme.dimens
    BoxWithConstraints(modifier.testTag("tables_grid")) {
        val cardHeight = (maxHeight - d.spaceLg * 2 - d.spaceMd * (GRID_ROWS - 1)) / GRID_ROWS
        Column(
            modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(d.spaceLg),
            verticalArrangement = Arrangement.spacedBy(d.spaceMd),
        ) {
            state.tables.chunked(GRID_COLUMNS).forEach { row ->
                Row(Modifier.fillMaxWidth().height(cardHeight), horizontalArrangement = Arrangement.spacedBy(d.spaceMd)) {
                    row.forEach { table -> TableCard(table, state, { onTable(table) }, Modifier.weight(1f).fillMaxHeight()) }
                    repeat(GRID_COLUMNS - row.size) { Spacer(Modifier.weight(1f)) }
                }
            }
        }
        if (state.loaded && state.tables.isEmpty()) {
            Text(
                uiText(R.string.ui_tables_none),
                style = PiliTheme.typography.title,
                color = PiliTheme.colors.textSecondary,
                modifier = Modifier.align(Alignment.Center).testTag("tables_none"),
            )
        }
    }
}

/** One table (UX guide section 5.3): an occupied one is filled with a band on top and its total in bold, a free one is dashed. */
@Composable
private fun TableCard(
    table: TableInfo,
    state: TablesUiState,
    onClick: () -> Unit,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val shape = RoundedCornerShape(d.radiusLg)
    val occupied = table.status == STATUS_OCCUPIED
    val source = remember { MutableInteractionSource() }
    val base = modifier.tapScale(source).clip(shape)
    val look =
        if (occupied) {
            base.background(c.bgSecondary).border(1.dp, c.border, shape)
        } else {
            base.drawBehind { dashedOutline(c.border, d.radiusLg.toPx()) }
        }
    Box(
        modifier =
            look
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .testTag("table_${table.tableNumber}"),
    ) {
        if (occupied) Box(Modifier.fillMaxWidth().height(BandHeight).background(c.textPrimary))
        Column(
            modifier = Modifier.fillMaxSize().padding(horizontal = d.spaceMd + d.spaceXs, vertical = 18.dp),
            verticalArrangement = Arrangement.SpaceBetween,
        ) {
            TableTop(table, state.cardCharged)
            if (occupied) OccupiedInfo(table, state) else FreeLabel()
        }
    }
}

/** The 2 dp dashed outline of a free table. */
private fun DrawScope.dashedOutline(
    color: Color,
    radius: Float,
) {
    val width = DashStroke.toPx()
    drawRoundRect(
        color = color,
        topLeft = Offset(width / 2f, width / 2f),
        size = Size(size.width - width, size.height - width),
        cornerRadius = CornerRadius(radius),
        style = Stroke(width = width, pathEffect = PathEffect.dashPathEffect(floatArrayOf(DASH_ON * width, DASH_OFF * width))),
    )
}

@Composable
private fun TableTop(
    table: TableInfo,
    cardCharged: Set<Long>,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val occupied = table.status == STATUS_OCCUPIED
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.Top) {
        Text(
            text = table.tableNumber.toString(),
            style = t.figure.copy(fontSize = t.sizeXl * TABLE_NUMBER_SCALE, fontWeight = t.weightBold),
            color = if (occupied) c.textPrimary else c.textSecondary,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            // A provisional bill is on the table (sign_pre_bill, P).
            if (table.preBillPrinted) {
                Tag(vueText("pre_bill"), Modifier.testTag("table_${table.tableNumber}_bill"))
            }
            // The customer's card was charged and no sale records it yet.
            if (table.orderId?.let { it in cardCharged } == true) {
                Tag(vueText("card_charged_badge"), Modifier.testTag("table_${table.tableNumber}_card"))
            }
        }
    }
}

/** A small amber tag on a table or a takeaway (`bill-tag` in `TablesView.vue`). */
@Composable
private fun Tag(
    text: String,
    modifier: Modifier = Modifier,
) {
    val t = PiliTheme.typography
    Text(
        text = text,
        style = t.body.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
        color = PiliTheme.colors.textOnAccent,
        modifier =
            modifier.background(
                PiliTheme.colors.accentWarning,
                RoundedCornerShape(6.dp),
            ).padding(horizontal = 8.dp, vertical = 2.dp),
    )
}

@Composable
private fun OccupiedInfo(
    table: TableInfo,
    state: TablesUiState,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val items = table.lineCount.toString() + " " + vueText("items_short")
    val meta =
        table.openedAt?.let {
            val minutes = PosDateTime.minutesSince(it, state.now)
            items + " · " + uiText(R.string.ui_tables_minutes, "minutes" to minutes)
        } ?: items
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(
            text = formatEuro(table.totalCents),
            style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
            color = c.textPrimary,
            maxLines = 1,
            modifier = Modifier.testTag("table_${table.tableNumber}_total"),
        )
        Text(
            text = meta,
            style = t.figure.copy(fontSize = t.sizeMd),
            color = c.textSecondary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.testTag("table_${table.tableNumber}_meta"),
        )
    }
}

@Composable
private fun FreeLabel() {
    Text(vueText("free"), style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
}

/** The open takeaways where the cart usually is, with the main action at the bottom (UX guide section 2.3). */
@Composable
private fun TakeawayPanel(
    state: TablesUiState,
    callbacks: TablesCallbacks,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Column(
        modifier =
            modifier
                .background(c.bgSecondary)
                .drawBehind { drawRect(c.border, size = Size(1.dp.toPx(), size.height)) }
                .testTag("takeaways_panel"),
    ) {
        Row(
            modifier =
                Modifier.fillMaxWidth().drawBottomLine(
                    c.border,
                ).padding(start = d.spaceMd, end = d.spaceMd, top = d.spaceMd, bottom = 12.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.Bottom,
        ) {
            Text(vueText("takeaways_open"), style = t.title, color = c.textPrimary)
            Text(
                text = state.takeaways.size.toString(),
                style = t.figure,
                color = c.textSecondary,
                modifier = Modifier.testTag("takeaways_count"),
            )
        }
        Column(
            modifier =
                Modifier.weight(
                    1f,
                ).fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = d.spaceMd, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(d.spaceSm),
        ) {
            state.takeaways.forEach { TakeawayCard(it, it.orderId in state.cardCharged, { callbacks.onTakeaway(it) }) }
        }
        Box(Modifier.fillMaxWidth().drawTopLine(c.border).padding(d.spaceMd)) {
            PiliButton(
                onClick = callbacks.onNewTakeaway,
                modifier = Modifier.fillMaxWidth().height(NewTakeawayHeight).testTag("takeaway_new"),
                variant = PiliButtonVariant.Primary,
            ) {
                Text(
                    text = vueText("takeaway_start"),
                    style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightHeavy),
                    color = c.textOnAccent,
                    textAlign = TextAlign.Center,
                )
            }
        }
    }
}

/** An amber card: a takeaway still waiting for its payment. */
@Composable
private fun TakeawayCard(
    takeaway: TakeawayInfo,
    cardCharged: Boolean,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val time = PosDateTime.formatTimestamp(takeaway.createdAt, PosDateTime::formatTime)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .tapScale(source)
                .heightIn(min = TakeawayMinHeight)
                .clip(RoundedCornerShape(d.radiusMd))
                .background(c.bgSurface)
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .testTag("takeaway_${takeaway.orderId}"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(TakeawayBand).fillMaxHeight().background(c.accentWarning))
        Row(
            modifier = Modifier.weight(1f).padding(horizontal = 20.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(
                    text = vueText("takeaway_started_at", "time" to time),
                    style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
                    color = c.textPrimary,
                )
                Text(
                    text = takeaway.lineCount.toString() + " " + vueText("items_short"),
                    style = t.figure.copy(fontSize = t.sizeMd),
                    color = c.textSecondary,
                )
                if (cardCharged) {
                    Tag(vueText("card_charged_badge"), Modifier.testTag("takeaway_${takeaway.orderId}_card"))
                }
            }
            Text(
                text = formatEuro(takeaway.totalCents),
                style = t.figure.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
                color = c.textPrimary,
                maxLines = 1,
            )
        }
    }
}

private fun Modifier.drawBottomLine(color: Color): Modifier =
    drawBehind { drawRect(color, topLeft = Offset(0f, size.height - 1.dp.toPx()), size = Size(size.width, 1.dp.toPx())) }

private fun Modifier.drawTopLine(color: Color): Modifier = drawBehind { drawRect(color, size = Size(size.width, 1.dp.toPx())) }

/** The table number is 44 px in Vue, the `sizeXl` token (28) times this. */
private const val TABLE_NUMBER_SCALE = 1.5f
