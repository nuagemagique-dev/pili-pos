package dev.nuagemagique.pilipos.ui.scaffold

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.feature.order.OrderRoutes
import dev.nuagemagique.pilipos.feature.reports.ReportsRoutes
import dev.nuagemagique.pilipos.feature.settings.SettingsRoutes
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.feature.webshop.WebshopRoutes
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.User
import java.util.concurrent.atomic.AtomicBoolean

/** Lifetime of the toast that reports a refused order registration, in milliseconds (6000 in `AppHeader.vue`). */
internal const val REGISTER_FAILED_TOAST_MS: Long = 6000L

/** What the header shows (the refs and computeds of the script of `AppHeader.vue`). */
internal data class HeaderUiState(
    /** The screen the header sits on. */
    val area: HeaderArea = HeaderArea.TABLES,
    /** The name of the operator; empty when nobody is logged in. */
    val userName: String = "",
    /** The Vue key of the role of the operator (`role_admin`, ...); null when nobody is logged in. */
    val roleKey: String? = null,
    /** True for an administrator: Reports and Settings show. */
    val isManager: Boolean = false,
    /** The table of the current dine-in order, shown on the order and checkout screens only. */
    val tableNumber: Int? = null,
    /** True while a takeaway order is open on the order or checkout screen. */
    val takeawayOrder: Boolean = false,
    /** True when the order is a dine-in one on the order or checkout screen: Tables is "you are here". */
    val dineInOrder: Boolean = false,
    /** True while HubRise is visible: the online orders button and the HubRise light show. */
    val hubriseVisible: Boolean = false,
    /** Online orders waiting for the operator (the red badge). */
    val pendingOrders: Int = 0,
    /** True while the own-PIN dialog is open. */
    val pinOpen: Boolean = false,
) {
    /** The Tables button is the active one (`tablesActive`). */
    val tablesActive: Boolean get() = area == HeaderArea.TABLES || dineInOrder

    /** The New takeaway button is the active one (`takeawayActive`). */
    val takeawayActive: Boolean get() = takeawayOrder
}

/**
 * State holder of the staff header, the script of `ui/src/components/AppHeader.vue` plus the leave guard of
 * `router/index.ts` (PD210).
 *
 * The engine owns every rule (the registration of the order, the lock, the PIN): this calls it through the shared
 * stores and the gateway. Leaving the order or checkout screen first asks [uniffi.pos_ffi.LeaveReason.NAVIGATE]; a
 * refusal shows `order_register_failed : <error>` for 6 s and the operator stays. It never navigates by itself:
 * [effects] carries each move, sent after an action of the operator succeeded (PD205).
 */
