package dev.nuagemagique.pilipos.feature.tables

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.TableMoves
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.TableInfo
import uniffi.pos_ffi.TakeawayInfo
import java.time.Clock
import java.time.Instant
import java.util.concurrent.atomic.AtomicBoolean

/** How often the minutes since opening are refreshed, in milliseconds (30 s in `TablesView.vue`). */
internal const val CLOCK_TICK_MS: Long = 30_000L

/** How long the error toast of a refused move stays, in milliseconds (6 s in `TablesView.vue`). */
private const val ERROR_TTL_MS: Long = 6_000L

/** What the tables screen shows. */
internal data class TablesUiState(
    /** The tables as the engine listed them, in its order. */
    val tables: List<TableInfo> = emptyList(),
    /** The takeaways left unpaid, each of which must still end in a sale (FAQ Q205). */
    val takeaways: List<TakeawayInfo> = emptyList(),
    /** The time the minutes since opening are counted to. */
    val now: Instant = Instant.EPOCH,
    /** False when nobody is logged in: the screen shows an empty state and does not redirect (PD205). */
    val signedIn: Boolean = false,
    /** True once the lists were read for the engine that is running. */
    val loaded: Boolean = false,
    /** The orders whose card was charged and not yet settled by a sale (`list_unsettled_card_payments`). */
    val cardCharged: Set<Long> = emptySet(),
    /** The "Déplacer" mode (`moving`, `source` of `TablesView.vue`). */
    val move: MoveState = MoveState(),
    /** The action sheet of the occupied table that was long pressed (UX audit T7), null when closed. */
    val sheet: TableSheet? = null,
)

/**
 * The action sheet of a long pressed table: "Ouvrir", "Addition", "Déplacer", "Payer". The bill and the payment are
 * for the roles that take payments ([canTakePayment]), as the engine checks (`Access::Cashier`).
 */
internal data class TableSheet(
    val table: TableInfo,
    val canTakePayment: Boolean,
)

/** The "Déplacer" mode (UX audit C8): tap the occupied table, then a free one; the move is signed at once. */
internal data class MoveState(
    val moving: Boolean = false,
    val source: TableInfo? = null,
    /** The move is being signed. */
    val transferring: Boolean = false,
) {
    /** In the mode only an occupied table (the source) or, once it is chosen, a free one (the target) can be tapped. */
    fun selectable(table: TableInfo): Boolean =
        if (source == null) table.status == STATUS_OCCUPIED && table.orderId != null else table.status == STATUS_FREE
}

private const val STATUS_OCCUPIED = "occupied"
private const val STATUS_FREE = "free"

/** What the view model asks of the navigation: the order screen of the order that was just opened. */
internal sealed interface TablesEffect {
    /** Opens the order screen of order [orderId]. */
    data class OpenOrder(
        val orderId: Long,
    ) : TablesEffect

    /** Opens the checkout of order [orderId] (the "Payer" of the action sheet, UX audit T7). */
    data class OpenCheckout(
        val orderId: Long,
    ) : TablesEffect
}

/**
 * State holder of the tables screen, a port of the script of `ui/src/views/TablesView.vue`.
 *
 * The engine owns the tables and the orders: this lists them (`list_tables`, `list_open_takeaways`) when the engine
 * is ready, when the screen comes back to the front and on a resync, and opens an order through [OrderStore]. An
 * occupied table loads its order (and its lines), a free one starts one, a takeaway resumes or starts. A refusal
 * is shown as an error toast. It never navigates by itself: [TablesEffect.OpenOrder] is sent after an action of the
 * operator succeeded (PD205).
 */
