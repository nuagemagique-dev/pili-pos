import { invoke } from "@tauri-apps/api/core";
import type {
  Category,
  MenuItem,
  Order,
  OrderLine,
  CartLine,
  User,
  CreateOrderInput,
  AddOrderLineInput,
  PaymentPart,
  CheckoutPaymentMethod,
  EftPayment,
  PaymentTerminalStatus,
  UnsettledCardPayment,
  CheckoutResult,
  FiscalTicket,
  TicketDeliveryRequest,
  DeliveryOutcome,
  MailStatus,
  DeliveryView,
  SlipOutcome,
  ExportStatus,
  DiscountInput,
  DiscountResult,
} from "../types";

// Menu
export function getCategories(lang?: string): Promise<Category[]> {
  return invoke("get_categories", { lang });
}

export function getMenuItems(
  categoryId?: number,
  lang?: string
): Promise<MenuItem[]> {
  return invoke("get_menu_items", { categoryId, lang });
}

// Orders
export function createOrder(input: CreateOrderInput): Promise<Order> {
  return invoke("create_order", { input });
}

export function addOrderLine(input: AddOrderLineInput): Promise<OrderLine> {
  return invoke("add_order_line", { input });
}

export function removeOrderLine(lineId: number): Promise<void> {
  return invoke("remove_order_line", { lineId });
}

export function updateOrderLineQuantity(
  lineId: number,
  quantity: number
): Promise<void> {
  return invoke("update_order_line_quantity", { lineId, quantity });
}

/** Set (or clear, with null or an empty text) the kitchen note of a
 * line. A note changed after the line went to the kitchen prints a note
 * slip; the answer says what became of it. Throws the error code. */
export function setLineNote(lineId: number, note: string | null): Promise<SlipOutcome> {
  return invoke("set_line_note", { lineId, note });
}

export function getOrder(orderId: number): Promise<Order> {
  return invoke("get_order", { orderId });
}

export function listOrderLines(orderId: number): Promise<OrderLine[]> {
  return invoke("list_order_lines", { orderId });
}

/** The order's lines as the cart shows them (both names, unsent quantity). */
export function listCartLines(orderId: number): Promise<CartLine[]> {
  return invoke("list_cart_lines", { orderId });
}

export function listOpenOrders(): Promise<Order[]> {
  return invoke("list_open_orders");
}

// Auth
export function login(insz: string, pin?: string): Promise<User> {
  return invoke("login", { insz, pin: pin ?? null });
}

/** Deactivates a user (manager only, never oneself). */
export function deactivateUser(insz: string): Promise<void> {
  return invoke("deactivate_user", { insz });
}

/** Ends the backend session: operator commands are refused until the
 * next login. */
export function logout(): Promise<void> {
  return invoke("logout");
}

/** What locking the screen reports: a failed registration of the open
 * order never blocks the lock, its error code comes back here. */
export interface LockOutcome {
  orderError: string | null;
}

/** Lock the screen through the engine: it registers the active order,
 * closes the session (the operator stays on shift) and emits
 * `session-locked`. */
export function lockSession(): Promise<LockOutcome> {
  return invoke("lock_session");
}

/** Payload of the `session-locked` event. */
export interface SessionLocked {
  reason: "manual" | "idle";
  orderError: string | null;
}

/** First login of a user without a PIN (required for a manager):
 * stores the chosen PIN and opens the session in one step. */
export function loginWithNewPin(insz: string, newPin: string): Promise<User> {
  return invoke("login_with_new_pin", { insz, newPin });
}

/** Another user's PIN needs the manager's own (`operatorPin`). */
export function setPin(insz: string, newPin: string, operatorPin?: string): Promise<void> {
  return invoke("set_pin", { insz, newPin, operatorPin });
}

export function listUsers(): Promise<User[]> {
  return invoke("list_users");
}

/** A new manager needs `initialPin` and the creating manager's `operatorPin`. */
export function createUser(input: {
  insz: string;
  name: string;
  role: string;
  initialPin?: string;
  operatorPin?: string;
}): Promise<User> {
  return invoke("create_user", { input });
}

