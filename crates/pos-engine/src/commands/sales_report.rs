//! Non-fiscal sales reports (V11-D20): by product, by hour, per booking date, for a range of
//! booking dates, on screen and on paper.
//!
//! Read-only and never signed: no event, no counter, nothing sent to the FDM. The figures come
//! from the archived N events (`pos_fiscal::archive`, the signed request of each sale), the
//! booking periods and the stored Z reports, never from the mutable order tables. Training
//! events (label T) are not N events and are left out; a refund is an N event (citing the refunded
//! ticket in `fdmRef`) with negative
//! quantities and shows as negative amounts.
//!
//! Reconciliation: the grand total is the sum of the `transactionTotal` of the sales, that is
//! the turnover of label N of the Z report (`report_totals::turnover`). Product rows carry the
//! price of their lines and the line price changes; the price changes of scope EVENT are their
//! own row. The cash rounding (ROUNDING payment lines) is not part of `transactionTotal`: it is
//! shown apart and is NOT added to the total, so that the total equals the Z.

use std::collections::BTreeMap;

use chrono::{NaiveDate, Timelike};
use chrono_tz::Europe::Brussels;
use pos_db::queries::reports::{self as report_queries, PeriodZ};
use pos_fiscal::archive;
use pos_fiscal::report_totals::{self, ReportEvent};
use pos_fiscal::ticket::ReceiptLanguage;
use pos_fiscal::types::EventLabel;
use pos_print::sales_report::{
    SalesDayPrint, SalesHourPrint, SalesProductPrint, SalesReportPrint, format_sales_report,
};
use serde::Serialize;
use serde_json::Value;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// `amount` against `biggest` as a whole percent; a non-zero amount always shows at least 1.
fn bar_percent(amount: i64, biggest: i64) -> u32 {
    if biggest == 0 || amount == 0 {
        return 0;
    }
    // Rounded half up in integers: exact where a float quotient could land
    // a hair below a tie.
    let biggest = i128::from(biggest).abs();
    let percent = (i128::from(amount).abs() * 200 + biggest) / (2 * biggest);
    let percent = u32::try_from(percent).unwrap_or(u32::MAX);
    percent.clamp(1, 100)
}

/// The longest range, in days (both ends included).
pub const MAX_RANGE_DAYS: i64 = 92;

/// One booking period of the range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesDay {
    /// `YYYY-MM-DD`. A date with two Z reports shows two rows.
    pub booking_date: String,
    /// The Z that closed the period; none while it is open.
    pub z_number: Option<i64>,
    /// The sales of the period, from the archived N events.
    pub total_cents: i64,
    /// The turnover of label N the Z printed.
    pub z_total_cents: Option<i64>,
    /// The period is still open ("période en cours").
    pub open: bool,
    /// Closed, with a Z, and the two totals are equal.
    pub matches_z: bool,
    /// `total_cents - z_total_cents` of a closed period with a Z: what the screens print as the
    /// gap, so that they compute no money themselves.
    pub gap_cents: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesProduct {
    pub product_id: String,
    pub name: String,
    pub department: String,
    /// Thousandths of a unit (2 pieces = 2000), negative for refunds.
    pub quantity_milli: i64,
    pub amount_cents: i64,
    /// Sold by weight: `quantity_milli` is then in grams.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub weighed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesHour {
    /// Local hour in Belgium, 0 to 23.
    pub hour: u32,
    pub tickets: i64,
    pub amount_cents: i64,
    /// Length of the hour's bar, 0 to 100: its amount against the biggest hour.
    pub bar_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesReport {
    pub from: String,
    pub to: String,
    /// RFC 3339, Belgian time.
    pub generated_at: String,
    /// The business date now (`YYYY-MM-DD`, from `day_end_hour`): what "today" means to the
    /// screens, so that a service past midnight still shows under its own day.
    pub business_date: String,
    pub days: Vec<SalesDay>,
    pub open_period: bool,
    pub by_product: Vec<SalesProduct>,
    /// Only the hours with sales, from `day_end_hour` round the clock.
    pub by_hour: Vec<SalesHour>,
    /// Price changes of scope EVENT (a row of the total, apart from the products).
    pub price_changes_cents: i64,
    /// Cash rounding, informational: NOT part of the total.
    pub rounding_cents: i64,
    /// Σ of the refund tickets (N events citing the refunded ticket in `fdmRef`), already inside the
    /// products and the total. Other negative lines (a discount reversal or a correction inside a
    /// sale) are not refunds and stay in the products only.
    pub refunds_cents: i64,
    /// N events of the range, refunds included.
    pub ticket_count: i64,
    /// Sum of the `transactionTotal` of the sales = turnover N of the Z reports.
    pub total_cents: i64,
}

fn range_invalid() -> String {
    errors::SALES_REPORT_RANGE_INVALID.to_string()
}

/// The two ends of the range, checked.
fn parse_range(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let parse = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| range_invalid());
    let (from, to) = (parse(from)?, parse(to)?);
    if to < from || (to - from).num_days() >= MAX_RANGE_DAYS {
        return Err(range_invalid());
    }
    Ok((from, to))
}