@Suppress("TooManyFunctions") // One entry per tap of the screen, the move mode included.
internal class TablesViewModel(
    private val entry: TablesEntryPoint,
    private val clock: Clock = Clock.systemUTC(),
) : ViewModel() {
    private val order = entry.order()
    private val session = entry.session()
    private val gateway = entry.gateway()
    private val tableMoves = TableMoves(order, entry.toasts(), entry.catalog(), entry.settings(), entry.errors())
    private val own = MutableStateFlow(Lists())
    private val now = MutableStateFlow(clock.instant())
    private val move = MutableStateFlow(MoveState())
    private val sheet = MutableStateFlow<TableInfo?>(null)
    private val effectChannel = Channel<TablesEffect>(Channel.BUFFERED)

    // One action at a time: a second tap while the first still talks to the engine is ignored.
    private val busy = AtomicBoolean(false)

    private data class Lists(
        val tables: List<TableInfo> = emptyList(),
        val takeaways: List<TakeawayInfo> = emptyList(),
        val cardCharged: Set<Long> = emptySet(),
        val loaded: Boolean = false,
    )

    private val sheetState = combine(sheet, session.canTakePayment) { table, pay -> table?.let { TableSheet(it, pay) } }

    /** What the screen draws. */
    val state: StateFlow<TablesUiState> =
        combine(own, now, session.currentUser, move, sheetState) { lists, instant, user, mode, open ->
            build(lists, instant, user != null, mode, open)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            build(own.value, now.value, session.currentUser.value != null, move.value, null),
        )

    /** One-shot moves, each delivered once. */
    val effects: Flow<TablesEffect> = effectChannel.receiveAsFlow()

    init {
        viewModelScope.launch {
            gateway.ready.collect { ready -> if (ready) load() else own.value = Lists() }
        }
        viewModelScope.launch {
            entry.events().events.collect { event -> if (event is PosEvent.Resync) load() }
        }
        // A move or its undo was signed (the undo can come long after the screen was left and entered again).
        viewModelScope.launch { order.tableMoves.drop(1).collect { load() } }
    }

    /**
     * Moves the clock of the minutes-since labels every [CLOCK_TICK_MS] until cancelled. The screen runs it while it
     * is composed (`onMounted` and `onUnmounted` of `TablesView.vue`), so nothing ticks on a screen that is gone.
     */
    suspend fun runClock() {
        while (true) {
            delay(CLOCK_TICK_MS)
            now.value = clock.instant()
        }
    }

    private fun build(
        lists: Lists,
        instant: Instant,
        signedIn: Boolean,
        mode: MoveState,
        open: TableSheet?,
    ) = TablesUiState(lists.tables, lists.takeaways, instant, signedIn, lists.loaded, lists.cardCharged, mode, open)

    // ── Loading ───────────────────────────────────────────────────

    /** Reads the lists again in the background: the screen came back to the front. */
    fun refresh() {
        viewModelScope.launch { load() }
    }

    /** Reads the tables and the open takeaways again (the screen came back to the front, or the engine resynced). */
    suspend fun load() {
        if (session.currentUser.value == null) return
        now.value = clock.instant()
        attempt {
            gateway.call { engine -> engine.listTables() to engine.listOpenTakeaways() }
        }.fold(
            onSuccess = { (tables, takeaways) ->
                // Only a badge: no answer means no badge.
                val charged = attempt { gateway.call { engine -> engine.listUnsettledCardPayments() } }.getOrDefault(emptyList())
                // The tables say their own zones: the other screens write "A-19" from them (UX audit T1).
                order.learnTables(tables)
                own.value = Lists(tables, takeaways, charged.map { it.orderId }.toSet(), loaded = true)
            },
            // The lists stay as they were: the next resume or resync reads them again.
            onFailure = { Timber.d(it, "Tables not loaded") },
        )
    }

    // ── Opening an order ──────────────────────────────────────────

    /** A tap on [table]: an occupied one loads its order and its lines, a free one starts an order (`selectTable`). */
    fun selectTable(table: TableInfo) {
        if (move.value.moving) {
            pickInMoveMode(table)
            return
        }
        val orderId = table.orderId
        open { insz -> if (orderId != null) order.loadOrder(orderId) else order.startOrder(table.tableNumber, insz) }
    }

    // ── The action sheet of a long pressed table (UX audit T7) ────

    /**
     * A long press (500 ms) on [table]: opens its action sheet. Only an occupied table has one, and not in the
     * "Déplacer" mode, where a press picks tables.
     */
    fun openSheet(table: TableInfo) {
        if (move.value.moving || table.status != STATUS_OCCUPIED || table.orderId == null) return
        sheet.value = table
    }

    /** Closes the sheet without doing anything. */
    fun closeSheet() {
        sheet.value = null
    }

    /** "Ouvrir": the usual tap on the table. */
    fun sheetOpen() {
        val table = takeSheet() ?: return
        selectTable(table)
    }

    /**
     * "Addition": the provisional bill of that table's order, the command of the cart's "Addition" button. The
     * screen stays on the plan; the table then shows its "Addition" tag.
     */
    fun sheetBill() {
        val orderId = takeSheet()?.orderId ?: return
        if (!busy.compareAndSet(false, true)) return
        viewModelScope.launch {
            try {
                val language = entry.settings().language.value
                attempt { order.preBillOrder(orderId) }.fold(
                    onSuccess = { entry.toasts().show(entry.catalog().text("pre_bill_printed", language), ToastVariant.Success) },
                    onFailure = { entry.errors().show(it, language, entry.catalog().text("pre_bill_failed", language), ERROR_TTL_MS) },
                )
                load()
            } finally {
                busy.set(false)
            }
        }
    }

    /** "Déplacer": the wave 2 move mode with this table already chosen; one tap on a free table moves it. */
    fun sheetMove() {
        val table = takeSheet() ?: return
        move.value = MoveState(moving = true, source = table)
    }

    /** "Payer": loads that table's order and opens its checkout. */
    fun sheetPay() {
        val orderId = takeSheet()?.orderId ?: return
        open({ TablesEffect.OpenCheckout(orderId) }) { order.loadOrder(orderId) }
    }

    private fun takeSheet(): TableInfo? = sheet.value.also { sheet.value = null }

    // ── Moving a table ────────────────────────────────────────────

    /** The "Déplacer une table" toggle (`toggleMove`): enters or leaves the mode, with nothing chosen. */
    fun toggleMove() {
        move.value = MoveState(moving = !move.value.moving)
    }

    private fun pickInMoveMode(table: TableInfo) {
        val mode = move.value
        if (!mode.selectable(table) || mode.transferring) return
        val source = mode.source
        if (source == null) {
            move.value = mode.copy(source = table)
            return
        }
        moveTo(source, table.tableNumber)
    }

    /**
     * The free table tapped as the target (`pickInMoveMode` of `TablesView.vue`): the engine signs the move at once
     * and the toast offers "Annuler"; a refusal shows the error and the tables are read again. The mode is left
     * either way.
     */
    private fun moveTo(
        source: TableInfo,
        to: Int,
    ) {
        val orderId = source.orderId ?: return
        move.value = move.value.copy(transferring = true)
        viewModelScope.launch {
            val language = entry.settings().language.value
            attempt { tableMoves.move(orderId, source.tableNumber, to) }.onFailure {
                entry.errors().show(it, language, null, ERROR_TTL_MS)
                load()
            }
            move.value = MoveState()
        }
    }

    /** A tap on an open takeaway: resumes its order (`resumeTakeaway`). */
    fun resumeTakeaway(takeaway: TakeawayInfo) {
        open { order.loadOrder(takeaway.orderId) }
    }

    /** The new takeaway button: starts a takeaway order (`newTakeaway`). */
    fun newTakeaway() {
        open { insz -> order.startOrder(null, insz) }
    }

    private fun open(
        effect: (Long) -> TablesEffect = { TablesEffect.OpenOrder(it) },
        action: suspend (insz: String) -> Unit,
    ) {
        val insz = session.currentUser.value?.insz ?: return
        if (!busy.compareAndSet(false, true)) return
        viewModelScope.launch {
            try {
                attempt { action(insz) }.fold(
                    onSuccess = { order.current.value?.let { effectChannel.send(effect(it.id)) } },
                    onFailure = { entry.errors().show(it, entry.settings().language.value) },
                )
            } finally {
                busy.set(false)
            }
        }
    }
}

/** Runs [block]; the answer or the failure it threw (cancellation is rethrown). */
@Suppress("TooGenericExceptionCaught")
private suspend fun <T> attempt(block: suspend () -> T): Result<T> =
    try {
        Result.success(block())
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Result.failure(e)
    }
