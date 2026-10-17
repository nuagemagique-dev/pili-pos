package dev.nuagemagique.pilipos.feature.reports

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.DEFAULT_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ERROR_TOAST_TTL_MS
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.B2bCustomerInput
import uniffi.pos_ffi.Customer

/** How many recent sales the invoice list asks for (`SALES_LIMIT` of `stores/invoice.ts`). */
private const val SALES_LIMIT: Long = 50L

/** How many invoices the list asks for (`INVOICES_LIMIT`). */
private const val INVOICES_LIMIT: Long = 30L

/**
 * State holder of the B2B invoice flow (event I, M150), a port of `ui/src/stores/invoice.ts`.
 *
 * The engine owns every rule: this only calls it through the gateway and keeps what the screen shows. No amount
 * is computed here (PD208): the lines, the VAT and the totals are the engine's preview of the archived sales.
 * Every engine failure is caught and shown in the dialog (or as a toast outside it); with the engine not ready
 * the lists are empty and no action crashes. The text fields are typed on the system keyboard and kept in upper
 * case, as the on-screen keyboard of the Vue UI only types capitals.
 */
@Suppress("TooManyFunctions") // One entry per action of the dialog, as the store of invoice.ts.
internal class InvoiceViewModel(
    private val entry: ReportsEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(InvoicesState())
    private var searching: Job? = null

    /** What the screen draws. */
    val state: StateFlow<InvoicesState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(canCopy = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(canCopy = entry.session().isManager.value))

    // ── Lists ─────────────────────────────────────────────────────

    /** Reads the invoiceable sales and the invoices (the tickets tab opens or a list changed). */
    fun load() {
        viewModelScope.launch {
            loadSales()
            loadInvoices()
        }
    }

    private suspend fun loadSales() {
        val sales = attempt { entry.gateway().call { engine -> engine.listInvoiceableSales(SALES_LIMIT, null) } }.getOrDefault(emptyList())
        // A sale invoiced meanwhile cannot stay ticked.
        val open = sales.filter { it.invoicedBy == null }.map { it.fiscalEventId }.toSet()
        own.update { current -> current.copy(sales = sales, selected = current.selected.filter { it in open }) }
    }

    private suspend fun loadInvoices() {
        val invoices = attempt { entry.gateway().call { engine -> engine.listInvoices(INVOICES_LIMIT) } }.getOrDefault(emptyList())
        own.update { it.copy(invoices = invoices) }
    }

    /** Ticks a sale, or unticks it. */
    fun toggleSale(fiscalEventId: Long) {
        own.update { current ->
            val next =
                if (fiscalEventId in current.selected) current.selected - fiscalEventId else current.selected + fiscalEventId
            current.copy(selected = next)
        }
    }

    private fun clearSelection() {
        own.update { it.copy(selected = emptyList()) }
    }

    // ── The dialog ────────────────────────────────────────────────

    /** The Facture button: opens the dialog on the customer step; nothing happens with no sale ticked. */
    fun openFlow() {
        val selected = own.value.selected
        if (selected.isEmpty()) return
        own.update { it.copy(flow = InvoiceFlow(saleEventIds = selected)) }
    }

    /** Closes the dialog; a signed invoice clears the ticks, and any answer refreshes both lists. */
    fun closeFlow() {
        val flow = own.value.flow ?: return
        val answered = flow.step == InvoiceStep.RESULT
        own.update { it.copy(flow = null) }
        if (answered && flow.result?.success == true) clearSelection()
        if (answered) load()
    }

    // ── Customer step ─────────────────────────────────────────────

    /** The search field: finds known customers by name or VAT number. */
    fun setSearch(text: String) {
        val value = text.uppercase().take(TEXT_MAX_CHARS)
        updateFlow { it.copy(search = value) }
        searching?.cancel()
        if (value.isBlank()) {
            updateFlow { it.copy(searchResults = emptyList()) }
            return
        }
        searching =
            viewModelScope.launch {
                val found = attempt { entry.gateway().call { engine -> engine.searchB2bCustomers(value.trim()) } }.getOrDefault(emptyList())
                updateFlow { if (it.search == value) it.copy(searchResults = found) else it }
            }
    }

    /** The VAT number field. Another number is another question for Peppol and another customer. */
    fun setVatNumber(text: String) {
        val value = text.uppercase().take(VAT_MAX_CHARS)
        updateFlow { flow ->
            flow.copy(form = flow.form.copy(vatNumber = value, id = null), lookup = null, customer = null, customerError = null)
        }
    }

    /** The name field. */
    fun setName(text: String) = setField(TEXT_MAX_CHARS, text) { form, value -> form.copy(name = value) }

    /** The street field. */
    fun setStreet(text: String) = setField(TEXT_MAX_CHARS, text) { form, value -> form.copy(street = value) }

    /** The postal code field. */
    fun setPostalCode(text: String) = setField(POSTAL_MAX_CHARS, text) { form, value -> form.copy(postalCode = value) }

    /** The city field. */
    fun setCity(text: String) = setField(TEXT_MAX_CHARS, text) { form, value -> form.copy(city = value) }

    /** The country field (two letters; the country of the VAT number when empty). */
    fun setCountryCode(text: String) = setField(COUNTRY_CHARS, text) { form, value -> form.copy(countryCode = value) }

    private fun setField(
        maxChars: Int,
        text: String,
        change: (CustomerForm, String) -> CustomerForm,
    ) {
        val value = text.uppercase().take(maxChars)
        updateFlow { it.copy(form = change(it.form, value), customerError = null) }
    }

    /** A known customer picked from the search results fills the form. */
    fun pickCustomer(customer: Customer) {
        updateFlow {
            it.copy(
                form = customer.toForm(),
                customer = customer,
                lookup = null,
                search = "",
                searchResults = emptyList(),
                customerError = null,
            )
        }
    }

    /**
     * The check button: the engine checks the number offline (check digits), finds a known customer under it and
     * asks Peppol (3 s at most, `unknown` when it cannot answer).
     */
    fun checkVat() {
        viewModelScope.launch { runCheck() }
    }

    private suspend fun runCheck(): Boolean {
        val flow = own.value.flow
        if (flow == null || flow.form.vatNumber.isBlank()) return false
        updateFlow { it.copy(lookupBusy = true, customerError = null) }
        return attempt { entry.gateway().call { engine -> engine.lookupB2bCustomer(flow.form.vatNumber) } }.fold(
            onSuccess = { answer ->
                updateFlow { current ->
                    val known = answer.customer
                    val form =
                        known?.toForm()
                            ?: current.form.copy(
                                vatNumber = answer.vatNumber,
                                countryCode = current.form.countryCode.ifEmpty { answer.countryCode },
                            )
                    current.copy(lookup = answer, lookupBusy = false, form = form, customer = known ?: current.customer)
                }
                true
            },
            onFailure = { failure ->
                updateFlow { it.copy(lookup = null, lookupBusy = false, customerError = describe(failure)) }
                false
            },
        )
    }

    /** The continue button: saves the customer, then shows what the invoice would contain. */
    fun continueToPreview() {
        val flow = own.value.flow ?: return
        if (!flow.canContinue) return
        updateFlow { it.copy(busy = true, customerError = null) }
        viewModelScope.launch {
            val ready = own.value.flow?.lookup != null || runCheck()
            if (ready) saveAndPreview() else updateFlow { it.copy(busy = false) }
        }
    }

    private suspend fun saveAndPreview() {
        val flow = own.value.flow ?: return
        val form = flow.form
        val input =
            B2bCustomerInput(
                id = form.id,
                vatNumber = form.vatNumber,
                name = form.name,
                street = form.street,
                postalCode = form.postalCode,
                city = form.city,
                countryCode = form.countryCode,
            )
        val outcome =
            attempt {
                entry.gateway().call { engine ->
                    val saved = engine.saveB2bCustomer(input)
                    saved to engine.invoicePreview(flow.saleEventIds)
                }
            }
        outcome.fold(
            onSuccess = { (saved, preview) ->
                val problems = preview.problems.map { entry.errors().textRaw(it, language()) }
                updateFlow {
                    it.copy(
                        busy = false,
                        customer = saved,
                        preview = preview,
                        problems = problems,
                        step = InvoiceStep.PREVIEW,
                    )
                }
            },
            onFailure = { failure -> updateFlow { it.copy(busy = false, customerError = describe(failure)) } },
        )
    }

    /** The back button of the preview step. */
    fun backToCustomer() {
        updateFlow { if (it.step == InvoiceStep.PREVIEW) it.copy(step = InvoiceStep.CUSTOMER) else it }
    }

    // ── Signing ───────────────────────────────────────────────────

    /** The confirm button: signs the invoice (event I), prints it and queues its Peppol delivery. */
    fun confirm() {
        val flow = own.value.flow ?: return
        val customer = flow.customer
        val insz = entry.session().currentUser.value?.insz
        val ready = flow.canConfirm && flow.step == InvoiceStep.PREVIEW
        if (customer == null || insz == null || !ready) return
        updateFlow { it.copy(busy = true, step = InvoiceStep.PROCESSING) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.signInvoiceFiscal(flow.saleEventIds, customer.id, insz) } }.fold(
                onSuccess = { result ->
                    val error = if (result.success) null else entry.errors().textRaw(result.error, language())
                    updateFlow { it.copy(busy = false, step = InvoiceStep.RESULT, result = result, resultError = error) }
                },
                onFailure = { failure ->
                    updateFlow { it.copy(busy = false, step = InvoiceStep.RESULT, result = null, resultError = describe(failure)) }
                },
            )
        }
    }

    // ── Copy of an invoice ────────────────────────────────────────

    /** The Copie button of an invoice: a copy (event C) on the invoice event, printed with the invoice layout. */
    fun copyInvoice(fiscalEventId: Long) {
        val insz = entry.session().currentUser.value?.insz ?: return
        if (own.value.copying != null) return
        own.update { it.copy(copying = fiscalEventId) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.copyTicket(fiscalEventId, insz, null) } }.fold(
                onSuccess = { result ->
                    if (result.success) {
                        toast(ToastVariant.Success, text("report_reprint_ok"), SUCCESS_TOAST_MS)
                    } else {
                        toast(ToastVariant.Error, entry.errors().textRaw(result.error, language()), ERROR_TOAST_TTL_MS)
                    }
                },
                onFailure = { toast(ToastVariant.Error, describe(it), ERROR_TOAST_TTL_MS) },
            )
            own.update { it.copy(copying = null) }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────

    private fun updateFlow(change: (InvoiceFlow) -> InvoiceFlow) {
        own.update { current -> current.flow?.let { current.copy(flow = change(it)) } ?: current }
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

    private companion object {
        /** Lifetime of the toast of a copy that was signed (3000 in `InvoiceList.vue`). */
        const val SUCCESS_TOAST_MS: Long = 3000L
    }
}

private fun Customer.toForm(): CustomerForm =
    CustomerForm(
        id = id,
        vatNumber = vatNumber,
        name = name,
        street = street,
        postalCode = postalCode,
        city = city,
        countryCode = countryCode,
    )

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