// Print
// print_receipt and open_drawer are no longer registered IPC commands:
// removed as SCE 2.0 loopholes (print_receipt let a screen print a
// second ORIGINAL, and open_drawer opened the drawer without a signed
// F event). The only way to print an original is
// print_missing_original; the drawer only opens via
// open_drawer_fiscal, or backend-side on cash sales/refunds.
export function checkPrinter(): Promise<boolean> {
  return invoke("check_printer");
}

/** `null` when the terminal has no separate kitchen printer. */
export function checkKitchenPrinter(): Promise<boolean | null> {
  return invoke("check_kitchen_printer");
}

// Checkout
export function checkout(
  orderId: number,
  payments: PaymentPart[],
  delivery?: TicketDeliveryRequest
): Promise<CheckoutResult> {
  return invoke("checkout", { orderId, payments, delivery });
}

// Linked card payment terminal (EFT): only with a payment_terminal in pos.json
// (see PosSettings.paymentTerminal).

/** Charge `amountCents` of the order on the terminal; resolves with the payment whatever
 * the terminal said (approved, declined, cancelled, unknown). Rejects when nothing was charged. */
export function eftPay(orderId: number, amountCents: number): Promise<EftPayment> {
  return invoke("eft_pay", { orderId, amountCents });
}

/** Ask the terminal to stop the payment in progress (a request, not a guarantee). */
export function eftAbort(): Promise<void> {
  return invoke("eft_abort");
}

/** Ask the terminal where a `sent` or `unknown` payment stands. */
export function eftCheck(id: number): Promise<EftPayment> {
  return invoke("eft_check", { id });
}

/** An order's approved payments not yet settled and those still waiting for an answer, oldest first. */
export function listEftPayments(orderId: number): Promise<EftPayment[]> {
  return invoke("list_eft_payments", { orderId });
}

/** Whether the linked terminal answers. Charges nothing; "busy" (and connected) while a payment runs. */
export function checkPaymentTerminal(): Promise<PaymentTerminalStatus> {
  return invoke("check_payment_terminal");
}

/** The approved card payments of every order that no sale has settled yet, oldest first: money taken that no ticket records. */
export function listUnsettledCardPayments(): Promise<UnsettledCardPayment[]> {
  return invoke("list_unsettled_card_payments");
}

/** Close an order the customer left without paying (manager, PIN):
 * the sale is signed with no payment line (UC4109). */
export function closeUnpaid(orderId: number, insz: string, pin: string): Promise<CheckoutResult> {
  return invoke("close_unpaid", { orderId, insz, pin });
}

/** One line of a signed sale's payments (command `list_sale_payments`). */
export interface SalePaymentLine {
  /** Position in the sale's payment lines: the index to correct. */
  index: number;
  /** CASH, CARD_DEBIT, CHEQUE_MEAL, ... */
  paymentType: string;
  name: string;
  amountCents: number;
  /** AUTOMATIC: a card the linked terminal took (never corrected). */
  inputMethod: "MANUAL" | "AUTOMATIC";
  /** PAYMENT, or ROUNDING (the cash rounding, not a payment). */
  amountType: "PAYMENT" | "ROUNDING";
  brand: string | null;
}

/** The correction a sale already has. */
export interface CorrectionDone {
  fromType: string;
  toType: string;
  amountCents: number;
  at: string;
}

/** A signed sale's payment lines and whether one can be corrected. */
export interface SalePayments {
  fiscalEventId: number;
  ticketNumber: number;
  totalCents: number;
  lines: SalePaymentLine[];
  correctable: boolean;
  /** Why not, as an engine error code, when `correctable` is false. */
  refusal: string | null;
  corrected: CorrectionDone | null;
}

/** The payment lines of a signed sale and whether one can be corrected (manager). */
export function listSalePayments(fiscalEventId: number): Promise<SalePayments> {
  return invoke("list_sale_payments", { fiscalEventId });
}

/** Correct the method of one payment line of a sale signed today (event F), confirmed by the manager's PIN. A refusal comes back as `success: false` with its error code. */
export function correctPayment(
  fiscalEventId: number,
  lineIndex: number,
  method: CheckoutPaymentMethod,
  insz: string,
  pin: string,
): Promise<{ success: boolean; error: string | null }> {
  return invoke("correct_payment", { fiscalEventId, lineIndex, method, insz, pin });
}

