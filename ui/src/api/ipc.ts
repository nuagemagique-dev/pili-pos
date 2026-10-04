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
  CheckoutResult,
  FiscalTicket,
  TicketDeliveryRequest,
  DeliveryOutcome,
  MailStatus,
  DeliveryView,
  SlipOutcome,
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

/** Close an order the customer left without paying (manager, PIN):
 * the sale is signed with no payment line (UC4109). */
export function closeUnpaid(orderId: number, insz: string, pin: string): Promise<CheckoutResult> {
  return invoke("close_unpaid", { orderId, insz, pin });
}

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

// Settings
export interface PosSettings {
  trainingMode: boolean;
  printerAddress: string | null;
  fdmUrl: string;
  posId: string;
  terminalId: string;
  restaurantName: string;
  language: string;
  /** What the FDM would refuse in pos.json; empty when usable. */
  configProblems: string[];
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

export function hubriseAdoptCatalog(adoptions: CatalogAdoption[]): Promise<unknown> {
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
export function hubriseSyncCatalog(): Promise<unknown> {
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
