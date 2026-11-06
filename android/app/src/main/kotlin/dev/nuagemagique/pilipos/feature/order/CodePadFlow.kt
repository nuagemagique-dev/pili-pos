package dev.nuagemagique.pilipos.feature.order

import dev.nuagemagique.pilipos.i18n.AppLanguage
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import timber.log.Timber
import uniffi.pos_ffi.MenuItem

/** The line under the field: what OK will do, or what the last OK did or refused. */
internal sealed interface CodeMessage {
    data object Hint : CodeMessage

    data class WaitingProduct(
        val quantity: Int,
    ) : CodeMessage

    /** The product OK takes ([pickByCode]). */
    data class Match(
        val quantity: Int,
        val item: MenuItem,
    ) : CodeMessage

    /** Several products start with the typed code: the grid shows them. */
    data class Results(
        val count: Int,
    ) : CodeMessage

    data object NoResult : CodeMessage

    data object Invalid : CodeMessage

    data class Unknown(
        val code: String,
    ) : CodeMessage

    data class Ambiguous(
        val code: String,
    ) : CodeMessage

    data object NoMultiplier : CodeMessage

    data object SoldOut : CodeMessage

    data class Added(
        val quantity: Int,
        val item: MenuItem,
    ) : CodeMessage

    /** A digit after an add gave [quantity] of [item]: "Maki saumon × 3". */
    data class Counted(
        val item: MenuItem,
        val quantity: Int,
    ) : CodeMessage
}

/** What the typed [text] would do, before OK. */
internal fun previewCode(
    text: String,
    items: List<MenuItem>,
): CodeMessage =
    when (val entry = parseCodeEntry(text)) {
        CodeEntry.Empty -> CodeMessage.Hint
        CodeEntry.Invalid -> CodeMessage.Invalid
        is CodeEntry.Multiplier -> CodeMessage.WaitingProduct(entry.quantity)
        is CodeEntry.Add -> previewAdd(entry, items)
    }

private fun previewAdd(
    entry: CodeEntry.Add,
    items: List<MenuItem>,
): CodeMessage {
    val pick = pickByCode(items, entry.code) as? CodeMatch.Found
    val count = searchByCode(items, entry.code).size
    return when {
        pick != null -> CodeMessage.Match(entry.quantity, pick.item)
        count == 0 -> CodeMessage.NoResult
        else -> CodeMessage.Results(count)
    }
}

/** What OK (or a tap with a quantity on the pad) does. */
internal sealed interface CodeAction {
    /** Nothing yet: the text is empty or a multiplier waits for a product. */
    data object Wait : CodeAction

    data class Add(
        val item: MenuItem,
        val quantity: Int,
    ) : CodeAction

    data class Refuse(
        val message: CodeMessage,
    ) : CodeAction
}

/**
 * [quantity] of [item] through the pad. While a mode takes the taps ([modeTakesTap]: allergens, sold out) the product
 * goes to the mode as a plain tap; otherwise a multiplier needs a plain product and a sold-out one is refused here.
 */
internal fun addThroughPad(
    item: MenuItem,
    quantity: Int,
    modeTakesTap: Boolean,
): CodeAction =
    when {
        modeTakesTap -> CodeAction.Add(item, 1)
        quantity > 1 && !item.takesQuantity() -> CodeAction.Refuse(CodeMessage.NoMultiplier)
        item.soldOut -> CodeAction.Refuse(CodeMessage.SoldOut)
        else -> CodeAction.Add(item, quantity)
    }

/** What OK does with [text]: the exact code, else the only product whose code starts with it. */
internal fun decideCode(
    text: String,
    items: List<MenuItem>,
    modeTakesTap: Boolean,
): CodeAction =
    when (val entry = parseCodeEntry(text)) {
        CodeEntry.Empty, is CodeEntry.Multiplier -> CodeAction.Wait
        CodeEntry.Invalid -> CodeAction.Refuse(CodeMessage.Invalid)
        is CodeEntry.Add ->
            when (val match = pickByCode(items, entry.code)) {
                CodeMatch.Unknown -> CodeAction.Refuse(CodeMessage.Unknown(entry.code))
                CodeMatch.Ambiguous -> CodeAction.Refuse(CodeMessage.Ambiguous(entry.code))
                is CodeMatch.Found -> addThroughPad(match.item, entry.quantity, modeTakesTap)
            }
    }

