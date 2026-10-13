package dev.nuagemagique.pilipos.feature.tables

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
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
)

/** What the view model asks of the navigation: the order screen of the order that was just opened. */
internal sealed interface TablesEffect {
    /** Opens the order screen of order [orderId]. */
    data class OpenOrder(
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
internal class TablesViewModel(
    private val entry: TablesEntryPoint,
    private val clock: Clock = Clock.systemUTC(),
) : ViewModel() {
    private val order = entry.order()
    private val session = entry.session()
    private val gateway = entry.gateway()
    private val own = MutableStateFlow(Lists())
    private val now = MutableStateFlow(clock.instant())
    private val effectChannel = Channel<TablesEffect>(Channel.BUFFERED)

    // One action at a time: a second tap while the first still talks to the engine is ignored.
    private val busy = AtomicBoolean(false)

    private data class Lists(
        val tables: List<TableInfo> = emptyList(),
        val takeaways: List<TakeawayInfo> = emptyList(),
        val cardCharged: Set<Long> = emptySet(),
        val loaded: Boolean = false,
    )

    /** What the screen draws. */
    val state: StateFlow<TablesUiState> =
        combine(own, now, session.currentUser) { lists, instant, user -> build(lists, instant, user != null) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, build(own.value, now.value, session.currentUser.value != null))

    /** One-shot moves, each delivered once. */
    val effects: Flow<TablesEffect> = effectChannel.receiveAsFlow()

    init {
        viewModelScope.launch {
            gateway.ready.collect { ready -> if (ready) load() else own.value = Lists() }
        }
        viewModelScope.launch {
            entry.events().events.collect { event -> if (event is PosEvent.Resync) load() }
        }
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
    ) = TablesUiState(lists.tables, lists.takeaways, instant, signedIn, lists.loaded, lists.cardCharged)

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
                own.value = Lists(tables, takeaways, charged.map { it.orderId }.toSet(), loaded = true)
            },
            // The lists stay as they were: the next resume or resync reads them again.
            onFailure = { Timber.d(it, "Tables not loaded") },
        )
    }

    // ── Opening an order ──────────────────────────────────────────

    /** A tap on [table]: an occupied one loads its order and its lines, a free one starts an order (`selectTable`). */
    fun selectTable(table: TableInfo) {
        val orderId = table.orderId
        open { insz -> if (orderId != null) order.loadOrder(orderId) else order.startOrder(table.tableNumber, insz) }
    }

    /** A tap on an open takeaway: resumes its order (`resumeTakeaway`). */
    fun resumeTakeaway(takeaway: TakeawayInfo) {
        open { order.loadOrder(takeaway.orderId) }
    }

    /** The new takeaway button: starts a takeaway order (`newTakeaway`). */
    fun newTakeaway() {
        open { insz -> order.startOrder(null, insz) }
    }

    private fun open(action: suspend (insz: String) -> Unit) {
        val insz = session.currentUser.value?.insz ?: return
        if (!busy.compareAndSet(false, true)) return
        viewModelScope.launch {
            try {
                attempt { action(insz) }.fold(
                    onSuccess = { order.current.value?.let { effectChannel.send(TablesEffect.OpenOrder(it.id)) } },
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
