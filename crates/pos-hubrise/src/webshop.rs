//! Online orders on their way to the FDM: stored, resolved against the
//! POS menu, classified by payment. Nothing here signs; acceptance
//! does, and only an order that resolved cleanly can be accepted.
//!
//! Resolution:
//! - every item, option and charge `ref` must match an active element
//!   of the POS menu (which mirrors the catalog);
//! - the VAT label comes from the menu (eat-in, or takeaway for
//!   delivery and collection), and must agree with the order's
//!   `tax_rate` when the order gives one;
//! - a charge the restaurant keeps (delivery, packaging) follows the
//!   food it comes with: it is split over the products' VAT labels in
//!   proportion to their totals (an accessory supply, Circular
//!   2017/C/70), whatever rate the order gives it;
//! - a platform's charges are its own turnover: excluded by default;
//! - the unit price is what the customer was charged; a price other
//!   than the catalog's is only a warning, since platforms mark up;
//! - the lines must add up to the order total.
//!
//! Payment: prepaid when non-cash payments cover the total, unpaid
//! with no payment or cash only (collected at handover), anything else
//! needs review.

use pos_core::vat::VatCode;
use pos_hubrise_types::money::{MAX_AMOUNT_CENTS, parse_cents, parse_quantity};
use pos_hubrise_types::{Order, OrderStatus, ServiceType};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::catalog_map::code_of_rate;
use crate::errors::Result;

/// The fiscal language of product names on the lines.
const FISCAL_LANG: &str = "fr";

/// Why an order cannot be accepted as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Attention {
    /// No `hubrise_channels` row for the order's channel.
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
    /// HubRise changed what was sold after the POS signed it: the
    /// signed sale stands, staff settle the difference.
    ChangedAfterSigning,
}

impl Attention {
    /// Whether a newer catalog could fix it.
    pub fn is_catalog_miss(&self) -> bool {
        matches!(
            self,
            Attention::UnknownProduct { .. }
                | Attention::UnknownOption { .. }
                | Attention::UnknownCharge { .. }
        )
    }
}

/// Worth showing, not blocking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Warning {
    /// Charged another price than the catalog's (platform markup).
    PriceDiffers {
        at: String,
        catalog_cents: i64,
        charged_cents: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    Product,
    Charge,
}

/// A line to sign: the menu product, the name printed and sent to the
/// FDM (options in it), the charged price and the menu's VAT label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLine {
    pub kind: LineKind,
    pub menu_item_id: i64,
    pub name: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub vat_code: VatCode,
    pub department_id: String,
    pub department_name: String,
    /// The order's discounts, each spread over the product lines in
    /// proportion to their totals (a priceChange of scope EVENT).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discounts: Vec<LineDiscount>,
}

/// A line's share of an order discount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineDiscount {
    /// The discount's rank in the order, from 1 (FAQ Q304: successive
    /// changes increment the grouping).
    pub grouping_id: i32,
    /// The discount's `ref`, or `discount:{rank}` without one.
    pub id: String,
    pub name: String,
    /// Negative.
    pub amount_cents: i64,
}

impl ResolvedLine {
    /// Before discounts. Saturates instead of wrapping: `resolve` bounds
    /// every line it builds, so a saturated value never equals an order
    /// total.
    pub fn total_cents(&self) -> i64 {
        self.unit_price_cents.saturating_mul(self.quantity)
    }

    /// After discounts: what the customer pays for the line.
    pub fn net_cents(&self) -> i64 {
        self.discounts.iter().fold(self.total_cents(), |sum, d| {
            sum.saturating_add(d.amount_cents)
        })
    }
}

