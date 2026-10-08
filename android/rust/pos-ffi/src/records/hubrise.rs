//! Records of the HubRise and webshop facade (PD56): the connection, the online-order board, the
//! catalog sync and the signing of online orders.
//!
//! Owner: the `hubrise` domain (engine modules hubrise, webshop). Each type mirrors an engine type:
//! same field names, same serde attributes, `deny_unknown_fields` on records. Shared types
//! (`CheckoutResult`, `PaymentPart`, `TicketDeliveryRequest`, `VatCode`) come from [`super::common`].
//!
//! Types of the T4 table defined here: `AcceptRequest`, `AcceptResult`, `Adoption`, `ApplyOutcome`,
//! `Attention`, `AuthorizeLink`, `Backlog`, `Channel`, `ConnectionHealth`, `HandoverRequest`,
//! `HubriseConnectionInfo`, `HubriseStatus`, `Issue`, `IssueCode`, `LineDiscount`, `LineKind`,
//! `PatchStatusRequest`, `RefundRequest` (the webshop one), `ResolvedLine`,
//! `SetConfirmedTimeRequest`, `SyncState`, `Warning`, `WebshopOrderItem`, `WebshopOrderSummary`
//! and `WebshopState`. No type outside the table is needed.
//!
//! One type is `// contract: custom`: [`HubriseStatus`]. The engine flattens `ConnectionHealth`
//! into it (`#[serde(flatten)]`). serde cannot combine `flatten` with `deny_unknown_fields`, so
//! the record carries the same `#[serde(flatten)]` on a `health` field and no
//! `deny_unknown_fields`: it reads the engine's JSON (health keys at the top level) but would
//! ignore a key the engine added. The contract test compares the engine's JSON with the record's
//! for a real status, so a new engine field fails the test instead of being dropped silently.
//!
//! [`ApplyOutcome`] goes through `convert::out` like the other types: since the engine's
//! `Rejected` became a struct variant (PD71), every variant has a serde form, the same on both
//! sides.
//!
//! `Value`, chrono and unsigned fields do not occur in these types except the `usize` counts of
//! [`ApplyOutcome::Applied`], which become `i64`.

use super::common::{CheckoutResult, PaymentPart, TicketDeliveryRequest, VatCode};
use crate::error::PosError;

// ───────────────────────────── Connection ─────────────────────────────

/// `pos_engine::commands::hubrise::HubriseConnectionInfo`: what the screens know about the
/// current HubRise connection.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct HubriseConnectionInfo {
    pub connected: bool,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub location_id: Option<String>,
    pub location_name: Option<String>,
    pub catalog_id: Option<String>,
    pub catalog_name: Option<String>,
    pub customer_list_id: Option<String>,
    pub customer_list_name: Option<String>,
}

/// `pos_engine::commands::hubrise::AuthorizeLink`: the HubRise page where the installer grants
/// access, and the same link as a QR code.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AuthorizeLink {
    pub url: String,
    /// SVG markup, rendered from `url` only.
    pub qr_svg: String,
}

// ───────────────────────────── Online orders board ─────────────────────────────

/// `pos_engine::commands::hubrise::WebshopOrderItem`: one line of an online order, flattened for
/// the board.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct WebshopOrderItem {
    pub product_name: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
    /// Names of the applied options; empty when the item has none.
    pub options: Vec<String>,
    pub customer_notes: Option<String>,
    /// Chinese name of the menu product whose French name matches, when there is one.
    pub product_name_zh: Option<String>,
}

/// `pos_engine::commands::hubrise::WebshopOrderSummary`: an online order kept locally, for the
/// board.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct WebshopOrderSummary {
    pub id: i64,
    pub hubrise_order_id: Option<String>,
    pub status: String,
    pub hubrise_status: Option<String>,
    pub source: String,
    pub service_type: Option<String>,
    pub total_cents: i64,
    pub created_at: String,
    pub items: Vec<WebshopOrderItem>,
    pub customer_notes: Option<String>,
    pub customer_name: Option<String>,
    /// HubRise `confirmed_time` (RFC 3339).
    pub confirmed_time: Option<String>,
    pub customer_address_1: Option<String>,
    pub customer_address_2: Option<String>,
    pub customer_postal_code: Option<String>,
    pub customer_city: Option<String>,
    /// How the order resolved against the menu and where it stands fiscally; `None` for rows
    /// stored before migration 22.
    pub webshop: Option<WebshopState>,
}

