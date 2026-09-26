import { invoke } from "@tauri-apps/api/core";
import type {
  Category,
  MenuItem,
  Order,
  OrderLine,
  User,
  CreateOrderInput,
  AddOrderLineInput,
  CheckoutPaymentMethod,
  CheckoutResult,
  FiscalTicket,
  TicketDeliveryRequest,
  DeliveryOutcome,
  MailStatus,
  DeliveryView,
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

export function setPin(insz: string, newPin: string): Promise<void> {
  return invoke("set_pin", { insz, newPin });
}

export function listUsers(): Promise<User[]> {
  return invoke("list_users");
}

export function createUser(input: {
  insz: string;
  name: string;
  role: string;
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

// Checkout
export function checkout(
  orderId: number,
  paymentMethod: CheckoutPaymentMethod,
  delivery?: TicketDeliveryRequest
): Promise<CheckoutResult> {
  return invoke("checkout", { orderId, paymentMethod, delivery });
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

export function printMissingOriginal(
  fiscalEventId: number,
  ticket: FiscalTicket
): Promise<void> {
  return invoke("print_missing_original", { fiscalEventId, ticket });
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
}

export function listTables(tableCount: number): Promise<TableInfo[]> {
  return invoke("list_tables", { tableCount });
}

/** Register (event P) what was ordered at a table since the last
 * registration. No-op when nothing changed. */
export function signTableOrder(orderId: number): Promise<void> {
  return invoke("sign_table_order", { orderId });
}

export function signPreBill(orderId: number): Promise<void> {
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

// ── Financial (Event F - cash in/out, drawer) ───────────────────────

export interface FinancialResult {
  success: boolean;
  error: string | null;
}

export function moneyInOut(
  employeeInsz: string,
  amountCents: number,
  direction: "in" | "out",
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
}

export function hubriseGetConnectionStatus(): Promise<HubriseConnectionInfo> {
  return invoke("hubrise_get_connection_status");
}

export function hubriseStartOauthFlow(): Promise<string> {
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