/**
 * A tap on [item] in the grid while [text] is on the pad: the quantity on it ("3 *", or "3 * A1" over the results)
 * applies to [item]; null when the pad holds no quantity (a plain tap).
 */
internal fun decideTap(
    text: String,
    item: MenuItem,
    modeTakesTap: Boolean,
): CodeAction? =
    when (val entry = parseCodeEntry(text)) {
        is CodeEntry.Multiplier -> addThroughPad(item, entry.quantity, modeTakesTap)
        is CodeEntry.Add -> addThroughPad(item, entry.quantity, modeTakesTap)
        else -> null
    }

/** The pad's part of the order screen. [items] are every product of the menu, for the lookups. */
internal data class CodePadState(
    val open: Boolean = false,
    val text: String = "",
    val items: List<MenuItem> = emptyList(),
    /** What the last OK did or refused, shown until the next key. */
    val outcome: CodeMessage? = null,
    /** The digit window of the product just added (C7), else null; [windowItem] is that product. */
    val window: DigitWindow? = null,
    val windowItem: MenuItem? = null,
) {
    val letters: List<String> get() = codeLetters(items)
    val message: CodeMessage get() = outcome ?: previewCode(text, items)

    /** The code being typed while the pad is open, else null (the grid shows the category). */
    private val code: String? get() = if (open) parseCodeEntry(text).typedCode() else null

    /** While a code is typed, the products whose code starts with it (all categories); null shows the category. */
    val results: List<MenuItem>? get() = code?.let { searchByCode(items, it) }

    /** The product OK would take, ringed in the results. */
    val pick: MenuItem? get() = code?.let { (pickByCode(items, it) as? CodeMatch.Found)?.item }
}

/** What the code pad needs from the order screen's model. */
internal interface CodePadHost {
    /** The pad as it is now. */
    val pad: CodePadState

    fun updatePad(change: (CodePadState) -> CodePadState)

    /** The order of the route is the current one: only then does the pad change it. */
    fun orderIsCurrent(): Boolean

    /** A mode (allergens, sold out) takes the taps on products: the product goes to it as a plain tap. */
    fun modeTakesTap(): Boolean

    /** [quantity] of [item] through the screen's own path (modes, dialogs, sold out). */
    fun add(
        item: MenuItem,
        quantity: Int,
    )

    /** [delta] more units on a cart line, one at a time; false when the line is gone or the engine refused. */
    suspend fun addUnits(
        lineId: Long,
        delta: Int,
    ): Boolean
}

/**
 * The code pad of the order screen (`CodePad.vue` and its part of `OrderView.vue`): the typed text that filters the
 * grid, OK, the quantity applied to a tap in the grid, and every product of the menu read for the lookups
 * (`get_menu_items` without a category) when the pad opens, with the menu, and again at each OK (the sold-out flags
 * may have changed). Whether the pad is open is this device's choice
 * ([PosSettingsStore.codePadOpen][dev.nuagemagique.pilipos.ui.state.PosSettingsStore.codePadOpen]).
 */