/** The engine's reading of the FDM (event "fdm-status", commands
 * `fdm_health` and `fdm_check_now`). The engine classifies the FDM
 * status; the UI only shows it. */
export interface FdmHealth {
  /** The FDM answered and is neither blocked nor uninitialised. */
  connected: boolean;
  /** The FDM answered at all (false on a transport failure). */
  answered: boolean;
  severity: "blocking" | "warning" | "info";
  code: string | null;
  /** The FDM asks the POS to keep showing the message. */
  mandatory: boolean;
  /** RFC 3339 */
  checkedAt: string;
  /** RFC 3339, when the monitor checks next; null when it does not say. */
  nextCheckAt: string | null;
}

/** The last reading of the engine's FDM monitor, null before the first. */
export function fdmHealth(): Promise<FdmHealth | null> {
  return invoke("fdm_health");
}

/** Check the FDM at once (also without a session); the monitor is rescheduled. */
export function fdmCheckNow(): Promise<FdmHealth> {
  return invoke("fdm_check_now");
}

/** Still a valid command; the UI no longer calls it (the engine monitors the FDM). */
export function checkFdmStatus(): Promise<unknown> {
  return invoke("check_fdm_status");
}

// ── Ticket delivery (paper / email / both) ──────────────────────────

export function checkMailStatus(): Promise<MailStatus> {
  return invoke("check_mail_status");
}

export function listTicketDeliveries(limit?: number): Promise<DeliveryView[]> {
  return invoke("list_ticket_deliveries", { limit });
}

/** The backend rebuilds the ticket from the archive. */
export function printMissingOriginal(fiscalEventId: number): Promise<void> {
  return invoke("print_missing_original", { fiscalEventId });
}

// ── Nightly export (manager only) ───────────────────────────────────

/** The nightly export state: configuration, last export and upload,
 * the outbox. Needs an admin session. */
export function exportStatus(): Promise<ExportStatus> {
  return invoke("export_status");
}

/** Make the pending exports due now and return at once; read
 * `exportStatus()` afterwards. Needs an admin session. */
export function exportRunNow(): Promise<ExportStatus> {
  return invoke("export_run_now");
}

// Settings
export interface PosSettings {
  trainingMode: boolean;
  printerAddress: string | null;
  fdmUrl: string;
  posId: string;
  terminalId: string;
  restaurantName: string;
  language: string;
  /** Software version declared to the FDM (AM Art. 47), e.g. pili-pos/1.0.0. */
  posSwVersion: string;
  /** What the FDM would refuse in pos.json; empty when usable. */
  configProblems: string[];
  /** Kind of the linked card payment terminal (as in pos.json); absent when cards are typed by hand. */
  paymentTerminal?: string;
}

export function getSettings(): Promise<PosSettings> {
  return invoke("get_settings");
}

export function setTrainingMode(enabled: boolean): Promise<boolean> {
  return invoke("set_training_mode", { enabled });
}

// Tables
export interface TableInfo {
  tableNumber: number;
  orderId: number | null;
  totalCents: number;
  lineCount: number;
  status: string;
  /** When the table's order was opened (UTC). */
  openedAt: string | null;
  /** A provisional bill was printed and the order has not changed since. */
  preBillPrinted: boolean;
}

export function listTables(): Promise<TableInfo[]> {
  return invoke("list_tables");
}

/** An open takeaway with lines: a direct sale not paid yet (FAQ Q205). */
export interface TakeawayInfo {
  orderId: number;
  totalCents: number;
  lineCount: number;
  createdAt: string;
}

export function listOpenTakeaways(): Promise<TakeawayInfo[]> {
  return invoke("list_open_takeaways");
}

export type LeaveReason = "navigate" | "switch_order" | "send_to_kitchen";

/** The operator leaves the order: the engine registers what was ordered
 * (event P) for an open dine-in order and does nothing otherwise. Throws
 * the error code on failure. */
export function leaveOrder(orderId: number, reason: LeaveReason): Promise<SlipOutcome> {
  return invoke("leave_order", { orderId, reason });
}

/** Tell the engine which order the operator is working on (null: none). */
export function setActiveOrder(orderId: number | null): Promise<void> {
  return invoke("set_active_order", { orderId });
}