/// `pos_hubrise::webshop::WebshopState`: the stored state of an online order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct WebshopState {
    pub order_id: i64,
    pub hubrise_order_id: String,
    pub channel: String,
    pub channel_kind: Option<String>,
    pub service_type: Option<String>,
    pub payment_mode: Option<String>,
    pub payment_type: Option<String>,
    pub fiscal_state: String,
    pub resolution: String,
    pub attention: Vec<Attention>,
    pub warnings: Vec<Warning>,
    pub lines: Vec<ResolvedLine>,
}

/// `pos_hubrise::webshop::Attention`: why an order cannot be accepted as it is.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Enum)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Attention {
    /// No channel row for the order's channel.
    UnknownChannel {
        channel: String,
    },
    MissingServiceType,
    NoItems,
    /// An item without `sku_ref`: nothing to match in the menu.
    ItemWithoutRef {
        name: String,
    },
    /// No active product with that key: the catalog may be behind.
    UnknownProduct {
        sku_ref: String,
    },
    UnknownOption {
        sku_ref: String,
        option_ref: String,
    },
    UnknownCharge {
        charge_ref: String,
    },
    /// A quantity that is not a whole number.
    FractionalQuantity {
        sku_ref: String,
    },
    /// A price HubRise sent that is not euro cents.
    BadPrice {
        at: String,
    },
    /// The order's VAT rate disagrees with the POS menu's label.
    VatMismatch {
        at: String,
        order: String,
        menu: VatCode,
    },
    /// A discount larger than the products it applies to.
    Discount {
        name: String,
    },
    /// The lines do not add up to the order total.
    TotalMismatch {
        total_cents: i64,
        lines_cents: i64,
    },
    /// Neither fully paid online nor unpaid / cash only.
    PaymentUnclear {
        paid_cents: i64,
        total_cents: i64,
    },
    /// HubRise changed what was sold after the POS signed it.
    ChangedAfterSigning,
}

/// `pos_hubrise::webshop::Warning`: worth showing, not blocking.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Enum)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Warning {
    /// Charged another price than the catalog's (platform markup).
    PriceDiffers {
        at: String,
        catalog_cents: i64,
        charged_cents: i64,
    },
}

/// `pos_hubrise::webshop::LineKind`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    Product,
    Charge,
}

/// `pos_hubrise::webshop::ResolvedLine`: a line to sign (menu product, printed name, charged price
/// and the menu's VAT label).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct ResolvedLine {
    pub kind: LineKind,
    pub menu_item_id: i64,
    pub name: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub vat_code: VatCode,
    pub department_id: String,
    pub department_name: String,
    /// The order's discounts, each spread over the product lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discounts: Vec<LineDiscount>,
}

/// `pos_hubrise::webshop::LineDiscount`: a line's share of an order discount.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct LineDiscount {
    /// The discount's rank in the order, from 1.
    pub grouping_id: i32,
    pub id: String,
    pub name: String,
    /// Negative.
    pub amount_cents: i64,
}

/// `pos_engine::commands::hubrise::PatchStatusRequest`: a status change staff set from the board.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct PatchStatusRequest {
    pub hubrise_order_id: String,
    pub new_status: String,
}

/// `pos_engine::commands::hubrise::SetConfirmedTimeRequest`: a new ready-at time for an order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SetConfirmedTimeRequest {
    pub hubrise_order_id: String,
    /// RFC 3339 UTC timestamp ("2026-04-11T11:30:00Z").
    pub confirmed_time: String,
}

// ───────────────────────────── Status, catalog, channels ─────────────────────────────

/// `pos_hubrise::token_store::ConnectionHealth`: what the status screen shows about the
/// connection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct ConnectionHealth {
    pub connected: bool,
    pub last_poll_at: Option<String>,
    pub auth_error_at: Option<String>,
    pub last_error: Option<String>,
    pub callback_registered_at: Option<String>,
    pub last_catalog_sync_at: Option<String>,
}

/// `pos_hubrise::outbox::Backlog`: rows waiting to be sent, and rows parked on an error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Backlog {
    pub pending: i64,
    pub parked: i64,
}