@Suppress("TooManyFunctions")
internal class CodePadFlow(
    private val entry: OrderEntryPoint,
    private val scope: CoroutineScope,
    private val host: CodePadHost,
    /** The clock of the digit window, in milliseconds (a fake one in the tests). */
    private val clock: () -> Long = System::currentTimeMillis,
) {
    // The extra units go one after the other: each reads the line as the one before left it.
    private val digitLock = Mutex()

    /** The "Saisie code" button: opens or closes the pad and remembers it on this device. */
    fun toggle() {
        val open = !host.pad.open
        entry.settings().setCodePadOpen(open)
        host.updatePad { CodePadState(open = open, items = it.items) }
        if (open) scope.launch { load(entry.settings().language.value) }
    }

    /** A key of the pad (a digit, a letter or [TIMES]): first offered to the digit window of the last add. */
    fun type(key: String) {
        if (!takeDigit(key)) setText(typeKey(host.pad.text, key))
    }

    /** The backspace key. */
    fun backspace() = setText(host.pad.text.dropLast(1))

    /** The clear button of the field. */
    fun clear() = setText("")

    /** A plain product was added to [lineId]: for 3 s a digit on the empty field sets how many this add gives. */
    fun openWindow(
        item: MenuItem,
        lineId: Long,
        units: Int,
    ) = host.updatePad { it.copy(window = openDigitWindow(lineId, item.id, units, clock()), windowItem = item) }

    /** Another product is being added, or a non-digit key came: a digit starts a code again. */
    fun closeWindow() = host.updatePad { if (it.window == null) it else it.copy(window = null, windowItem = null) }

    /** A category tab leaves the results of a typed code. */
    fun leaveResults() {
        if (parseCodeEntry(host.pad.text).typedCode() != null) clear()
    }

    /** OK: reads the menu again, then adds or refuses. */
    fun submit() {
        closeWindow()
        if (!host.orderIsCurrent()) return
        scope.launch {
            val text = host.pad.text
            if (parseCodeEntry(text) is CodeEntry.Add) load(entry.settings().language.value)
            run(decideCode(text, host.pad.items, host.modeTakesTap()))
        }
    }

    /** A tap on [item] in the grid: true when the quantity on the pad took it. */
    fun tap(item: MenuItem): Boolean {
        val action = if (host.pad.open) decideTap(host.pad.text, item, host.modeTakesTap()) else null
        action?.let { run(it) }
        return action != null
    }

    /** Every product of the menu, for the lookups; a failure keeps what was read. */
    suspend fun load(language: AppLanguage) {
        try {
            val items = entry.gateway().call { engine -> engine.getMenuItems(null, language.code) }
            host.updatePad { it.copy(items = items) }
        } catch (e: kotlinx.coroutines.CancellationException) {
            throw e
        } catch (
            @Suppress("TooGenericExceptionCaught") e: Exception,
        ) {
            Timber.d(e, "Code pad products not read")
        }
    }

    private fun setText(text: String) = host.updatePad { it.copy(text = text, outcome = null, window = null, windowItem = null) }

    /** A digit inside the open window sets the add's quantity (extra units of the same line); false for any other key. */
    private fun takeDigit(key: String): Boolean {
        val pad = host.pad
        val open = pad.window
        val item = pad.windowItem
        val step = if (pad.text.isEmpty() && host.orderIsCurrent()) stepDigitWindow(open, key, clock()) else DigitStep.Pass
        if (step !is DigitStep.Set || open == null || item == null) return false
        host.updatePad { it.copy(window = step.window) }
        val delta = step.window.quantity - open.quantity
        scope.launch {
            digitLock.withLock {
                if (host.addUnits(step.window.lineId, delta)) {
                    host.updatePad { it.copy(outcome = CodeMessage.Counted(item, step.window.quantity)) }
                } else {
                    host.updatePad { if (it.window == step.window) it.copy(window = open) else it }
                }
            }
        }
        return true
    }

    private fun run(action: CodeAction) {
        when (action) {
            CodeAction.Wait -> Unit
            is CodeAction.Refuse -> host.updatePad { it.copy(outcome = action.message) }
            is CodeAction.Add -> {
                val direct = !host.modeTakesTap() && action.item.addsDirectly()
                setText("")
                host.add(action.item, action.quantity)
                if (direct) host.updatePad { it.copy(outcome = CodeMessage.Added(action.quantity, action.item)) }
            }
        }
    }
}