/// One sale as the report reads it.
struct Sale {
    amount: i64,
    event_changes: i64,
    hour: Option<u32>,
    rounding: i64,
}

/// Amount of a line after its LINE-scope price changes, and the EVENT-scope changes apart.
fn split_line(line: &Value) -> Result<(i64, i64), String> {
    let mut own = 0;
    let mut event = 0;
    for vat in line["mainProduct"]["vats"].as_array().into_iter().flatten() {
        own += errors::archived_cents(&vat["price"], "price")?;
        for change in vat["priceChanges"].as_array().into_iter().flatten() {
            let amount = errors::archived_cents(&change["amount"], "priceChanges")?;
            if change["scope"].as_str() == Some("EVENT") {
                event += amount;
            } else {
                own += amount;
            }
        }
    }
    Ok((own, event))
}

/// Σ of the ROUNDING payment lines of a sale.
fn rounding_of(event: &ReportEvent) -> Result<i64, String> {
    event.request["financials"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["amountType"].as_str() == Some("ROUNDING"))
        .map(|f| errors::archived_cents(&f["amount"], "amount"))
        .sum()
}

/// The turnover of label N in a stored turnover Z request.
fn z_turnover_n(z_json: &str) -> Option<i64> {
    let z: Value = serde_json::from_str(z_json).ok()?;
    z["turnover"]["transactions"]
        .as_array()?
        .iter()
        .find(|t| t["eventLabel"].as_str() == Some("N"))
        .map_or(Some(0), |t| pos_core::money::cents_of(&t["amount"]))
}

