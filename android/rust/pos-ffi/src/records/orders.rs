//! Records of the orders facade (PD56): orders, lines, the cart, the menu, the tables and the
//! session answers, plus the two records of the `vat_split` helper (PD62).
//!
//! Owner: the orders domain (order, modifiers, menu, tables, session). Each type mirrors an engine
//! type: same field names, same serde attributes, `deny_unknown_fields` on records. Shared types
//! (`VatCode`, `SlipOutcome`) come from [`super::common`].
//!
//! Types not in the T4 table (`records/mod.rs`):
//!
//! - `Modifier`: an option of a [`ModifierGroup`] (`pos_db::queries::menu::Modifier`).
//! - The table lists `ModifierGroup` as `pos_core::menu::ModifierGroup`, but `get_item_modifiers`
//!   returns `pos_db::queries::menu::ModifierGroup` (`id`, `name`, `min`, `max`, `options`); that is
//!   the shape mirrored here. The `pos_core` one is not reachable from a command.
//! - `VatLine` and `VatSplitRow`: the argument and the answer of the `vat_split` helper; they
//!   mirror no engine type.
//!
//! `CartLine` flattens an `OrderLine` in the engine (`#[serde(flatten)]`). serde cannot combine
//! `flatten` with `deny_unknown_fields`, so the record lists the `OrderLine` fields itself; the
//! serde form is the same.

use super::common::{SlipOutcome, VatCode};

/// `pos_core::order::OrderType`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    DineIn,
    Takeaway,
}

/// `pos_core::order::OrderStatus`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Open,
    Closed,
    Voided,
}

/// `pos_core::order::Order`. Times are RFC 3339 text.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Order {
    pub id: i64,
    pub table_number: Option<i32>,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub user_insz: String,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub subtotal_cents: i64,
    pub total_cents: i64,
}

/// `pos_core::order::OrderLine`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct OrderLine {
    pub id: i64,
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub vat_code: VatCode,
    pub name: String,
    pub modifiers_json: Option<String>,
    pub created_at: String,
    /// Department sent to the FDM: the product's category, captured when the line is added.
    #[serde(default)]
    pub department_id: String,
    #[serde(default)]
    pub department_name: String,
}

/// `pos_engine::commands::order::CreateOrderInput`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct CreateOrderInput {
    pub order_type: OrderType,
    pub table_number: Option<i32>,
    pub user_insz: String,
}

/// `pos_engine::commands::order::AddOrderLineInput`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AddOrderLineInput {
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub modifiers_json: Option<String>,
    /// Required for free-price products, refused for the others.
    #[serde(default)]
    pub unit_price_cents: Option<i64>,
}

/// `pos_db::queries::orders::CartLine`: an order line as the cart shows it. The engine flattens
/// an `OrderLine` into it; here its fields are listed (see the module comment).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct CartLine {
    pub id: i64,
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub vat_code: VatCode,
    pub name: String,
    pub modifiers_json: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub department_id: String,
    #[serde(default)]
    pub department_name: String,
    pub name_fr: Option<String>,
    pub name_zh: Option<String>,
    /// The product's menu code ("A2"), when the catalog gives one.
    pub code: Option<String>,
    /// The quantity not yet sent to the bar or the kitchen.
    pub unsent_quantity: i64,
    /// Σ of the line's price changes in cents (negative for a discount).
    #[serde(default)]
    pub discount_cents: i64,
    /// The line's consolidated price changes (a till discount, an online order's).
    #[serde(default)]
    pub price_changes: Vec<CartPriceChange>,
    /// The note for the kitchen ("sans wasabi"), local only.
    #[serde(default)]
    pub kitchen_note: Option<String>,
}

/// `pos_db::queries::orders::CartPriceChange`: a price change on a cart line.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct CartPriceChange {
    pub id: String,
    pub name: String,
    /// `LINE` or `EVENT`.
    pub scope: String,
    pub amount_cents: i64,
}

/// `pos_engine::commands::discount::DiscountTarget`: what a discount applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Enum)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DiscountTarget {
    /// The whole order, spread over its lines.
    Order,
    /// One line, for free.
    Line { line_id: i64 },
}

/// `pos_engine::commands::discount::DiscountKind`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum DiscountKind {
    /// An integer percent, 1 to 99.
    Percent,
    /// An amount in cents, 1 to the net total.
    Amount,
    /// Everything ("Offert").
    Free,
}

/// `pos_engine::commands::discount::DiscountReason`: why the manager gives it (audit only).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum DiscountReason {
    CommercialGesture,
    StaffMeal,
    ServiceIssue,
    Other,
}

