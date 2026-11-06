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
import dev.nuagemagique.pilipos.ui.state.IdleWarning
import dev.nuagemagique.pilipos.ui.state.TableMoves
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
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
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.TableInfo
import uniffi.pos_ffi.TakeawayTag
import uniffi.pos_ffi.User
import java.util.concurrent.atomic.AtomicBoolean

/** Lifetime of the toast that reports a refused order registration, in milliseconds (6000 in `AppHeader.vue`). */
internal const val REGISTER_FAILED_TOAST_MS: Long = 6000L

/** How long a first tap on "Fin de service" stays armed, in milliseconds (`ARM_MS` of `armedAction.ts`). */
internal const val END_SHIFT_ARM_MS: Long = 5000L

/** Lifetime of the toast that reports a failed clock-out, in milliseconds (6000 in `auth.ts`). */
private const val PRESENCE_TOAST_MS: Long = 6000L

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
    /** The engine's warning that the idle lock is near (UX audit L4), or null. */
    val idleWarning: IdleWarning? = null,
    /** The open dine-in order on screen is on a table (the lines are checked apart): it may move. */
    val tableOpen: Boolean = false,
    /** The table chip is a button (UX audit C8): the order is dine-in, open, and holds lines. */
    val canMoveTable: Boolean = false,
    /** The other tables of the plan while the table picker is open, else null. */
    val tablePicker: List<TableInfo>? = null,
    /** The number of the day of the takeaway on screen (`E-61`), when it has one (UX audit T2). */
    val takeawayLabel: String? = null,
    /** The customer name of the takeaway on screen, when one was entered. */
    val customerName: String? = null,
    /** The customer phone of the takeaway on screen, when one was entered. */
    val customerPhone: String? = null,
    /** The takeaway is open and has a number: the header offers "Ajouter un nom". */
    val customerEditable: Boolean = false,
    /** The step of the customer dialogs that is open, else null. */
    val customerStep: CustomerStep? = null,
    /** The menu under the user button is open (UX audit L1, R5). */
    val userMenuOpen: Boolean = false,
    /** "Fin de service" was tapped once: its button asks for confirmation for [END_SHIFT_ARM_MS]. */
    val endShiftArmed: Boolean = false,
    /** The language the screens use now, the one the language row of the user menu shows as chosen. */
    val language: AppLanguage = AppLanguage.FR,
) {
    /** The takeaway on screen already has a name or a phone. */
    val hasCustomer: Boolean get() = !customerName.isNullOrEmpty() || !customerPhone.isNullOrEmpty()

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
    private val tablePicker = MutableStateFlow<List<TableInfo>?>(null)
    private val customerStep = MutableStateFlow<CustomerStep?>(null)
    private val userMenu = MutableStateFlow(false)
    private val endShiftArmed = MutableStateFlow(false)
    private var armTimer: Job? = null
    private val tableMoves = TableMoves(order, entry.toasts(), entry.catalog(), entry.settings(), entry.errors())
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
            combine(
                combine(pinOpen, tablePicker, order.lines, order.takeawayTag, customerStep) { pin, picker, lines, tag, step ->
                    Overlay(pin, picker, lines.isNotEmpty(), tag, step)
                },
                combine(userMenu, endShiftArmed, entry.settings().language, ::Menu),
            ) { over, menu -> over.copy(menu = menu) },
            session.idleWarning,
        ) { base, over, warning -> overlay(base, over, warning) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, initialState())

    private fun initialState(): HeaderUiState =
        overlay(
            build(
                session.currentUser.value,
                session.isManager.value,
                order.current.value,
                entry.integrations().hubriseVisible.value,
                entry.webshop().pendingCount.value,
            ),
            Overlay(
                pinOpen.value,
                tablePicker.value,
                order.lines.value.isNotEmpty(),
                order.takeawayTag.value,
                customerStep.value,
                Menu(userMenu.value, endShiftArmed.value, entry.settings().language.value),
            ),
            session.idleWarning.value,
        )

    /** What does not come from the order: the open dialogs, the lines, the takeaway tag. */
    private data class Overlay(
        val pin: Boolean,
        val picker: List<TableInfo>?,
        val hasLines: Boolean,
        val tag: TakeawayTag?,
        val step: CustomerStep?,
        val menu: Menu = Menu(),
    )

    /** The user menu: whether it is open, whether "Fin de service" is armed, and the language shown as chosen. */
    private data class Menu(
        val open: Boolean = false,
        val armed: Boolean = false,
        val language: AppLanguage = AppLanguage.FR,
    )

    /** The idle warning and the open dialogs on top of the order's state. */
    private fun overlay(
        base: HeaderUiState,
        over: Overlay,
        warning: IdleWarning?,
    ): HeaderUiState {
        val movable = base.tableOpen && over.hasLines
        val tag = over.tag.takeIf { base.takeawayOrder }
        return base.copy(
            pinOpen = over.pin && base.roleKey != null,
            idleWarning = warning,
            canMoveTable = movable,
            tablePicker = if (movable) over.picker else null,
            takeawayLabel = tag?.label,
            customerName = tag?.customerName,
            customerPhone = tag?.customerPhone,
            customerEditable = tag != null,
            customerStep = over.step.takeIf { tag != null },
            userMenuOpen = over.menu.open && base.roleKey != null,
            endShiftArmed = over.menu.armed,
            language = over.menu.language,
        )
    }

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
        val dineIn = onOrder && current?.orderType == OrderType.DINE_IN
        return HeaderUiState(
            area = area,
            userName = user?.name.orEmpty(),
            roleKey = user?.let { "role_" + it.role.name.lowercase() },
            isManager = manager,
            tableNumber = if (onOrder && current?.orderType == OrderType.DINE_IN) current.tableNumber else null,
            takeawayOrder = onOrder && current?.orderType == OrderType.TAKEAWAY,
            dineInOrder = onOrder && current?.orderType == OrderType.DINE_IN,
            tableOpen = dineIn && current?.tableNumber != null && current.status == OrderStatus.OPEN,
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
        closeUserMenu()
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

    /** The tap on the user button opens the menu under it (only with a user). */
    fun openUserMenu() {
        if (session.currentUser.value == null) return
        userMenu.value = !userMenu.value
        if (!userMenu.value) disarmEndShift()
    }

    /** A tap outside the user menu closes it. */
    fun closeUserMenu() {
        userMenu.value = false
        disarmEndShift()
    }

    /** "Changer mon PIN" of the user menu opens the own-PIN dialog (only with a user). */
    fun openPin() {
        closeUserMenu()
        if (session.currentUser.value != null) pinOpen.value = true
    }

    /**
     * The language row of the user menu (UX audit R5): the choice is kept for this user on this terminal and applies
     * at once; the terminal language of Settings is untouched.
     */
    fun pickLanguage(language: AppLanguage) {
        val insz = session.currentUser.value?.insz ?: return
        entry.settings().setUserLanguage(insz, language)
    }

    /**
     * "Fin de service" of the user menu (UX audit L1): the first tap arms it ("Confirmer la fin de service ?" for
     * [END_SHIFT_ARM_MS]), the second one within that time ends the shift. The first label comes back by itself.
     */
    fun tapEndShift() {
        if (!endShiftArmed.value) {
            endShiftArmed.value = true
            armTimer?.cancel()
            armTimer =
                viewModelScope.launch {
                    delay(END_SHIFT_ARM_MS)
                    endShiftArmed.value = false
                }
            return
        }
        closeUserMenu()
        endShift()
    }

    private fun disarmEndShift() {
        armTimer?.cancel()
        armTimer = null
        endShiftArmed.value = false
    }

    /**
     * Ends the shift of the logged-in operator: what was ordered is registered (a refusal is shown and does not stop
     * it), the engine signs their clock-out (S event, the same as the end of shift of the login screen; the open
     * session is the proof of the PIN, so none is asked again), then the till locks and the login screen opens. A
     * clock-out that was not registered is shown and never keeps the operator logged in.
     */
    private fun endShift() {
        val name = session.currentUser.value?.name ?: return
        runOnce {
            attempt { order.leaveIfActive(LeaveReason.NAVIGATE) }.onFailure { showRegisterFailed(it) }
            val result = attempt { session.workOutSelf() }
            session.lock()
            result.fold(
                onSuccess = {
                    if (it.success) {
                        entry.toasts().show(text("shift_ended", "name" to name), ToastVariant.Success)
                    } else {
                        entry.errors().showRaw(it.error ?: "FDM error", language(), text("presence_failed"), PRESENCE_TOAST_MS)
                    }
                },
                onFailure = { entry.errors().show(it, language(), text("presence_failed"), PRESENCE_TOAST_MS) },
            )
            order.resetLocal()
            effectChannel.send(HeaderEffect.ToLogin)
        }
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

    // ── Moving the table (UX audit C8) ────────────────────────────

    /**
     * The tap on the table chip (`TableMover.vue`): reads the plan and opens the picker with the other tables. A
     * refusal to read it is shown as an error toast and nothing opens.
     */
    fun openTablePicker() {
        val from = state.value.tableNumber ?: return
        if (!state.value.canMoveTable) return
        runOnce {
            attempt { entry.gateway().call { engine -> engine.listTables() } }.fold(
                onSuccess = { tables -> tablePicker.value = tables.filter { it.tableNumber != from } },
                onFailure = { entry.errors().show(it, language()) },
            )
        }
    }

    /** "Ajouter un nom": opens the name keypad of the takeaway on screen. */
    fun openCustomer() {
        if (state.value.customerEditable) customerStep.value = CustomerStep.NAME
    }

    /** Closes the customer dialogs; what was already saved stays. */
    fun closeCustomer() {
        customerStep.value = null
    }

    /** The name was confirmed: it is saved at once, then the phone step follows (a refusal is shown by the order screen). */
    fun submitCustomerName(name: String) {
        val phone = order.takeawayTag.value?.customerPhone
        customerStep.value = null
        viewModelScope.launch {
            if (order.setTakeawayCustomer(name.ifBlank { null }, phone)) customerStep.value = CustomerStep.PHONE
        }
    }

    /** The phone was confirmed ("Passer" is [closeCustomer]); an empty one clears it. */
    fun submitCustomerPhone(digits: String) {
        val name = order.takeawayTag.value?.customerName
        customerStep.value = null
        viewModelScope.launch { order.setTakeawayCustomer(name, PhoneEntry.value(digits)) }
    }

    /** Closes the table picker without moving anything. */
    fun closeTablePicker() {
        tablePicker.value = null
    }

    /** The free table tapped in the picker: the order moves there at once, the toast offers "Annuler". */
    fun pickTable(toTable: Int) {
        val current = order.current.value
        val from = current?.tableNumber
        tablePicker.value = null
        if (current == null || from == null) return
        runOnce {
            attempt { tableMoves.move(current.id, from, toTable) }.onFailure {
                entry.errors().show(it, language(), null, REGISTER_FAILED_TOAST_MS)
            }
        }
    }

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
