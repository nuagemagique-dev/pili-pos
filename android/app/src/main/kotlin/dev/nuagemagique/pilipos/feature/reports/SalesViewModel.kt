package dev.nuagemagique.pilipos.feature.reports

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.DEFAULT_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ERROR_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.Instant

/** How many digits a typed date takes (`ddmmyyyy`). */
private const val TYPED_DIGITS: Int = 8

/** Lifetime of the toast of a printed report. */
private const val PRINTED_TOAST_MS: Long = 4000L

/**
 * State holder of the sales tab, a port of `ui/src/components/SalesReportPanel.vue`.
 *
 * The engine owns every figure and rule (the range limit, what a sale is, the gap with the Z); this only asks for
 * the report of a range, keeps it, and sends the printing. Read only: nothing is signed. A failure is caught and
 * shown in the tab (the report) or as a toast (the printing); with the engine not ready the tab is empty.
 */
@Suppress("TooManyFunctions") // One entry per button of the tab.
internal class SalesViewModel(
    private val entry: ReportsEntryPoint,
    private val now: () -> Instant = Instant::now,
) : ViewModel() {
    private val own = MutableStateFlow(SalesState())

    /** What the screen draws. */
    val state: StateFlow<SalesState> = own.asStateFlow()

    /** The tab is opened: shows today, as the panel does when it mounts. */
    fun open() {
        pick(SalesPreset.TODAY)
    }

    /**
     * Chooses a range. A preset is loaded from the business date (the calendar date until the engine said it,
     * then once more if the two differ); [SalesPreset.CUSTOM] only opens the empty date fields.
     */
    fun pick(preset: SalesPreset) {
        own.update { it.copy(preset = preset) }
        if (preset == SalesPreset.CUSTOM) {
            own.update { it.copy(fromDigits = "", toDigits = "", field = SalesField.FROM) }
            return
        }
        viewModelScope.launch {
            val first = SalesRange.presetRange(preset, own.value.businessDate ?: SalesRange.calendarToday(now()))
            load(first.first, first.second)
            val business = own.value.businessDate ?: return@launch
            val corrected = SalesRange.presetRange(preset, business)
            if (corrected != first && own.value.preset == preset) load(corrected.first, corrected.second)
        }
    }

    /** The Show button of the custom range. */
    fun applyCustom() {
        val from = own.value.typedFrom ?: return
        val to = own.value.typedTo ?: return
        viewModelScope.launch { load(from, to) }
    }

    /** Chooses the date field the keypad types into. */
    fun setField(field: SalesField) {
        own.update { it.copy(field = field) }
    }

    /** A key of the date keypad; a complete start date moves on to the end date. */
    fun onKey(key: NumPadKey) {
        own.update { current ->
            val digits = if (current.field == SalesField.FROM) current.fromDigits else current.toDigits
            val next =
                when (key) {
                    is NumPadKey.Digit -> if (digits.length < TYPED_DIGITS) digits + key.digit else digits
                    NumPadKey.Backspace -> digits.dropLast(1)
                    NumPadKey.Clear -> ""
                }
            val typed = if (current.field == SalesField.FROM) current.copy(fromDigits = next) else current.copy(toDigits = next)
            if (typed.field == SalesField.FROM && next.length == TYPED_DIGITS) typed.copy(field = SalesField.TO) else typed
        }
    }

    /** Chooses the order of the by-product table. */
    fun setSort(sort: SalesSort) {
        own.update { it.copy(sort = sort) }
    }

    /** The Print button: the report on the ticket printer, for the range on screen. */
    fun print() {
        val current = own.value
        if (current.printing || current.report == null) return
        own.update { it.copy(printing = true) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.printSalesReport(current.from, current.to) } }.fold(
                onSuccess = { toast(ToastVariant.Success, text("sales_printed"), PRINTED_TOAST_MS) },
                onFailure = { toast(ToastVariant.Error, describe(it), ERROR_TOAST_TTL_MS) },
            )
            own.update { it.copy(printing = false) }
        }
    }

    private suspend fun load(
        from: String,
        to: String,
    ) {
        own.update { it.copy(loading = true, error = null) }
        attempt { entry.gateway().call { engine -> engine.salesReport(from, to) } }.fold(
            onSuccess = { report ->
                own.update { it.copy(report = report, from = from, to = to, businessDate = report.businessDate, loading = false) }
            },
            onFailure = { failure -> own.update { it.copy(report = null, error = describe(failure), loading = false) } },
        )
    }

    private fun describe(error: Throwable): String = entry.errors().text(error, language())

    private fun language(): AppLanguage = entry.settings().language.value

    private fun text(key: String): String = entry.catalog().text(key, language())

    private fun toast(
        variant: ToastVariant,
        message: String,
        ttlMs: Long = DEFAULT_TOAST_TTL_MS,
    ) {
        entry.toasts().show(message, variant, ttlMs)
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