/// `pos_engine::commands::discount::DiscountInput`: a discount to apply.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct DiscountInput {
    pub target: DiscountTarget,
    pub kind: DiscountKind,
    /// The percent or the amount in cents; ignored for `Free`.
    #[serde(default)]
    pub value: Option<i64>,
    pub reason_code: DiscountReason,
    #[serde(default)]
    pub reason_text: Option<String>,
}

/// `pos_engine::commands::discount::DiscountResult`: what a discount command did.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscountResult {
    pub order: Order,
    /// The kitchen slip of the entries the immediate P registered.
    pub kitchen_slip: SlipOutcome,
    /// Set when the immediate P could not be signed: the discount is recorded and goes with the
    /// next registration or the sale.
    pub registration_error: Option<String>,
}

/// `pos_core::menu::Category`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub name_fr: Option<String>,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub sort_order: i32,
    pub color: Option<String>,
    pub active: bool,
}

/// `pos_core::menu::MenuItem`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    #[serde(default)]
    pub name_fr: Option<String>,
    #[serde(default)]
    pub name_zh: Option<String>,
    /// The menu code ("A2") from a mirrored catalog; display only.
    #[serde(default)]
    pub code: Option<String>,
    pub price_cents: i64,
    pub vat_code: VatCode,
    /// VAT label for takeaway sales; `None` means the dine-in label applies.
    #[serde(default)]
    pub vat_code_takeaway: Option<VatCode>,
    /// Free-price product: the operator enters the unit price.
    #[serde(default)]
    pub open_price: bool,
    pub kitchen_printer: bool,
    pub sort_order: i32,
    pub active: bool,
}

/// `pos_db::queries::menu::Modifier`: an option of a group.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Modifier {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
}

/// `pos_db::queries::menu::ModifierGroup`: an option group of a product.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct ModifierGroup {
    pub id: i64,
    pub name: String,
    pub min: i64,
    /// `None`: no maximum.
    pub max: Option<i64>,
    pub options: Vec<Modifier>,
}

/// `pos_engine::commands::modifiers::AddWithModifiersInput`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AddWithModifiersInput {
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub modifier_ids: Vec<i64>,
}

/// `pos_engine::commands::tables::TableInfo`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableInfo {
    pub table_number: i32,
    pub order_id: Option<i64>,
    pub total_cents: i64,
    pub line_count: i64,
    pub status: String,
    /// When the table's order was opened (UTC, RFC 3339).
    pub opened_at: Option<String>,
    /// A provisional bill was printed and the order has not changed since.
    pub pre_bill_printed: bool,
}

/// `pos_engine::commands::table_transfer::TableTransferResult`: what moving an order to another
/// table did (the kitchen slip of what was registered first, the transfer notice).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableTransferResult {
    pub order_id: i64,
    pub from_table: i32,
    pub to_table: i32,
    pub kitchen_slip: SlipOutcome,
    pub transfer_slip: SlipOutcome,
}

/// `pos_engine::commands::tables::TakeawayInfo`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeawayInfo {
    pub order_id: i64,
    pub total_cents: i64,
    pub line_count: i64,
    pub created_at: String,
}

/// `pos_engine::commands::session::LeaveReason`: why the operator leaves an order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum LeaveReason {
    /// To another screen.
    Navigate,
    /// To another order.
    SwitchOrder,
    /// The order is sent to the kitchen and stays on screen.
    SendToKitchen,
    /// The operator locks the screen.
    Lock,
    /// The screen locks itself after the idle delay.
    Idle,
}

/// `pos_engine::commands::session::LockOutcome`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockOutcome {
    pub order_error: Option<String>,
}

/// One line of the `vat_split` input: the VAT code, the unit price and the quantity.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct VatLine {
    pub vat_code: VatCode,
    pub unit_price_cents: i64,
    pub quantity: i64,
}

/// One row of the `vat_split` answer: a VAT code present in the lines.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct VatSplitRow {
    pub vat_code: VatCode,
    /// 21, 12, 6 or 0.
    pub rate_percent: i64,
    /// Sum of `unit_price_cents * quantity` of the lines with this code, VAT included.
    pub total_incl_cents: i64,
    /// The VAT included in the total, rounded as `VatSummary.vue` does.
    pub vat_cents: i64,
}

/// `pos_db::queries::table_plan::PlanChange`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct TablePlanChange {
    pub plan: String,
    /// `config` (pos.json) or `admin` (a manager).
    pub source: String,
    pub changed_by: Option<String>,
    pub changed_at: String,
}

/// `pos_engine::commands::table_plan::TablePlan`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct TablePlan {
    /// The table numbers as ranges, canonical form ("1-12, 20-25").
    pub plan: String,
    pub table_count: i64,
    /// The latest changes first.
    pub log: Vec<TablePlanChange>,
}
