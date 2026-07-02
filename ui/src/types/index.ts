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

export interface AddOrderLineInput {
  order_id: number;
  menu_item_id: number;
  quantity: number;
  unit_price_cents: number;
  vat_code: VatCode;
  name: string;
  modifiers_json: string | null;
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
}