/// What the customer pays for `lines`.
pub fn net_total_cents(lines: &[ResolvedLine]) -> i64 {
    lines
        .iter()
        .fold(0, |sum, l| sum.saturating_add(l.net_cents()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMode {
    Prepaid,
    Unpaid,
}

impl PaymentMode {
    fn as_str(self) -> &'static str {
        match self {
            PaymentMode::Prepaid => "prepaid",
            PaymentMode::Unpaid => "unpaid",
        }
    }
}

/// A `hubrise_channels` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    pub channel: String,
    /// `platform` or `webshop`.
    pub kind: String,
    /// `APP` or `ONLINE`.
    pub payment_type: String,
    /// `include` or `exclude`.
    pub charge_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Resolution {
    pub channel: Option<Channel>,
    pub lines: Vec<ResolvedLine>,
    pub payment_mode: Option<PaymentMode>,
    pub attention: Vec<Attention>,
    pub warnings: Vec<Warning>,
}

impl Resolution {
    pub fn is_ok(&self) -> bool {
        self.attention.is_empty()
    }
}

/// The channel key of an order: its lower-cased channel, `webshop`
/// without one.
pub fn channel_key(order: &Order) -> String {
    order
        .channel
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .unwrap_or("webshop")
        .to_lowercase()
}

pub fn channel(conn: &Connection, key: &str) -> Result<Option<Channel>> {
    Ok(conn
        .query_row(
            "SELECT channel, kind, payment_type, charge_policy
             FROM hubrise_channels WHERE channel = ?1",
            params![key],
            |r| {
                Ok(Channel {
                    channel: r.get(0)?,
                    kind: r.get(1)?,
                    payment_type: r.get(2)?,
                    charge_policy: r.get(3)?,
                })
            },
        )
        .optional()?)
}

pub fn list_channels(conn: &Connection) -> Result<Vec<Channel>> {
    let mut stmt = conn.prepare(
        "SELECT channel, kind, payment_type, charge_policy FROM hubrise_channels ORDER BY channel",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Channel {
                channel: r.get(0)?,
                kind: r.get(1)?,
                payment_type: r.get(2)?,
                charge_policy: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Add or change a channel mapping (the key is lower-cased), then
/// resolve pending orders again. `Err(Db)` on a value the table's
/// checks refuse.
pub fn set_channel(conn: &Connection, c: &Channel) -> Result<usize> {
    conn.execute(
        "INSERT INTO hubrise_channels (channel, kind, payment_type, charge_policy)
         VALUES (lower(trim(?1)), ?2, ?3, ?4)
         ON CONFLICT(channel) DO UPDATE SET kind = excluded.kind,
            payment_type = excluded.payment_type, charge_policy = excluded.charge_policy",
        params![c.channel, c.kind, c.payment_type, c.charge_policy],
    )?;
    re_resolve_pending(conn)
}

fn is_cash_ref(conn: &Connection, payment_ref: Option<&str>) -> Result<bool> {
    let Some(r) = payment_ref else {
        return Ok(false);
    };
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM hubrise_cash_payment_refs WHERE upper(ref) = upper(?1))",
        params![r.trim()],
        |row| row.get(0),
    )?)
}

/// An active menu element by key: (id, catalog price).
fn menu_item(conn: &Connection, key: &str, kind: &str) -> Result<Option<(i64, i64)>> {
    Ok(conn
        .query_row(
            "SELECT id, price_cents FROM menu_items
             WHERE external_id = ?1 AND active = 1 AND kind = ?2",
            params![key, kind],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

/// An active option of `menu_item_id` whose key ends in `|{option_ref}`:
/// its catalog price.
fn option_price(conn: &Connection, menu_item_id: i64, option_ref: &str) -> Result<Option<i64>> {
    let suffix = format!("|{option_ref}");
    Ok(conn
        .query_row(
            "SELECT m.price_cents FROM modifiers m
             JOIN modifier_groups g ON g.id = m.group_id
             WHERE g.menu_item_id = ?1 AND g.active = 1 AND m.active = 1
               AND substr(m.external_id, -length(?2)) = ?2
             LIMIT 1",
            params![menu_item_id, suffix],
            |r| r.get(0),
        )
        .optional()?)
}

fn cents(money: &str, at: &str, attention: &mut Vec<Attention>) -> i64 {
    parse_cents(money).unwrap_or_else(|_| {
        attention.push(Attention::BadPrice { at: at.to_owned() });
        0
    })
}

/// Resolve `order` against the POS menu and classify its payment.
pub fn resolve(conn: &Connection, order: &Order) -> Result<Resolution> {
    let mut attention = Vec::new();
    let mut warnings = Vec::new();
    let mut lines = Vec::new();

    let key = channel_key(order);
    let channel = channel(conn, &key)?;
    if channel.is_none() {
        attention.push(Attention::UnknownChannel { channel: key });
    }
    let takeaway = match order.service_type {
        Some(ServiceType::Delivery | ServiceType::Collection) => true,
        Some(ServiceType::EatIn) => false,
        None => {
            attention.push(Attention::MissingServiceType);
            true
        }
    };

    let items: Vec<_> = order.items.iter().filter(|i| !i.deleted).collect();
    if items.is_empty() {
        attention.push(Attention::NoItems);
    }
    for item in items {
        let Some(sku_ref) = item.sku_ref.as_deref().filter(|r| !r.trim().is_empty()) else {
            attention.push(Attention::ItemWithoutRef {
                name: item.product_name.clone(),
            });
            continue;
        };
        let Some((id, catalog_price)) = menu_item(conn, sku_ref, "product")? else {
            attention.push(Attention::UnknownProduct {
                sku_ref: sku_ref.to_owned(),
            });
            continue;
        };
        let Some(quantity) = parse_quantity(&item.quantity) else {
            attention.push(Attention::FractionalQuantity {
                sku_ref: sku_ref.to_owned(),
            });
            continue;
        };

        let mut charged = cents(&item.price, sku_ref, &mut attention);
        let mut catalog = catalog_price;
        let mut option_names = Vec::new();
        // Remote numbers: any overflow holds the line for staff.
        let mut overflow = false;
        for opt in &item.options {
            let qty = i64::from(opt.quantity.max(1));
            let price = opt
                .price
                .as_deref()
                .map_or(0, |p| cents(p, sku_ref, &mut attention));
            match price.checked_mul(qty).and_then(|x| charged.checked_add(x)) {
                Some(v) => charged = v,
                None => overflow = true,
            }
            match opt.ref_.as_deref() {
                Some(option_ref) => match option_price(conn, id, option_ref)? {
                    Some(p) => match p.checked_mul(qty).and_then(|x| catalog.checked_add(x)) {
                        Some(v) => catalog = v,
                        None => overflow = true,
                    },
                    None => attention.push(Attention::UnknownOption {
                        sku_ref: sku_ref.to_owned(),
                        option_ref: option_ref.to_owned(),
                    }),
                },
                None => attention.push(Attention::UnknownOption {
                    sku_ref: sku_ref.to_owned(),
                    option_ref: opt.name.clone(),
                }),
            }
            let name = if opt.removed {
                format!("sans {}", opt.name)
            } else if qty > 1 {
                format!("{qty}x {}", opt.name)
            } else {
                opt.name.clone()
            };
            option_names.push(name);
        }
        if overflow
            || charged
                .checked_mul(quantity)
                .is_none_or(|t| t.abs() > MAX_AMOUNT_CENTS)
        {
            attention.push(Attention::BadPrice {
                at: sku_ref.to_owned(),
            });
            continue;
        }
        if charged != catalog {
            warnings.push(Warning::PriceDiffers {
                at: sku_ref.to_owned(),
                catalog_cents: catalog,
                charged_cents: charged,
            });
        }

        let menu = pos_db::queries::menu::resolve_catalogue_line(conn, id, takeaway, FISCAL_LANG)?;
        check_vat(
            item.tax_rate.as_deref(),
            menu.vat_code,
            sku_ref,
            &mut attention,
        );
        let name = if option_names.is_empty() {
            menu.name
        } else {
            format!("{} ({})", menu.name, option_names.join(", "))
        };
        lines.push(ResolvedLine {
            kind: LineKind::Product,
            menu_item_id: id,
            name,
            quantity,
            unit_price_cents: charged,
            vat_code: menu.vat_code,
            department_id: menu.department_id,
            department_name: menu.department_name,
            discounts: Vec::new(),
        });
    }

    // Discounts first: only products get them, and the charges are
    // split by what the products come to after them.
    let mut discount_cents = 0_i64;
    for (rank, discount) in order.discounts.iter().filter(|d| !d.deleted).enumerate() {
        let off = cents(&discount.price_off, &discount.name, &mut attention);
        discount_cents = discount_cents.saturating_add(off);
        spread_discount(&mut lines, rank + 1, discount, off, &mut attention);
    }

    let include_charges = channel
        .as_ref()
        .is_none_or(|c| c.charge_policy == "include");
    let mut excluded_cents = 0_i64;
    for charge in order.charges.iter().filter(|c| !c.deleted) {
        let at = charge.ref_.clone().unwrap_or_else(|| charge.name.clone());
        let price = cents(&charge.price, &at, &mut attention);
        if !include_charges {
            excluded_cents = excluded_cents.saturating_add(price);
            continue;
        }
        let key = format!("charge:{}", charge.ref_.as_deref().unwrap_or_default());
        let Some((id, _)) = menu_item(conn, &key, "charge")? else {
            attention.push(Attention::UnknownCharge { charge_ref: at });
            continue;
        };
        let menu = pos_db::queries::menu::resolve_catalogue_line(conn, id, takeaway, FISCAL_LANG)?;
        for (vat_code, share) in charge_shares(&lines, price, menu.vat_code) {
            lines.push(ResolvedLine {
                kind: LineKind::Charge,
                menu_item_id: id,
                name: menu.name.clone(),
                quantity: 1,
                unit_price_cents: share,
                vat_code,
                department_id: menu.department_id.clone(),
                department_name: menu.department_name.clone(),
                discounts: Vec::new(),
            });
        }
    }

    let total_cents = order
        .total
        .as_deref()
        .map_or(0, |t| cents(t, "total", &mut attention));
    let lines_cents = lines
        .iter()
        .try_fold(0_i64, |sum, l| sum.checked_add(l.total_cents()));
    let blocked_lines = attention.iter().any(|a| {
        a.is_catalog_miss()
            || matches!(
                a,
                Attention::ItemWithoutRef { .. } | Attention::FractionalQuantity { .. }
            )
    });
    // Missing lines already explain a gap; only a full set is checked.
    let expected = lines_cents
        .and_then(|l| l.checked_sub(discount_cents))
        .and_then(|l| l.checked_add(excluded_cents));
    if !blocked_lines && expected != Some(total_cents) {
        attention.push(Attention::TotalMismatch {
            total_cents,
            lines_cents: lines_cents.unwrap_or(i64::MAX),
        });
    }

    let payment_mode = classify_payment(conn, order, total_cents, &mut attention)?;

    Ok(Resolution {
        channel,
        lines,
        payment_mode,
        attention,
        warnings,
    })
}

fn check_vat(rate: Option<&str>, menu: VatCode, at: &str, attention: &mut Vec<Attention>) {
    let Some(rate) = rate else { return };
    if code_of_rate(rate) != Some(menu) {
        attention.push(Attention::VatMismatch {
            at: at.to_owned(),
            order: rate.to_owned(),
            menu,
        });
    }
}

/// A charge of `price` cents split over the products' VAT labels, in
/// proportion to what each label's products come to (after their
/// discounts), to the cent. With no product to follow, the menu's own
/// label. A free charge is one line at 0.
fn charge_shares(lines: &[ResolvedLine], price: i64, fallback: VatCode) -> Vec<(VatCode, i64)> {
    let mut by_vat: Vec<(VatCode, i64)> = Vec::new();
    for line in lines.iter().filter(|l| l.kind == LineKind::Product) {
        match by_vat.iter_mut().find(|(code, _)| *code == line.vat_code) {
            Some((_, total)) => *total += line.net_cents(),
            None => by_vat.push((line.vat_code, line.net_cents())),
        }
    }
    by_vat.retain(|(_, total)| *total > 0);
    if by_vat.is_empty() || price <= 0 {
        return vec![(fallback, price)];
    }
    // The label with the most first, so its line comes first.
    by_vat.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.as_str().cmp(b.0.as_str())));
    let weights: Vec<i64> = by_vat.iter().map(|(_, t)| *t).collect();
    let total: i64 = weights.iter().sum();
    let shares = charge_round_shares(&weights, total, price);
    by_vat
        .into_iter()
        .zip(shares)
        .filter(|(_, share)| *share != 0)
        .map(|((code, _), share)| (code, share))
        .collect()
}

/// `price` cents over `weights` (summing to `total`, all positive): a
/// charge larger than the products goes round whole times (each label
/// takes its weight per round), then the rest is distributed. Computed
/// in one step, so a huge remote price costs nothing.
fn charge_round_shares(weights: &[i64], total: i64, price: i64) -> Vec<i64> {
    // `distribute` never gives a label more than its weight.
    let rounds = price / total;
    let rest = price % total;
    weights
        .iter()
        .zip(pos_core::money::distribute(weights, rest))
        // `w * rounds <= total * rounds <= price`: no overflow.
        .map(|(w, d)| w * rounds + d)
        .collect()
}

/// Spread a discount of `off` cents over the product lines, by their
/// totals after the discounts already spread, to the cent. A discount
/// larger than those lines is held for staff.
fn spread_discount(
    lines: &mut [ResolvedLine],
    rank: usize,
    discount: &pos_hubrise_types::order::OrderDiscount,
    off: i64,
    attention: &mut Vec<Attention>,
) {
    if off <= 0 {
        return;
    }
    let products: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].kind == LineKind::Product)
        .collect();
    let weights: Vec<i64> = products.iter().map(|&i| lines[i].net_cents()).collect();
    if off > weights.iter().sum::<i64>() {
        attention.push(Attention::Discount {
            name: discount.name.clone(),
        });
        return;
    }
    let grouping_id = i32::try_from(rank).unwrap_or(i32::MAX);
    let id = discount
        .ref_
        .clone()
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| format!("discount:{rank}"));
    for (&i, share) in products
        .iter()
        .zip(pos_core::money::distribute(&weights, -off))
    {
        if share != 0 {
            lines[i].discounts.push(LineDiscount {
                grouping_id,
                id: id.clone(),
                name: discount.name.clone(),
                amount_cents: share,
            });
        }
    }
}