@Suppress("TooManyFunctions") // One entry per button of the header.
internal class HeaderViewModel(
    private val entry: ScaffoldEntryPoint,
    private val area: HeaderArea,
) : ViewModel() {
    private val session = entry.session()
    private val order = entry.order()
    private val pinOpen = MutableStateFlow(false)
    private val effectChannel = Channel<HeaderEffect>(Channel.BUFFERED)

    // One action at a time: a second tap while the first still talks to the engine is ignored.
    private val busy = AtomicBoolean(false)

    /** What the header draws. */
    val state: StateFlow<HeaderUiState> =
        combine(
            combine(
                session.currentUser,
                session.isManager,
                order.current,
                entry.integrations().hubriseVisible,
                entry.webshop().pendingCount,
            ) { user, manager, current, hubrise, pending -> build(user, manager, current, hubrise, pending) },
            pinOpen,
        ) { base, pin -> base.copy(pinOpen = pin && base.roleKey != null) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, initialState())

    private fun initialState(): HeaderUiState =
        build(
            session.currentUser.value,
            session.isManager.value,
            order.current.value,
            entry.integrations().hubriseVisible.value,
            entry.webshop().pendingCount.value,
        ).let { it.copy(pinOpen = pinOpen.value && it.roleKey != null) }

    /** One-shot moves, each delivered once. */
    val effects: Flow<HeaderEffect> = effectChannel.receiveAsFlow()

    private fun build(
        user: User?,
        manager: Boolean,
        current: Order?,
        hubrise: Boolean,
        pending: Int,
    ): HeaderUiState {
        val onOrder = area.holdsOrder && current != null
        return HeaderUiState(
            area = area,
            userName = user?.name.orEmpty(),
            roleKey = user?.let { "role_" + it.role.name.lowercase() },
            isManager = manager,
            tableNumber = if (onOrder && current?.orderType == OrderType.DINE_IN) current.tableNumber else null,
            takeawayOrder = onOrder && current?.orderType == OrderType.TAKEAWAY,
            dineInOrder = onOrder && current?.orderType == OrderType.DINE_IN,
            hubriseVisible = hubrise,
            pendingOrders = pending,
        )
    }

    // ── Workspaces ────────────────────────────────────────────────

    /** The Tables button: Tables is where the operator already is when [area] is Tables. */
    fun goTables() = leaveThenGo(TablesRoutes.TABLES, stay = area == HeaderArea.TABLES)

    /** The online orders button, only while HubRise is visible. */
    fun goOnlineOrders() =
        leaveThenGo(WebshopRoutes.WEBSHOP, stay = area == HeaderArea.WEBSHOP || !entry.integrations().hubriseVisible.value)

    /** The Reports button, managers only. */
    fun goReports() = leaveThenGo(ReportsRoutes.REPORTS, stay = area == HeaderArea.REPORTS || !session.isManager.value)

    /** The Settings button, managers only. */
    fun goSettings() = leaveThenGo(SettingsRoutes.SETTINGS, stay = area == HeaderArea.SETTINGS || !session.isManager.value)

    private fun leaveThenGo(
        route: String,
        stay: Boolean,
    ) {
        if (stay) return
        runOnce {
            if (leaveOrderArea()) effectChannel.send(HeaderEffect.Navigate(route))
        }
    }

    /**
     * The guard of `router/index.ts`: leaving the order or checkout screen with an order registers what was ordered
     * first. True when the operator may go; a refusal shows the toast and answers false.
     */
    private suspend fun leaveOrderArea(): Boolean {
        if (!area.holdsOrder || session.currentUser.value == null) return true
        return attempt { order.leaveIfActive(LeaveReason.NAVIGATE) }.getOrElse {
            showRegisterFailed(it)
            false
        }
    }

    // ── New takeaway ──────────────────────────────────────────────

    /**
     * The New takeaway button. A second tap on a takeaway already on the order screen does nothing instead of
     * silently replacing the cart. Otherwise the order in hand is registered (`switch_order`), a takeaway starts and
     * the order screen opens on it.
     */
    fun newTakeaway() {
        if (area == HeaderArea.ORDER && order.current.value?.orderType == OrderType.TAKEAWAY) return
        val insz = session.currentUser.value?.insz ?: return
        runOnce {
            attempt { order.leaveIfActive(LeaveReason.SWITCH_ORDER) }.onFailure {
                showRegisterFailed(it)
                return@runOnce
            }
            attempt { order.startOrder(null, insz) }.onFailure {
                entry.errors().show(it, language())
                return@runOnce
            }
            val id = order.current.value?.id ?: return@runOnce
            effectChannel.send(HeaderEffect.Navigate(OrderRoutes.order(id)))
        }
    }

    // ── Lock ──────────────────────────────────────────────────────

    /**
     * The Lock button (PD210): the engine registers the open order while the session is still open and ends the
     * session; a refused registration is shown and never blocks the lock. The local order is then forgotten and
     * the login screen opens. The operator stays on shift: clocking out is done from the login screen.
     */
    fun lock() {
        runOnce {
            val outcome = session.lock()
            outcome.orderError?.let {
                entry.errors().showRaw(it, language(), text("order_register_failed"), REGISTER_FAILED_TOAST_MS)
            }
            order.resetLocal()
            effectChannel.send(HeaderEffect.ToLogin)
        }
    }

    // ── Own PIN ───────────────────────────────────────────────────

    /** The tap on the user button opens the own-PIN dialog (only with a user). */
    fun openPin() {
        if (session.currentUser.value != null) pinOpen.value = true
    }

    /** Closes the own-PIN dialog without saving. */
    fun closePin() {
        pinOpen.value = false
    }

    /**
     * Saves the PIN typed twice for the operator (`set_pin`, no operator PIN: the own PIN needs none) and confirms
     * it with a toast. A refusal propagates to the dialog.
     */
    suspend fun submitPin(pin: String) {
        val insz = session.currentUser.value?.insz ?: return
        entry.gateway().call { engine -> engine.setPin(insz, pin, null) }
        entry.toasts().show(text("pin_changed"), ToastVariant.Success)
    }

    /** The screen text of a refusal, in the language of the till. */
    fun describeError(error: Throwable): String = entry.errors().text(error, language())

    // ── Helpers ───────────────────────────────────────────────────

    private fun showRegisterFailed(error: Throwable) {
        entry.errors().show(error, language(), text("order_register_failed"), REGISTER_FAILED_TOAST_MS)
    }

    private fun runOnce(block: suspend () -> Unit) {
        if (!busy.compareAndSet(false, true)) return
        viewModelScope.launch {
            try {
                block()
            } finally {
                busy.set(false)
            }
        }
    }

    private fun language(): AppLanguage = entry.settings().language.value

    private fun text(
        key: String,
        vararg vars: Pair<String, Any>,
    ): String = entry.catalog().text(key, language(), vars.toMap())
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