/** Register (event P) what was ordered at a table since the last
 * registration, then print it on the bar / kitchen slip. No-op when
 * nothing changed. */
export function signTableOrder(orderId: number): Promise<SlipOutcome> {
  return invoke("sign_table_order", { orderId });
}

/** Pre-bill; its new lines go to the bar / kitchen slip first. */
export function signPreBill(orderId: number): Promise<SlipOutcome> {
  return invoke("sign_pre_bill", { orderId });
}

/** What moving an order to another table did. */
export interface TableTransferResult {
  orderId: number;
  fromTable: number;
  toTable: number;
  /** The bar / kitchen slip of what was registered before the move. */
  kitchenSlip: SlipOutcome;
  /** The notice that the order moved. */
  transferSlip: SlipOutcome;
}

/** Move the open order of a table to a free table (event P, cost center
 * change). Entries not yet registered are sent first. Throws the error
 * code on a refusal. */
export function transferTable(orderId: number, toTable: number): Promise<TableTransferResult> {
  return invoke("transfer_table", { orderId, toTable });
}

/** Give a discount on an open till order (manager, PIN): a percent, an
 * amount or the whole order off, or one line for free. A table already
 * registered gets its P signed at once. Throws the error code. */
export function applyDiscount(orderId: number, input: DiscountInput, insz: string, pin: string): Promise<DiscountResult> {
  return invoke("apply_discount", { orderId, input, insz, pin });
}

/** Remove the order discount (no `lineId`) or the free line `lineId`
 * (manager, PIN). */
export function removeDiscount(orderId: number, lineId: number | null, insz: string, pin: string): Promise<DiscountResult> {
  return invoke("remove_discount", { orderId, lineId, insz, pin });
}

// Reports
export interface ReportResult {
  success: boolean;
  reportType: string;
  reportNo: number | null;
  error: string | null;
}

export function generateXReport(employeeInsz: string): Promise<ReportResult> {
  return invoke("generate_x_report", { employeeInsz });
}

export function generateZReport(employeeInsz: string): Promise<ReportResult> {
  return invoke("generate_z_report", { employeeInsz });
}

export function generateUserXReport(employeeInsz: string): Promise<ReportResult> {
  return invoke("generate_user_x_report", { employeeInsz });
}

export interface ZReportSummary {
  id: number;
  reportDate: string;
  sequenceNumber: number;
  createdAt: string;
  fiscalEventId: number | null;
}

export function listZReports(): Promise<ZReportSummary[]> {
  return invoke("list_z_reports");
}

export function needsZReport(): Promise<boolean> {
  return invoke("needs_z_report");
}

// Copy
export interface CopyResult {
  success: boolean;
  error: string | null;
  /** The copy ticket, when the original event had an order to show. */
  ticket: FiscalTicket | null;
  /** Archive id of the signed C event. */
  fiscalEventId: number | null;
  /** How the copy reached the customer (paper and/or email). */
  delivery: DeliveryOutcome | null;
}

export function copyTicket(
  fiscalEventId: number,
  employeeInsz: string,
  delivery?: TicketDeliveryRequest
): Promise<CopyResult> {
  return invoke("copy_ticket", { fiscalEventId, employeeInsz, delivery });
}

/** A ticket a customer received, offered for a copy (event C). */
export interface CopyableTicket {
  fiscalEventId: number;
  /** N (sale or refund), P (pre-bill) or T (training). */
  eventLabel: "N" | "P" | "T";
  ticketNumber: number;
  tableNumber: number | null;
  /** The ticket's own total; negative for a refund. */
  totalCents: number;
  createdAt: string;
}

export function listCopyableTickets(limit?: number): Promise<CopyableTicket[]> {
  return invoke("list_copyable_tickets", { limit });
}

export function listFiscalEvents(
  limit?: number
): Promise<Record<string, unknown>[]> {
  return invoke("list_fiscal_events", { limit });
}

// ── Refund ──────────────────────────────────────────────────────

export interface RefundableEvent {
  fiscalEventId: number;
  ticketNumber: number;
  orderId: number;
  tableNumber: number | null;
  totalCents: number;
  createdAt: string;
}

export function listRefundableEvents(limit?: number): Promise<RefundableEvent[]> {
  return invoke("list_refundable_events", { limit: limit ?? 20 });
}

