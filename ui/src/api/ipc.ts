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
export function printReceipt(ticket: FiscalTicket): Promise<void> {
  return invoke("print_receipt", { ticket });
}

export function openDrawer(): Promise<void> {
  return invoke("open_drawer");
}

export function checkPrinter(): Promise<boolean> {
  return invoke("check_printer");
}

// Checkout
export function checkout(
  orderId: number,
  paymentMethod: CheckoutPaymentMethod
): Promise<CheckoutResult> {
  return invoke("checkout", { orderId, paymentMethod });
}

export function checkFdmStatus(): Promise<unknown> {
  return invoke("check_fdm_status");
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
}

export function getSettings(): Promise<PosSettings> {
  return invoke("get_settings");
}

export function setTrainingMode(enabled: boolean): Promise<boolean> {
  return invoke("set_training_mode", { enabled });
}

export function newBookingPeriod(): Promise<string> {
  return invoke("new_booking_period");
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

export function signTableOrder(
  orderId: number,
  costCenterRef: string
): Promise<void> {
  return invoke("sign_table_order", { orderId, costCenterRef });
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
}

export function copyTicket(
  fiscalEventId: number,
  employeeInsz: string
): Promise<CopyResult> {
  return invoke("copy_ticket", { fiscalEventId, employeeInsz });
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
  unitPriceCents: number;
  name: string;
  vatCode: string;
}

export function refundCheckout(
  originalFiscalEventId: number,
  items: RefundItem[],
  employeeInsz: string,
): Promise<CheckoutResult> {
  return invoke("refund_checkout", { originalFiscalEventId, items, employeeInsz });
}

// ── Social (Event S — ONSS clock in/out) ────────────────────────────

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

// ── Financial (Event F — cash in/out, drawer) ───────────────────────

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
