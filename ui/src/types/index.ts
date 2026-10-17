export type VatCode = "A" | "B" | "C" | "D" | "X";

export type OrderType = "dine_in" | "takeaway";

export type OrderStatus = "open" | "closed" | "voided";

export type UserRole = "admin" | "cashier" | "waiter";

export type CheckoutPaymentMethod = "cash" | "card" | "meal_voucher";

/** One part of a sale's payment. Cash has no amount: it pays the rest. */
export interface PaymentPart {
  method: CheckoutPaymentMethod;
  amount_cents?: number;
  /** A card part taken on the linked terminal: the approved payment it settles (`EftPayment.id`). */
  eft_payment_id?: number;
}

/** Where a card payment asked of the linked terminal stands. */
export type EftStatus = "sent" | "approved" | "declined" | "cancelled" | "unknown" | "not_sent";

/** A card payment asked of the linked terminal (commands `eft_pay`, `eft_check`, `list_eft_payments`). */
export interface EftPayment {
  id: number;
  order_id: number;
  amount_cents: number;
  status: EftStatus;
  /** Set once approved. */
  card_type: "debit" | "credit" | "unknown" | null;
  brand: string | null;
  terminal_reference: string | null;
  authorisation_code: string | null;
  masked_pan: string | null;
  /** Why it was declined, what is missing for an unknown one. */
  detail: string | null;
  created_at: string;
  /** A sale has settled it. */
  used: boolean;
}

/** A card payment the terminal approved that no sale has settled yet (command `list_unsettled_card_payments`). */
export interface UnsettledCardPayment {
  payment_id: number;
  order_id: number;
  order_type: "dine_in" | "takeaway";
  /** The table of a dine-in order. */
  table_number: number | null;
  amount_cents: number;
  brand: string | null;
  created_at: string;
}

/** Whether the linked card terminal answers (command `check_payment_terminal`). */
export interface PaymentTerminalStatus {
  configured: boolean;
  /** The driver, as in pos.json; null when not configured. */
  kind: string | null;
  /** The terminal answered (or a payment is running on it). */
  connected: boolean;
  /** A payment holds the terminal's line: it was not probed. */
  busy: boolean;
  /** Why it is not connected. */
  detail: string | null;
}

export interface Category {
  id: number;
  name: string;
  /** French and Chinese names, whatever the UI language. */
  name_fr?: string | null;
  name_zh?: string | null;
  sort_order: number;
  color: string | null;
  active: boolean;
}

export interface MenuItem {
  id: number;
  category_id: number;
  name: string;
  /** French and Chinese names, whatever the UI language. */
  name_fr?: string | null;
  name_zh?: string | null;
  /** Menu code ("A2") from the catalog, when it gives one. */
  code?: string | null;
  price_cents: number;
  vat_code: VatCode;
  vat_code_takeaway: VatCode | null;
  /** Free-price product: the operator enters the price when adding it. */
  open_price: boolean;
  kitchen_printer: boolean;
  sort_order: number;
  active: boolean;
  /** Sold out right now ("Épuisé"): the till refuses to add it to an order. */
  sold_out?: boolean;
  /** End of the business day it is sold out until (RFC 3339); null with `sold_out`: until re-enabled. */
  sold_out_until?: string | null;
  /** The product has an active option group with an active option: the picker opens (V11-D18). */
  has_modifiers?: boolean;
}

/** A product that is sold out now (`list_sold_out`). */
export interface SoldOutItem {
  menu_item_id: number;
  name: string;
  /** End of the business day (RFC 3339); null: until re-enabled. */
  until: string | null;
  set_by: string;
  set_at: string;
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

/** A cart line: the order line, its product's French and Chinese
 * names, and the quantity not yet sent to the bar / kitchen. */
export interface CartLine extends OrderLine {
  name_fr: string | null;
  name_zh: string | null;
  code?: string | null;
  unsent_quantity: number;
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
  /** Control data of the sale a refund reverses, or of the event a
   * copy reproduces ("Informations FDM d'origine"). */
  original?: { fdm_ref: FdmReference; short_signature: string | null } | null;
  /** Copies only: when the copy was made. */
  copy_made_at?: string | null;
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

/** camelCase, like FdmReference: the FDM protocol's own shape. */
export interface VatCalcItem {
  label: string;
  rate: number;
  taxableAmount: number;
  vatAmount: number;
  totalAmount: number;
  outOfScope: boolean;
}

/** The FDM's own reference, as it signed it: camelCase like the FDM
 * protocol (pos_fiscal::response::FdmReference), unlike the rest of
 * the IPC. */
export interface FdmReference {
  fdmId: string;
  fdmDateTime: string;
  eventLabel: string;
  eventCounter: number;
  totalCounter: number;
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
  /** The bar / kitchen slip of the lines not yet sent (sales only). */
  kitchen_slip?: SlipOutcome | null;
}

/** What became of the bar / kitchen order slip. */
export type SlipOutcome = "printed" | "nothing_to_print" | "no_printer" | "failed";

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

/** The nightly export as the manager sees it (Settings). Times are
 * RFC 3339 UTC; show them through utils/datetime. */
export interface ExportStatus {
  enabled: boolean;
  /** Why the configuration is unusable, when it is. */
  problem: string | null;
  endpoint: string | null;
  keepDays: number;
  lastExportId: string | null;
  lastPreparedAt: string | null;
  lastUploadedId: string | null;
  lastUploadedAt: string | null;
  /** Exports prepared and not uploaded yet. */
  pending: number;
  nextAttemptAt: string | null;
  lastError: string | null;
  lastErrorAt: string | null;
  /** Exports kept on the device. */
  kept: number;
  /** Exports dropped after the keep window. */
  dropped: number;
}