/// `pos_engine::commands::hubrise::HubriseStatus`: what the status lights and the settings panel
/// show. The engine flattens `ConnectionHealth` into it: see the module doc.
// contract: custom
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct HubriseStatus {
    #[serde(flatten)]
    pub health: ConnectionHealth,
    pub outbox: Backlog,
    pub outbox_error: Option<String>,
    /// Events HubRise sent that could not be handled.
    pub parked_events: i64,
    /// Online orders still waiting for staff after 5 minutes.
    pub stale_orders: i64,
    /// The last catalog sync: when, and the issues of a refused one.
    pub catalog: Option<SyncState>,
    /// A first catalog waits for the manager to adopt local products.
    pub catalog_awaiting_adoption: bool,
}

/// `pos_catalog::apply::SyncState`: what Settings shows about a source's catalog.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SyncState {
    pub source: String,
    pub manages_menu: bool,
    pub applied_at: Option<String>,
    pub rejected_at: Option<String>,
    pub issues: Vec<Issue>,
}

/// `pos_catalog::validate::Issue`: one problem of a catalog, with the key of the element at fault.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    pub code: IssueCode,
    /// The key of the category, item, group or option at fault; empty for the catalog itself.
    pub key: String,
}

/// `pos_catalog::validate::IssueCode`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum IssueCode {
    EmptyKey,
    DuplicateKey,
    UnknownCategory,
    MissingName,
    NameTooLong,
    UnknownLanguage,
    NegativePrice,
    MinAboveMax,
    NothingToSell,
    BadPrice,
    UnknownVatRate,
    VatMismatch,
    MissingVat,
    UnknownOptionList,
}

/// `pos_catalog::apply::Adoption`: a local product that matches a catalog item and can take its
/// key, keeping its id, before the first apply.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Adoption {
    pub menu_item_id: i64,
    pub key: String,
    pub name: String,
    pub price_cents: i64,
}

/// `pos_catalog::apply::ApplyOutcome`: what applying a catalog did, tagged by `outcome`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Enum)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ApplyOutcome {
    /// The menu now mirrors the catalog.
    Applied {
        hash: String,
        active_items: i64,
        /// Items on sale before and no longer.
        deactivated_items: i64,
    },
    /// Same catalog as the last applied one: nothing written.
    Unchanged,
    /// First sync over a menu entered by hand: the catalog waits for the manager's adoption.
    AwaitingAdoption,
    /// The catalog has issues: the menu is untouched.
    Rejected { issues: Vec<Issue> },
}

/// A count of the engine (`usize`) as the record's `i64`.
pub(crate) fn count(n: usize) -> Result<i64, PosError> {
    i64::try_from(n).map_err(|e| {
        let detail = format!("contract: count {n}: {e}");
        tracing::error!(%detail, "FFI contract failure");
        PosError::Internal { detail }
    })
}

/// `pos_hubrise::webshop::Channel`: how a HubRise channel is signed (Settings).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub channel: String,
    /// `platform` or `webshop`.
    pub kind: String,
    /// `APP` or `ONLINE`.
    pub payment_type: String,
    /// `include` or `exclude`.
    pub charge_policy: String,
}

// ───────────────────────────── Signing online orders ─────────────────────────────

/// `pos_engine::commands::webshop::AcceptRequest`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AcceptRequest {
    pub order_id: i64,
    /// The time the customer is told (RFC 3339), sent to HubRise with `accepted`.
    #[serde(default)]
    pub confirmed_time: Option<String>,
}

/// `pos_engine::commands::webshop::AcceptResult`: what acceptance did.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AcceptResult {
    /// `signed_n` (prepaid) or `signed_p` (unpaid, to be collected).
    pub fiscal_state: String,
    pub fiscal_event_id: i64,
    /// The sale, for a prepaid order: its ticket and delivery.
    pub sale: Option<CheckoutResult>,
    /// Whether the kitchen ticket printed.
    pub kitchen_printed: bool,
}

/// `pos_engine::commands::webshop::HandoverRequest`: collect an unpaid online order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct HandoverRequest {
    pub order_id: i64,
    /// What the customer paid, as at the till.
    pub payments: Vec<PaymentPart>,
    #[serde(default)]
    pub delivery: Option<TicketDeliveryRequest>,
}

/// `pos_engine::commands::webshop::RefundRequest`: refund a cancelled, signed online order.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct RefundRequest {
    pub order_id: i64,
    /// The manager confirming the refund.
    pub employee_insz: String,
    #[serde(default)]
    pub delivery: Option<TicketDeliveryRequest>,
}