export interface RefundItem {
  menuItemId: number;
  quantity: number;
  /** The original sale's line (a product may be sold at several prices). */
  lineId?: number;
}

export function refundCheckout(
  originalFiscalEventId: number,
  items: RefundItem[],
  employeeInsz: string,
  delivery?: TicketDeliveryRequest,
): Promise<CheckoutResult> {
  return invoke("refund_checkout", { originalFiscalEventId, items, employeeInsz, delivery });
}

// ── Social (Event S - ONSS clock in/out) ────────────────────────────

export interface SocialResult {
  success: boolean;
  error: string | null;
}

/** Raised by the engine when the clock-in it does itself after a login
 * was not registered (event `presence-failed`). */
export interface PresenceFailed {
  insz: string;
  clockIn: boolean;
  error: string;
}

/** End the employee's shift from the login screen: the engine checks the
 * PIN (a refused PIN is thrown, as for a login), signs the clock-out and
 * always ends the session. */
export function endShift(insz: string, pin?: string): Promise<SocialResult> {
  return invoke("end_shift", { insz, pin: pin ?? null });
}

/** Tell the engine the operator touched the screen: it owns the idle lock. */
export function userActivity(): Promise<void> {
  return invoke("user_activity");
}

export function workIn(employeeInsz: string): Promise<SocialResult> {
  return invoke("work_in", { employeeInsz });
}

export function workOut(employeeInsz: string): Promise<SocialResult> {
  return invoke("work_out", { employeeInsz });
}

/** An employee on shift: clocked in, not yet clocked out. */
export interface OpenShift {
  userInsz: string;
  /** UTC, RFC 3339. */
  startedAt: string;
}

export function listOpenShifts(): Promise<OpenShift[]> {
  return invoke("list_open_shifts");
}

// ── Financial (Event F - cash in/out, drawer) ───────────────────────

export interface FinancialResult {
  success: boolean;
  error: string | null;
}

export function moneyInOut(
  employeeInsz: string,
  amountCents: number,
  direction: "in" | "out" | "declaration",
  reason: string,
): Promise<FinancialResult> {
  return invoke("money_in_out", { employeeInsz, amountCents, direction, reason });
}

export function openDrawerFiscal(employeeInsz: string): Promise<FinancialResult> {
  return invoke("open_drawer_fiscal", { employeeInsz });
}

// ── HubRise ─────────────────────────────────────────────────────────

export interface HubriseConnectionInfo {
  connected: boolean;
  account_id: string | null;
  account_name: string | null;
  location_id: string | null;
  location_name: string | null;
  catalog_id: string | null;
  catalog_name: string | null;
  customer_list_id: string | null;
  customer_list_name: string | null;
}

export interface WebshopOrderItem {
  product_name: string;
  quantity: number;
  unit_price_cents: number;
  line_total_cents: number;
  options: string[];
  customer_notes: string | null;
  /** Chinese name of the matching menu product, if any. */
  product_name_zh: string | null;
}

export interface WebshopOrderSummary {
  id: number;
  hubrise_order_id: string | null;
  status: string;
  hubrise_status: string | null;
  source: string;
  service_type: string | null;
  total_cents: number;
  created_at: string;
  items: WebshopOrderItem[];
  customer_notes: string | null;
  customer_name: string | null;
  confirmed_time: string | null;
  customer_address_1: string | null;
  customer_address_2: string | null;
  customer_postal_code: string | null;
  customer_city: string | null;
  /** The fiscal side of the order, once resolved against the menu. */
  webshop?: WebshopState | null;
}

/** Why an order cannot be accepted as it is (`code` plus details). */
export interface WebshopAttention {
  code: string;
  [detail: string]: unknown;
}

export interface WebshopState {
  order_id: number;
  hubrise_order_id: string;
  channel: string;
  channel_kind: "platform" | "webshop" | null;
  service_type: string | null;
  payment_mode: "prepaid" | "unpaid" | null;
  payment_type: "APP" | "ONLINE" | null;
  /** pending, signing, signed_p, signed_n, voided, rejected,
   * refund_pending, cancel_pending, refunded, closed */
  fiscal_state: string;
  resolution: "ok" | "needs_review";
  attention: WebshopAttention[];
  warnings: WebshopAttention[];
}