fn classify_payment(
    conn: &Connection,
    order: &Order,
    total_cents: i64,
    attention: &mut Vec<Attention>,
) -> Result<Option<PaymentMode>> {
    let mut paid = 0_i64;
    let mut cash = 0;
    let mut other = 0;
    for p in order.payments.iter().filter(|p| !p.deleted) {
        let amount = cents(&p.amount, "payment", attention);
        paid = paid.saturating_add(amount);
        if is_cash_ref(conn, p.ref_.as_deref())? {
            cash += 1;
        } else {
            other += 1;
        }
    }
    Ok(match (cash, other) {
        (0, 0) => Some(PaymentMode::Unpaid),
        (_, 0) => Some(PaymentMode::Unpaid),
        (0, _) if paid == total_cents => Some(PaymentMode::Prepaid),
        _ => {
            attention.push(Attention::PaymentUnclear {
                paid_cents: paid,
                total_cents,
            });
            None
        }
    })
}

/// What ingesting an order found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingested {
    pub order_id: i64,
    /// Still waiting for staff: acknowledge it as `received`.
    pub pending: bool,
    /// A newer catalog could resolve it.
    pub catalog_miss: bool,
}

/// Store or refresh an online order: the `orders` row the board lists,
/// and its `webshop_orders` row. The resolution is redone only while
/// the order is pending; once signed, changes are Phase 6's
/// (cancellations, refunds). A platform cancelling before acceptance
/// voids the order: nothing was signed.
pub fn ingest(conn: &Connection, order: &Order) -> Result<Ingested> {
    let order_type = match order.service_type {
        Some(ServiceType::EatIn) => "dine_in",
        _ => "takeaway",
    };
    let service_type = order.service_type.map(|s| match s {
        ServiceType::Delivery => "delivery",
        ServiceType::Collection => "collection",
        ServiceType::EatIn => "eat_in",
    });
    let total_cents = order
        .total
        .as_deref()
        .and_then(|t| parse_cents(t).ok())
        .unwrap_or(0);
    let source = order
        .channel
        .clone()
        .unwrap_or_else(|| "webshop".to_string());
    let raw_json = serde_json::to_string(order)?;
    let previous: Option<String> = conn
        .query_row(
            "SELECT raw_hubrise_json FROM orders WHERE hubrise_order_id = ?1",
            params![order.id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();

    let order_id: i64 = conn.query_row(
        "INSERT INTO orders
            (order_type, status, user_insz, subtotal_cents, total_cents,
             hubrise_order_id, is_fiscal, source, hubrise_status, service_type,
             raw_hubrise_json)
         VALUES (?1, 'open', ?2, ?3, ?3, ?4, 0, ?5, ?6, ?7, ?8)
         ON CONFLICT(hubrise_order_id) WHERE hubrise_order_id IS NOT NULL DO UPDATE SET
            hubrise_status = excluded.hubrise_status,
            raw_hubrise_json = excluded.raw_hubrise_json
         RETURNING id",
        params![
            order_type,
            pos_core::user::SYSTEM_INSZ,
            total_cents,
            order.id,
            source,
            order.status.as_wire(),
            service_type,
            raw_json,
        ],
        |r| r.get(0),
    )?;

    let state: Option<String> = conn
        .query_row(
            "SELECT fiscal_state FROM webshop_orders WHERE order_id = ?1",
            params![order_id],
            |r| r.get(0),
        )
        .optional()?;
    if state.as_deref().is_some_and(|s| s != "pending") {
        if previous
            .as_deref()
            .is_some_and(|prev| content_changed(prev, &raw_json))
        {
            flag_changed_after_signing(conn, order_id)?;
        }
        follow_cancellation(conn, order_id)?;
        return Ok(Ingested {
            order_id,
            pending: false,
            catalog_miss: false,
        });
    }

    if matches!(
        order.status,
        OrderStatus::Cancelled | OrderStatus::Rejected | OrderStatus::DeliveryFailed
    ) {
        void_pending(conn, order_id, order)?;
        return Ok(Ingested {
            order_id,
            pending: false,
            catalog_miss: false,
        });
    }

    let resolution = resolve(conn, order)?;
    store_resolution(conn, order_id, order, &resolution)?;
    // A total the POS does not know yet shows as the order's own.
    if total_cents == 0 {
        let lines = net_total_cents(&resolution.lines);
        conn.execute(
            "UPDATE orders SET subtotal_cents = ?1, total_cents = ?1 WHERE id = ?2",
            params![lines, order_id],
        )?;
    }
    Ok(Ingested {
        order_id,
        pending: true,
        catalog_miss: resolution.attention.iter().any(Attention::is_catalog_miss),
    })
}

/// What HubRise last said of an order ends it without the food being
/// handed over.
const CANCELLED_UPSTREAM: &str = "json_extract(o.raw_hubrise_json, '$.status')
    IN ('cancelled', 'rejected', 'delivery_failed')";

/// A signed order HubRise cancelled waits for staff: a prepaid sale
/// (N) for its refund, an unpaid order (P) for its closing. Nothing is
/// signed here. Returns the new state, if it changed.
pub fn follow_cancellation(conn: &Connection, order_id: i64) -> Result<Option<String>> {
    let state = conn
        .query_row(
            &format!(
                "UPDATE webshop_orders SET fiscal_state = CASE fiscal_state
                    WHEN 'signed_n' THEN 'refund_pending' ELSE 'cancel_pending' END
                 WHERE order_id = ?1 AND fiscal_state IN ('signed_n', 'signed_p')
                   AND EXISTS (SELECT 1 FROM orders o WHERE o.id = ?1 AND {CANCELLED_UPSTREAM})
                 RETURNING fiscal_state"
            ),
            params![order_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(state)
}

/// Whether HubRise cancelled the order.
pub fn cancelled_upstream(conn: &Connection, order_id: i64) -> Result<bool> {
    Ok(conn.query_row(
        &format!("SELECT EXISTS (SELECT 1 FROM orders o WHERE o.id = ?1 AND {CANCELLED_UPSTREAM})"),
        params![order_id],
        |r| r.get(0),
    )?)
}

/// What was sold: a change after signing is not signed again.
fn content_changed(previous: &str, current: &str) -> bool {
    let (Ok(a), Ok(b)) = (
        serde_json::from_str::<serde_json::Value>(previous),
        serde_json::from_str::<serde_json::Value>(current),
    ) else {
        return false;
    };
    ["items", "charges", "discounts", "total"]
        .iter()
        .any(|k| a.get(k) != b.get(k))
}

fn flag_changed_after_signing(conn: &Connection, order_id: i64) -> Result<()> {
    let raw: String = conn.query_row(
        "SELECT attention_json FROM webshop_orders WHERE order_id = ?1",
        params![order_id],
        |r| r.get(0),
    )?;
    let mut attention: Vec<Attention> = serde_json::from_str(&raw)?;
    if !attention.contains(&Attention::ChangedAfterSigning) {
        attention.push(Attention::ChangedAfterSigning);
        conn.execute(
            "UPDATE webshop_orders SET attention_json = ?2 WHERE order_id = ?1",
            params![order_id, serde_json::to_string(&attention)?],
        )?;
    }
    Ok(())
}

fn void_pending(conn: &Connection, order_id: i64, order: &Order) -> Result<()> {
    conn.execute(
        "UPDATE orders SET status = 'voided', closed_at = datetime('now')
         WHERE id = ?1 AND status = 'open'",
        params![order_id],
    )?;
    conn.execute(
        "INSERT INTO webshop_orders (order_id, hubrise_order_id, channel, fiscal_state)
         VALUES (?1, ?2, ?3, 'voided')
         ON CONFLICT(order_id) DO UPDATE SET fiscal_state = 'voided'
         WHERE fiscal_state = 'pending'",
        params![order_id, order.id, channel_key(order)],
    )?;
    Ok(())
}

fn store_resolution(conn: &Connection, order_id: i64, order: &Order, r: &Resolution) -> Result<()> {
    let service_type = order.service_type.map(|s| match s {
        ServiceType::Delivery => "delivery",
        ServiceType::Collection => "collection",
        ServiceType::EatIn => "eat_in",
    });
    let payment_type = match r.payment_mode {
        Some(PaymentMode::Prepaid) => r.channel.as_ref().map(|c| c.payment_type.clone()),
        _ => None,
    };
    conn.execute(
        "INSERT INTO webshop_orders
            (order_id, hubrise_order_id, channel, channel_kind, service_type, payment_mode,
             payment_type, fiscal_state, resolution, attention_json, warnings_json, lines_json,
             resolved_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9, ?10, ?11, datetime('now'))
         ON CONFLICT(order_id) DO UPDATE SET
            channel = excluded.channel, channel_kind = excluded.channel_kind,
            service_type = excluded.service_type, payment_mode = excluded.payment_mode,
            payment_type = excluded.payment_type, resolution = excluded.resolution,
            attention_json = excluded.attention_json, warnings_json = excluded.warnings_json,
            lines_json = excluded.lines_json, resolved_at = excluded.resolved_at
         WHERE webshop_orders.fiscal_state = 'pending'
           -- Lines written by an acceptance whose signature failed are
           -- what a retry signs: they are not resolved again.
           AND NOT EXISTS (SELECT 1 FROM order_lines l WHERE l.order_id = webshop_orders.order_id)",
        params![
            order_id,
            order.id,
            channel_key(order),
            r.channel.as_ref().map(|c| c.kind.clone()),
            service_type,
            r.payment_mode.map(PaymentMode::as_str),
            payment_type,
            if r.is_ok() { "ok" } else { "needs_review" },
            serde_json::to_string(&r.attention)?,
            serde_json::to_string(&r.warnings)?,
            serde_json::to_string(&r.lines)?,
        ],
    )?;
    Ok(())
}

/// Resolve again every pending order that needs review, from its
/// stored HubRise payload: after a catalog change or a channel
/// setting. Returns how many now resolve cleanly.
pub fn re_resolve_pending(conn: &Connection) -> Result<usize> {
    let pending: Vec<(i64, String)> = {
        let mut stmt = conn.prepare(
            "SELECT o.id, o.raw_hubrise_json FROM webshop_orders w
             JOIN orders o ON o.id = w.order_id
             WHERE w.fiscal_state = 'pending' AND w.resolution = 'needs_review'
               AND o.raw_hubrise_json IS NOT NULL",
        )?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?
    };
    let mut fixed = 0;
    for (order_id, raw) in pending {
        let order: Order = serde_json::from_str(&raw)?;
        let resolution = resolve(conn, &order)?;
        if resolution.is_ok() {
            fixed += 1;
        }
        store_resolution(conn, order_id, &order, &resolution)?;
    }
    Ok(fixed)
}

/// The stored state of an online order, as the board and acceptance
/// read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

pub fn load(conn: &Connection, order_id: i64) -> Result<Option<WebshopState>> {
    let row = conn
        .query_row(
            "SELECT order_id, hubrise_order_id, channel, channel_kind, service_type,
                    payment_mode, payment_type, fiscal_state, resolution,
                    attention_json, warnings_json, lines_json
             FROM webshop_orders WHERE order_id = ?1",
            params![order_id],
            |r| {
                Ok((
                    WebshopState {
                        order_id: r.get(0)?,
                        hubrise_order_id: r.get(1)?,
                        channel: r.get(2)?,
                        channel_kind: r.get(3)?,
                        service_type: r.get(4)?,
                        payment_mode: r.get(5)?,
                        payment_type: r.get(6)?,
                        fiscal_state: r.get(7)?,
                        resolution: r.get(8)?,
                        attention: Vec::new(),
                        warnings: Vec::new(),
                        lines: Vec::new(),
                    },
                    r.get::<_, String>(9)?,
                    r.get::<_, String>(10)?,
                    r.get::<_, String>(11)?,
                ))
            },
        )
        .optional()?;
    let Some((mut state, attention, warnings, lines)) = row else {
        return Ok(None);
    };
    state.attention = serde_json::from_str(&attention)?;
    state.warnings = serde_json::from_str(&warnings)?;
    state.lines = serde_json::from_str(&lines)?;
    Ok(Some(state))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The loop `charge_round_shares` replaced.
    fn looped(weights: &[i64], total: i64, price: i64) -> Vec<i64> {
        let mut shares = vec![0; weights.len()];
        let mut left = price;
        while left > 0 {
            let chunk = left.min(total);
            for (s, d) in shares
                .iter_mut()
                .zip(pos_core::money::distribute(weights, chunk))
            {
                *s += d;
            }
            left -= chunk;
        }
        shares
    }

    #[test]
    fn round_shares_match_the_old_loop() {
        for weights in [
            vec![1000],
            vec![700, 300],
            vec![333, 333, 334],
            vec![5, 1, 1],
        ] {
            let total: i64 = weights.iter().sum();
            for price in (1..=3 * total).chain([10 * total + 7]) {
                assert_eq!(
                    charge_round_shares(&weights, total, price),
                    looped(&weights, total, price),
                    "{weights:?} {price}"
                );
            }
        }
    }

    #[test]
    fn a_huge_charge_is_computed_at_once() {
        let weights = [600, 400];
        // The old loop ran ~1e6 times here; at the parse cap it ran 1e9.
        let shares = charge_round_shares(&weights, 1000, MAX_AMOUNT_CENTS);
        assert_eq!(shares.iter().sum::<i64>(), MAX_AMOUNT_CENTS);
        assert_eq!(shares, vec![600_000_000, 400_000_000]);
    }

    #[test]
    fn line_totals_saturate_instead_of_wrapping() {
        let line = ResolvedLine {
            kind: LineKind::Product,
            menu_item_id: 1,
            name: "x".into(),
            quantity: i64::MAX / 2,
            unit_price_cents: 10,
            vat_code: VatCode::B,
            department_id: String::new(),
            department_name: String::new(),
            discounts: Vec::new(),
        };
        assert_eq!(line.total_cents(), i64::MAX);
        assert_eq!(net_total_cents(&[line.clone(), line]), i64::MAX);
    }
}
