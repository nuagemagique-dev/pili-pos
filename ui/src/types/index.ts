export type VatCode = "A" | "B" | "C" | "D" | "X";

export type OrderType = "dine_in" | "takeaway";

export type OrderStatus = "open" | "closed" | "voided";

export type UserRole = "admin" | "cashier" | "waiter";

export type CheckoutPaymentMethod = "cash" | "card";

export interface Category {
  id: number;
  name: string;
  sort_order: number;
  color: string | null;
  active: boolean;
}

export interface MenuItem {
  id: number;
  category_id: number;
  name: string;
  price_cents: number;
  vat_code: VatCode;
  vat_code_takeaway: VatCode | null;
  /** Free-price product: the operator enters the price when adding it. */
  open_price: boolean;
  kitchen_printer: boolean;
  sort_order: number;
  active: boolean;
}

export interface Order {
  id: number;
  table_number: number | null;
  order_type: OrderType;
  status: OrderStatus;
  user_insz: string;
  created_at: string;
  closed_at: string | null;
  subtotal_cents: number;
  total_cents: number;
}

export interface OrderLine {
  id: number;
  order_id: number;
  menu_item_id: number;
  quantity: number;
  unit_price_cents: number;
  vat_code: VatCode;
  name: string;
  modifiers_json: string | null;
  created_at: string;
  department_id: string;
  department_name: string;
}

export interface User {
  insz: string;
  name: string;
  role: UserRole;
  active: boolean;
  has_pin: boolean;
}

export interface CreateOrderInput {
  order_type: OrderType;
  table_number: number | null;
  user_insz: string;
}

/** Price, VAT label, name and department are resolved by the backend
 * from the catalogue - the UI only picks the product. */
export interface AddOrderLineInput {
  order_id: number;
  menu_item_id: number;
  quantity: number;
  modifiers_json: string | null;
  /** Only for free-price products. */
  unit_price_cents?: number;
}

export interface FiscalTicket {
  restaurant_name: string;
  restaurant_address: string;
  restaurant_vat: string;
  pos_id: string;
  terminal_id: string;
  pos_datetime: string;
  ticket_number: number;
  event_label: string;
  is_training: boolean;
  lines: TicketLine[];
  subtotal: number;
  total: number;
  vat_breakdown: VatCalcItem[];
  payments: TicketPayment[];
  rounding: number | null;
  fdm_ref: FdmReference | null;
  short_signature: string | null;
  verification_url: string | null;
  footer: string[];
  ticket_type_label: string;
  is_refund: boolean;
  /** Language of the printed fiscal mentions (FR/NL/DE). */
  lang: 'fr' | 'nl' | 'de';
}

export interface TicketLine {
  quantity: number;
  description: string;
  unit_price: number;
  total: number;
  vat_code: string;
}

export interface TicketPayment {
  method: string;
  amount: number;
}

export interface VatCalcItem {
  label: string;
  rate: number;
  taxable_amount: number;
  vat_amount: number;
  total_amount: number;
  out_of_scope: boolean;
}

export interface FdmReference {
  fdm_id: string;
  fdm_date_time: string;
  event_label: string;
  event_counter: number;
  total_counter: number;
}

export interface CheckoutResult {
  success: boolean;
  ticket: FiscalTicket | null;
  error: string | null;
  fdm_connected: boolean;
  /** Archive id of the signed event, when successful. */
  fiscal_event_id: number | null;
  /** How the ticket reached the customer (paper and/or email). */
  delivery: DeliveryOutcome | null;
}

// ── Ticket delivery (paper / email / both) ─────────────────────────

/** The medium the operator picked on screen. Sent to the backend. */
export type TicketMediumRequest = "paper" | "digital" | "paper_digital";

/** The operator's choice, sent with checkout, refund and copy. */
export interface TicketDeliveryRequest {
  medium: TicketMediumRequest;
  /** Required unless `medium` is "paper". */
  email?: string;
}

/** The medium actually signed, as returned by the backend. */
export type TicketMedium = "NONE" | "PAPER" | "DIGITAL" | "PAPER_DIGITAL";

/** Email outcome shown on the "done" screen. */
export type EmailDeliveryStatus = "not_requested" | "sent" | "queued";

/** What happened to a ticket after checkout, refund or copy. */
export interface DeliveryOutcome {
  fiscalEventId: number;
  deliveryId: number | null;
  medium: TicketMedium;
  email: EmailDeliveryStatus;
  recipientHint: string | null;
  paperPrinted: boolean;
  /** The customer should have a paper ticket but none printed yet. */
  needsPaper: boolean;
}

/** Email availability for the ticket picker. */
export interface MailStatus {
  configured: boolean;
  reachable: boolean;
  allowDigitalOnly: boolean;
}

/** A delivery as listed in Settings. Never carries the address itself. */
export interface DeliveryView {
  id: number;
  fiscalEventId: number;
  medium: string;
  recipientHint: string;
  status: "pending" | "sending" | "sent" | "failed" | "fallback_printed" | "purged";
  attempts: number;
  lastError: string | null;
  fallbackPrinted: boolean;
  createdAt: string;
}