export interface WebshopAcceptResult {
  fiscal_state: string;
  fiscal_event_id: number;
  sale: CheckoutResult | null;
  kitchen_printed: boolean;
}

/** Sign an online order: N if prepaid, P if unpaid. */
export function webshopAcceptOrder(
  orderId: number,
  confirmedTime?: string
): Promise<WebshopAcceptResult> {
  return invoke("webshop_accept_order", {
    req: { order_id: orderId, confirmed_time: confirmedTime ?? null },
  });
}

export function webshopRejectOrder(orderId: number): Promise<void> {
  return invoke("webshop_reject_order", { orderId });
}

/** Collect an unpaid order: the sale N closes its P. */
export function webshopHandoverOrder(
  orderId: number,
  payments: PaymentPart[]
): Promise<CheckoutResult> {
  return invoke("webshop_handover_order", {
    req: { order_id: orderId, payments },
  });
}

/** Hand over a prepaid order (its sale is already signed). */
export function webshopCompleteOrder(orderId: number): Promise<void> {
  return invoke("webshop_complete_order", { orderId });
}

/** A manager refunds an order cancelled after its sale. */
export function webshopRefundOrder(
  orderId: number,
  employeeInsz: string
): Promise<CheckoutResult> {
  return invoke("webshop_refund_order", {
    req: { order_id: orderId, employee_insz: employeeInsz },
  });
}

/** Close an unpaid order cancelled after its P (N at 0,00 EUR). */
export function webshopCloseCancelledOrder(
  orderId: number
): Promise<CheckoutResult> {
  return invoke("webshop_close_cancelled_order", { orderId });
}

/** One problem of a refused catalog. */
export interface CatalogIssue {
  code: string;
  key: string;
}

export interface CatalogSyncState {
  source: string;
  manages_menu: boolean;
  applied_at: string | null;
  rejected_at: string | null;
  issues: CatalogIssue[];
}

/** The health of the HubRise link, for the status light and Settings. */
export interface HubriseHealth {
  connected: boolean;
  last_poll_at: string | null;
  auth_error_at: string | null;
  last_error: string | null;
  callback_registered_at: string | null;
  last_catalog_sync_at: string | null;
  outbox: { pending: number; parked: number };
  outbox_error: string | null;
  parked_events: number;
  /** Online orders still waiting for staff after 5 minutes. */
  stale_orders: number;
  catalog: CatalogSyncState | null;
  catalog_awaiting_adoption: boolean;
}

export function hubriseStatus(): Promise<HubriseHealth> {
  return invoke("hubrise_status");
}

/** A local product that keeps its id when HubRise takes over the menu. */
export interface CatalogAdoption {
  menu_item_id: number;
  key: string;
  name: string;
  price_cents: number;
}

export function hubriseAdoptionPreview(): Promise<CatalogAdoption[]> {
  return invoke("hubrise_adoption_preview");
}

/** What applying a HubRise catalog did (`pos_catalog::ApplyOutcome`,
 * tagged by `outcome`). A refused catalog comes with its issues; the
 * menu is then untouched. */
export type CatalogApplyOutcome =
  | { outcome: "applied"; hash: string; active_items: number; deactivated_items: number }
  | { outcome: "unchanged" }
  | { outcome: "awaiting_adoption" }
  | { outcome: "rejected"; issues: CatalogIssue[] };

export function hubriseAdoptCatalog(adoptions: CatalogAdoption[]): Promise<CatalogApplyOutcome> {
  return invoke("hubrise_adopt_catalog", { adoptions });
}

/** How a HubRise channel is signed. */
export interface HubriseChannel {
  channel: string;
  kind: "platform" | "webshop";
  payment_type: "APP" | "ONLINE";
  charge_policy: "include" | "exclude";
}

export function hubriseListChannels(): Promise<HubriseChannel[]> {
  return invoke("hubrise_list_channels");
}

/** Returns how many waiting orders now resolve cleanly. */
export function hubriseSetChannel(channel: HubriseChannel): Promise<number> {
  return invoke("hubrise_set_channel", { channel });
}

/** Fetch the HubRise catalog now (after an order to check). */
export function hubriseSyncCatalog(): Promise<CatalogApplyOutcome | null> {
  return invoke("hubrise_sync_catalog");
}