/// Compute the report from the database.
#[expect(
    clippy::too_many_lines,
    reason = "one linear aggregation of the sales report"
)]
fn compute(state: &AppState, from: &str, to: &str) -> Result<SalesReport, String> {
    let (from_date, to_date) = parse_range(from, to)?;
    let day_end_hour = state.config.lock().map_err(errors::internal)?.day_end_hour;
    let db = state.db.lock().map_err(errors::internal)?;
    let periods: Vec<PeriodZ> =
        report_queries::periods_with_z(&db, &from_date.to_string(), &to_date.to_string())
            .map_err(errors::internal)?;

    let mut days = Vec::new();
    let mut products: BTreeMap<(String, String, String, bool), (i64, i64)> = BTreeMap::new();
    let mut hours: BTreeMap<u32, (i64, i64)> = BTreeMap::new();
    let (mut price_changes, mut rounding, mut refunds, mut tickets, mut total) = (0, 0, 0, 0, 0);

    for period in &periods {
        let mut period_total = 0;
        for archived in archive::events_in_booking_period(&db, &period.period_id)
            .map_err(errors::internal)?
            .iter()
            .filter(|e| e.event_type == EventLabel::N)
        {
            let event = ReportEvent::from_archive(archived).ok_or_else(|| {
                errors::internal(format!(
                    "archived fiscal event {} is unreadable",
                    archived.id
                ))
            })?;
            if !event.has_transaction() {
                continue;
            }
            let mut sale = Sale {
                amount: 0,
                event_changes: 0,
                hour: event
                    .pos_date_time()
                    .map(|t| t.with_timezone(&Brussels).hour()),
                rounding: rounding_of(&event)?,
            };
            for line in event.lines() {
                let (own, event_level) = split_line(line)?;
                let product = &line["mainProduct"];
                let key = (
                    product["productId"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    product["productName"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    product["departmentName"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    product["quantityType"] == "KILOGRAM",
                );
                let entry = products.entry(key).or_default();
                entry.0 += errors::archived_milli(&product["quantity"], "quantity")?;
                entry.1 += own;
                sale.amount += own;
                sale.event_changes += event_level;
            }
            // The sale's transactionTotal as the Z counts it.
            debug_assert_eq!(
                sale.amount + sale.event_changes,
                report_totals::transaction_amount(&event)
            );
            let sale_total = sale.amount + sale.event_changes;
            // A refund ticket is the N event that cites the refunded ticket (`fdmRef`, the same
            // mark `payment_correction` uses); a negative line inside another sale is not one.
            if event.request.get("fdmRef").is_some_and(Value::is_object) {
                refunds += sale_total;
            }
            price_changes += sale.event_changes;
            rounding += sale.rounding;
            tickets += 1;
            period_total += sale_total;
            if let Some(hour) = sale.hour {
                let slot = hours.entry(hour).or_default();
                slot.0 += 1;
                slot.1 += sale_total;
            }
        }
        total += period_total;
        let z_total = period.z.as_ref().and_then(|(_, json)| z_turnover_n(json));
        days.push(SalesDay {
            booking_date: period.booking_date.clone(),
            z_number: period.z.as_ref().map(|(n, _)| *n),
            total_cents: period_total,
            z_total_cents: z_total,
            open: period.open,
            matches_z: !period.open && z_total == Some(period_total),
            gap_cents: z_total.filter(|_| !period.open).map(|z| period_total - z),
        });
    }

    let mut by_product: Vec<SalesProduct> = products
        .into_iter()
        .map(
            |((product_id, name, department, weighed), (quantity_milli, amount_cents))| {
                SalesProduct {
                    product_id,
                    name,
                    department,
                    quantity_milli,
                    amount_cents,
                    weighed,
                }
            },
        )
        .collect();
    by_product.sort_by(|a, b| {
        b.amount_cents
            .cmp(&a.amount_cents)
            .then_with(|| a.name.cmp(&b.name))
    });

    // From day_end_hour round the clock: the business day starts there.
    let start = day_end_hour % 24;
    let mut by_hour: Vec<SalesHour> = hours
        .into_iter()
        .map(|(hour, (tickets, amount_cents))| SalesHour {
            hour,
            tickets,
            amount_cents,
            bar_percent: 0,
        })
        .collect();
    let biggest = by_hour
        .iter()
        .map(|h| h.amount_cents.abs())
        .max()
        .unwrap_or(0);
    for hour in &mut by_hour {
        hour.bar_percent = bar_percent(hour.amount_cents, biggest);
    }
    by_hour.sort_by_key(|h| (h.hour + 24 - start) % 24);

    Ok(SalesReport {
        from: from_date.to_string(),
        to: to_date.to_string(),
        generated_at: pos_fiscal::event::now_in_belgium().to_rfc3339(),
        business_date: super::business_date(pos_fiscal::event::now_in_belgium(), day_end_hour),
        open_period: days.iter().any(|d| d.open),
        days,
        by_product,
        by_hour,
        price_changes_cents: price_changes,
        rounding_cents: rounding,
        refunds_cents: refunds,
        ticket_count: tickets,
        total_cents: total,
    })
}

/// The sales report of the booking dates `from..=to` (`YYYY-MM-DD`, at most 92 days). Manager.
pub fn do_sales_report(state: &AppState, from: String, to: String) -> Result<SalesReport, String> {
    access::operator(state, Access::Admin)?;
    compute(state, &from, &to)
}

fn print_data(state: &AppState, report: &SalesReport) -> Result<SalesReportPrint, String> {
    let config = state.config.lock().map_err(errors::internal)?;
    let generated = chrono::DateTime::parse_from_rfc3339(&report.generated_at)
        .map(|t| t.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_default();
    Ok(SalesReportPrint {
        lang: ReceiptLanguage::from_code(&config.language),
        restaurant_name: config.restaurant_name.clone(),
        restaurant_address: config.restaurant_address.clone(),
        restaurant_vat: config.restaurant_vat.clone(),
        from: report.from.clone(),
        to: report.to.clone(),
        generated_at: generated,
        days: report
            .days
            .iter()
            .map(|d| SalesDayPrint {
                booking_date: d.booking_date.clone(),
                z_number: d.z_number,
                total_cents: d.total_cents,
                z_total_cents: d.z_total_cents,
                open: d.open,
                matches_z: d.matches_z,
            })
            .collect(),
        products: report
            .by_product
            .iter()
            .map(|p| SalesProductPrint {
                name: p.name.clone(),
                quantity_milli: p.quantity_milli,
                amount_cents: p.amount_cents,
                weighed: p.weighed,
            })
            .collect(),
        hours: report
            .by_hour
            .iter()
            .map(|h| SalesHourPrint {
                hour: h.hour,
                tickets: h.tickets,
                amount_cents: h.amount_cents,
            })
            .collect(),
        price_changes_cents: report.price_changes_cents,
        rounding_cents: report.rounding_cents,
        refunds_cents: report.refunds_cents,
        ticket_count: report.ticket_count,
        total_cents: report.total_cents,
    })
}

/// Print the sales report of the range on the ticket printer. Manager. Fails with
/// `printer_not_configured` or `print_failed` rather than pretending to have printed.
pub async fn do_print_sales_report(
    state: &AppState,
    from: String,
    to: String,
) -> Result<(), String> {
    access::operator(state, Access::Admin)?;
    let report = compute(state, &from, &to)?;
    let data = print_data(state, &report)?;
    let (addr, paper) = {
        let config = state.config.lock().map_err(errors::internal)?;
        let addr = config
            .printer_address
            .clone()
            .filter(|a| !a.is_empty())
            .ok_or_else(|| errors::PRINTER_NOT_CONFIGURED.to_string())?;
        let paper = config.paper_for(&addr);
        (addr, paper)
    };
    let bytes = format_sales_report(&data, paper);
    match pos_print::transport::send_to_printer_with(
        &addr,
        &bytes,
        state.timers.printer_status_read,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::warn!(%e, "sales report not printed");
            Err(errors::coded(errors::PRINT_FAILED, &[("detail", &e)]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_is_checked() {
        assert!(parse_range("2026-10-01", "2026-10-01").is_ok());
        assert!(parse_range("2026-10-02", "2026-10-01").is_err());
        assert!(parse_range("2026-10-01", "nope").is_err());
        // 92 days are fine, 93 are not.
        assert!(parse_range("2026-07-01", "2026-09-30").is_ok());
        assert!(parse_range("2026-07-01", "2026-10-01").is_err());
    }

    #[test]
    fn a_bar_is_a_percent_of_the_biggest_hour() {
        assert_eq!(bar_percent(9000, 9000), 100);
        assert_eq!(bar_percent(4500, 9000), 50);
        assert_eq!(bar_percent(-4500, 9000), 50);
        assert_eq!(bar_percent(1, 9000), 1);
        assert_eq!(bar_percent(0, 9000), 0);
        assert_eq!(bar_percent(5, 0), 0);
    }

    #[test]
    fn the_z_turnover_is_the_amount_of_label_n() {
        let z = r#"{"turnover":{"transactions":[{"eventLabel":"N","ticketCount":2,"amount":41.5},{"eventLabel":"P","ticketCount":1,"amount":9.0}]}}"#;
        assert_eq!(z_turnover_n(z), Some(4150));
        assert_eq!(z_turnover_n("{}"), None);
        let no_amount = r#"{"turnover":{"transactions":[{"eventLabel":"N"}]}}"#;
        assert_eq!(z_turnover_n(no_amount), None, "a missing amount is not 0");
    }

    #[test]
    fn a_line_without_a_price_is_an_error() {
        let line = serde_json::json!({"mainProduct": {"vats": [{"priceChanges": []}]}});
        assert!(split_line(&line).is_err());
    }

    #[test]
    fn event_price_changes_are_split_from_the_line() {
        let line = serde_json::json!({"mainProduct": {"vats": [
            {"price": 10.0, "priceChanges": [
                {"scope": "LINE", "amount": -1.0}, {"scope": "EVENT", "amount": -0.5}]}
        ]}});
        assert_eq!(split_line(&line).unwrap(), (900, -50));
    }
}
