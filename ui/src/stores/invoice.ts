import { defineStore } from "pinia";
import { computed, ref } from "vue";
import * as ipc from "../api/ipc";
import { describeError } from "../utils/errorMessages";
import { useAuthStore } from "./auth";

/** The steps of the invoice dialog (customer, preview, signing, result). */
export type InvoiceStep = "customer" | "preview" | "processing" | "result";

/** The fields the on-screen keyboard types into. */
export type CustomerField =
  | "search"
  | "vatNumber"
  | "name"
  | "street"
  | "postalCode"
  | "city"
  | "countryCode";

/** The customer form of the dialog. */
export interface CustomerForm {
  id: number | null;
  vatNumber: string;
  name: string;
  street: string;
  postalCode: string;
  city: string;
  countryCode: string;
}

const EMPTY_FORM: CustomerForm = {
  id: null,
  vatNumber: "",
  name: "",
  street: "",
  postalCode: "",
  city: "",
  countryCode: "",
};

/** How many recent sales and invoices the Reports screen asks for. */
const SALES_LIMIT = 50;
const INVOICES_LIMIT = 30;

/**
 * B2B invoices (event I, M150): the sales a cashier can invoice, the
 * multi-selection, the invoice dialog (customer, preview, signing,
 * result) and the list of invoices. The engine owns every rule: the
 * lines, VAT and totals come from the archived sales (the UI sends sale
 * ids and a customer id), the VAT number is checked by the engine, and
 * a Peppol lookup that fails or is offline gives `unknown` and never
 * blocks the invoice.
 */