export function hubriseGetConnectionStatus(): Promise<HubriseConnectionInfo> {
  return invoke("hubrise_get_connection_status");
}

/** HubRise's login page, and the same link as a QR code (SVG built by
 * the backend from the URL only): the kiosk has no browser. */
export interface HubriseAuthorizeLink {
  url: string;
  qr_svg: string;
}

export function hubriseStartOauthFlow(): Promise<HubriseAuthorizeLink> {
  return invoke("hubrise_start_oauth_flow");
}

export function hubriseCompleteOauth(
  code: string
): Promise<HubriseConnectionInfo> {
  return invoke("hubrise_complete_oauth", { code });
}

export function hubriseDisconnect(): Promise<void> {
  return invoke("hubrise_disconnect");
}

export function hubriseListWebshopOrders(): Promise<WebshopOrderSummary[]> {
  return invoke("hubrise_list_webshop_orders");
}

export function hubrisePatchOrderStatus(
  hubriseOrderId: string,
  newStatus: string
): Promise<void> {
  return invoke("hubrise_patch_order_status", {
    req: { hubrise_order_id: hubriseOrderId, new_status: newStatus },
  });
}

export function hubriseSetConfirmedTime(
  hubriseOrderId: string,
  confirmedTime: string
): Promise<void> {
  return invoke("hubrise_set_confirmed_time", {
    req: {
      hubrise_order_id: hubriseOrderId,
      confirmed_time: confirmedTime,
    },
  });
}

export function printWebshopKitchen(orderId: number): Promise<void> {
  return invoke("print_webshop_kitchen", { orderId });
}

export function printWebshopCustomer(orderId: number): Promise<void> {
  return invoke("print_webshop_customer", { orderId });
}

// Remote support (commands/support.rs)

export interface SupportStatus {
  /** RustDesk is installed on this terminal. */
  available: boolean;
  running: boolean;
  /** Name of the manager who started the running session. */
  startedBy: string | null;
  /** RFC 3339, UTC. */
  startedAt: string | null;
}

export function remoteSupportStatus(): Promise<SupportStatus> {
  return invoke("remote_support_status");
}

/** Start a session: the logged-in manager, confirmed by their PIN. */
export function startRemoteSupport(insz: string, pin: string): Promise<SupportStatus> {
  return invoke("start_remote_support", { insz, pin });
}

/** Whether a session runs, for the login screen (no login needed). */
export interface SupportRunning {
  running: boolean;
  /** RFC 3339, UTC. */
  startedAt: string | null;
}

export function remoteSupportRunning(): Promise<SupportRunning> {
  return invoke("remote_support_running");
}

/** Anyone may stop a session, logged in or not. */
export function stopRemoteSupport(): Promise<SupportStatus> {
  return invoke("stop_remote_support");
}

/** One change of the HubRise switch (`integration_switch_log`). */
export interface SwitchChange {
  enabled: boolean;
  /** `config`: pos.json; `admin`: Settings. */
  source: "config" | "admin";
  changed_by: string | null;
  changed_at: string;
}

/** Whether the HubRise integration runs on this installation. */
export interface Integrations {
  hubrise_enabled: boolean;
  /** Online orders still to settle (the board stays reachable). */
  online_orders_open: number;
  hubrise_log: SwitchChange[];
}

export function getIntegrations(): Promise<Integrations> {
  return invoke("get_integrations");
}

/** Admins only; switching off waits until every online order is settled. */
export function setHubriseEnabled(enabled: boolean): Promise<Integrations> {
  return invoke("set_hubrise_enabled", { enabled });
}

/** One change of the table plan. */
export interface TablePlanChange {
  plan: string;
  source: "config" | "admin";
  changed_by: string | null;
  changed_at: string;
}

/** The table numbers in use, as ranges ("1-12, 20-25"). */
export interface TablePlan {
  plan: string;
  table_count: number;
  log: TablePlanChange[];
}

export function getTablePlan(): Promise<TablePlan> {
  return invoke("get_table_plan");
}

/** Managers only. A table left out that still has an open order stays on the Tables screen. */
export function setTablePlan(plan: string): Promise<TablePlan> {
  return invoke("set_table_plan", { plan });
}