export const useInvoiceStore = defineStore("invoice", () => {
  const auth = useAuthStore();

  // ── Sales and invoices lists ──────────────────────────────────
  const sales = ref<ipc.InvoiceableSale[]>([]);
  const invoices = ref<ipc.InvoiceSummary[]>([]);
  /** Fiscal event ids ticked in the tickets list. */
  const selected = ref<number[]>([]);

  async function loadSales() {
    try {
      sales.value = (await ipc.listInvoiceableSales(SALES_LIMIT)) ?? [];
    } catch {
      sales.value = [];
    }
    // A sale invoiced meanwhile cannot stay ticked.
    const open = new Set(sales.value.filter((s) => !s.invoicedBy).map((s) => s.fiscalEventId));
    selected.value = selected.value.filter((id) => open.has(id));
  }

  async function loadInvoices() {
    try {
      invoices.value = (await ipc.listInvoices(INVOICES_LIMIT)) ?? [];
    } catch {
      invoices.value = [];
    }
  }

  function toggleSale(fiscalEventId: number) {
    selected.value = selected.value.includes(fiscalEventId)
      ? selected.value.filter((id) => id !== fiscalEventId)
      : [...selected.value, fiscalEventId];
  }

  function clearSelection() {
    selected.value = [];
  }

  // ── The dialog ────────────────────────────────────────────────
  const open = ref(false);
  const step = ref<InvoiceStep>("customer");
  const form = ref<CustomerForm>({ ...EMPTY_FORM });
  const activeField = ref<CustomerField>("vatNumber");
  const search = ref("");
  const searchResults = ref<ipc.B2bCustomer[]>([]);
  const lookup = ref<ipc.CustomerLookup | null>(null);
  const lookupBusy = ref(false);
  const customerError = ref<string | null>(null);
  const customer = ref<ipc.B2bCustomer | null>(null);
  const preview = ref<ipc.InvoicePreview | null>(null);
  const result = ref<ipc.InvoiceResult | null>(null);
  const busy = ref(false);

  /** The Peppol status shown on the chip: the lookup's, else the saved customer's. */
  const peppolStatus = computed<ipc.PeppolStatus>(
    () => lookup.value?.peppolStatus ?? customer.value?.peppolStatus ?? "unknown",
  );

  const formComplete = computed(() => {
    const f = form.value;
    return [f.vatNumber, f.name, f.street, f.postalCode, f.city].every((v) => v.trim() !== "");
  });

  const previewProblems = computed(() => (preview.value?.problems ?? []).map((p) => describeError(p)));

  function openDialog() {
    if (selected.value.length === 0) return;
    form.value = { ...EMPTY_FORM };
    activeField.value = "vatNumber";
    search.value = "";
    searchResults.value = [];
    lookup.value = null;
    customer.value = null;
    preview.value = null;
    result.value = null;
    customerError.value = null;
    step.value = "customer";
    open.value = true;
  }

  /** Closes the dialog; a signed invoice refreshes both lists and clears the ticks. */
  function closeDialog() {
    const signed = step.value === "result" && result.value?.success === true;
    open.value = false;
    if (signed) clearSelection();
    if (step.value === "result") {
      void loadSales();
      void loadInvoices();
    }
  }

  function setField(field: CustomerField, value: string) {
    if (field === "search") {
      search.value = value;
      void runSearch();
      return;
    }
    form.value = { ...form.value, [field]: value };
    customerError.value = null;
    // A changed VAT number is a different question for Peppol.
    if (field === "vatNumber") {
      lookup.value = null;
      // Another number is another customer (a new one, or one found by the check).
      form.value = { ...form.value, id: null };
      customer.value = null;
    }
  }

  function fieldValue(field: CustomerField): string {
    return field === "search" ? search.value : form.value[field];
  }

  async function runSearch() {
    const q = search.value.trim();
    if (q === "") {
      searchResults.value = [];
      return;
    }
    try {
      searchResults.value = (await ipc.searchB2bCustomers(q)) ?? [];
    } catch {
      searchResults.value = [];
    }
  }

  function fillFrom(c: ipc.B2bCustomer) {
    form.value = {
      id: c.id,
      vatNumber: c.vatNumber,
      name: c.name,
      street: c.street,
      postalCode: c.postalCode,
      city: c.city,
      countryCode: c.countryCode,
    };
    customer.value = c;
  }

  /** A known customer picked from the search results. */
  function pickCustomer(c: ipc.B2bCustomer) {
    fillFrom(c);
    lookup.value = null;
    search.value = "";
    searchResults.value = [];
    customerError.value = null;
  }

  /**
   * The engine checks the number offline (check digits), finds a known
   * customer under it and asks Peppol (3 s at most, `unknown` when it
   * cannot answer). Returns false when the number is refused.
   */
  async function checkVat(): Promise<boolean> {
    if (form.value.vatNumber.trim() === "") return false;
    lookupBusy.value = true;
    customerError.value = null;
    try {
      const answer = await ipc.lookupB2bCustomer(form.value.vatNumber);
      lookup.value = answer;
      if (answer?.customer) {
        fillFrom(answer.customer);
      } else if (answer) {
        form.value = {
          ...form.value,
          vatNumber: answer.vatNumber,
          countryCode: form.value.countryCode || answer.countryCode,
        };
      }
      return answer != null;
    } catch (e) {
      lookup.value = null;
      customerError.value = describeError(e);
      return false;
    } finally {
      lookupBusy.value = false;
    }
  }

  /** Saves the customer, then shows what the invoice would contain. */
  async function continueToPreview() {
    if (!formComplete.value || busy.value) return;
    busy.value = true;
    customerError.value = null;
    try {
      if (!lookup.value && !(await checkVat())) return;
      const f = form.value;
      customer.value = await ipc.saveB2bCustomer({
        id: f.id,
        vatNumber: f.vatNumber,
        name: f.name,
        street: f.street,
        postalCode: f.postalCode,
        city: f.city,
        countryCode: f.countryCode,
      });
      preview.value = await ipc.invoicePreview(selected.value);
      step.value = "preview";
    } catch (e) {
      customerError.value = describeError(e);
    } finally {
      busy.value = false;
    }
  }

  function backToCustomer() {
    step.value = "customer";
  }

  /** M150: signs the invoice, prints it and queues its Peppol delivery. */
  async function confirm() {
    const c = customer.value;
    const insz = auth.currentUser?.insz;
    if (!c || !insz || busy.value || (preview.value?.problems.length ?? 0) > 0) return;
    busy.value = true;
    step.value = "processing";
    try {
      result.value = await ipc.signInvoiceFiscal(selected.value, c.id, insz);
    } catch (e) {
      result.value = {
        success: false,
        fiscalEventId: null,
        invoiceNo: null,
        printed: false,
        deliveryStatus: null,
        error: String(e),
      };
    } finally {
      busy.value = false;
      step.value = "result";
    }
  }

  /** A copy (event C) of a signed invoice; printed with the invoice layout. */
  async function copyInvoice(fiscalEventId: number): Promise<ipc.CopyResult> {
    const insz = auth.currentUser?.insz ?? "";
    return ipc.copyTicket(fiscalEventId, insz);
  }

  return {
    sales,
    invoices,
    selected,
    loadSales,
    loadInvoices,
    toggleSale,
    clearSelection,
    open,
    step,
    form,
    activeField,
    search,
    searchResults,
    lookup,
    lookupBusy,
    customerError,
    customer,
    preview,
    result,
    busy,
    peppolStatus,
    formComplete,
    previewProblems,
    openDialog,
    closeDialog,
    setField,
    fieldValue,
    pickCustomer,
    checkVat,
    continueToPreview,
    backToCustomer,
    confirm,
    copyInvoice,
  };
});
