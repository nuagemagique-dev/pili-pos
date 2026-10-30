//! The checkout scale: a product sold by weight gets its line from a
//! reading of the scale, never from the screen.
//!
//! Touching the product starts a weighing: the unit price goes to the
//! scale, the engine waits for a stable load and writes the line from what
//! the scale returned, with its proof in `scale_weighings`, in one
//! transaction. The weighing is not a fiscal event: the line then follows
//! the usual path (P or N). Without a `scale` section in `pos.json` nothing
//! here runs and a product sold by weight stays off sale.
//!
//! The protocol is `pos-scale`; decisions in
//! `docs/internal/design/BALANCE_DIBAL_FR.md`.

use std::sync::Arc;
use std::time::Duration;

use pos_core::config::{PosConfig, ScaleCheckValue, ScaleConfig};
use pos_core::order::{OrderLine, OrderStatus, OrderType};
use pos_scale::frame::{CheckValue, PriceDigits, scale_text};
use pos_scale::session::Failure;
use pos_scale::{CheckEvent, DriverConfig, Progress, ScaleDriver, WeighRequest};
use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::events::EngineEvent;
use crate::state::AppState;

/// The driver of the configured scale, kept while the section is unchanged.
#[derive(Default)]
pub struct ScaleSlot(std::sync::Mutex<Option<(ScaleConfig, Arc<ScaleDriver>)>>);

#[derive(Debug, Clone, Deserialize)]
pub struct ScaleWeighInput {
    pub order_id: i64,
    pub menu_item_id: i64,
}

/// How the scale is doing, for the header light and the order screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScaleStatus {
    pub configured: bool,
    /// It answered just now (or is in the middle of a weighing).
    pub connected: bool,
    pub busy: bool,
    /// The last verdict of its software check: `ok`, `refused`, `no_values`,
    /// or `unknown` before any check.
    pub check: String,
}

/// Where a weighing stands: `waiting`, `stabilizing` or `checking`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScaleProgress {
    pub stage: String,
}

fn check_values(values: &[ScaleCheckValue]) -> Vec<CheckValue> {
    let hex = |v: &str| u16::from_str_radix(v, 16).ok();
    values
        .iter()
        .filter_map(|v| {
            Some(CheckValue {
                checksum: hex(&v.checksum)?,
                correction: hex(&v.correction)?,
            })
        })
        .collect()
}

fn driver_config(config: &ScaleConfig) -> DriverConfig {
    DriverConfig::new(
        if config.price_digits == 5 {
            PriceDigits::Five
        } else {
            PriceDigits::Six
        },
        check_values(&config.check_values),
        Duration::from_millis(config.weigh_timeout_ms),
    )
}

fn scale_section(config: &PosConfig) -> Result<ScaleConfig, String> {
    config
        .scale
        .clone()
        .ok_or_else(|| errors::SCALE_NOT_CONFIGURED.to_string())
}

fn driver(state: &AppState, config: &ScaleConfig) -> Result<Arc<ScaleDriver>, String> {
    let mut slot = state.scale.0.lock().map_err(errors::internal)?;
    if let Some((known, driver)) = slot.as_ref()
        && known == config
    {
        return Ok(driver.clone());
    }
    let driver = Arc::new(ScaleDriver::new(
        driver_config(config),
        config.tcp_address.clone(),
    ));
    *slot = Some((config.clone(), driver.clone()));
    Ok(driver)
}

fn failure_code(failure: Failure) -> &'static str {
    match failure {
        Failure::Unreachable => errors::SCALE_UNREACHABLE,
        Failure::BelowMinimum => errors::SCALE_BELOW_MIN,
        Failure::Underload => errors::SCALE_UNDERLOAD,
        Failure::Overload => errors::SCALE_OVERLOAD,
        Failure::NoMotion => errors::SCALE_NO_MOTION,
        Failure::PriceRefused | Failure::PriceTooLarge => errors::SCALE_PRICE_REFUSED,
        Failure::ScaleError | Failure::Damaged => errors::SCALE_ERROR,
        Failure::CheckFailed | Failure::CheckValuesMissing => errors::SCALE_CHECK_FAILED,
        Failure::PriceMismatch => errors::SCALE_PRICE_MISMATCH,
        Failure::UnitUnsupported => errors::SCALE_UNIT_UNSUPPORTED,
        Failure::Cancelled => errors::SCALE_CANCELLED,
        Failure::TimedOut => errors::SCALE_TIMED_OUT,
    }
}

fn log_checks(state: &AppState, checks: &[CheckEvent]) {
    if checks.is_empty() {
        return;
    }
    let Ok(db) = state.db.lock() else {
        return;
    };
    for check in checks {
        let (event, detail) = match check {
            CheckEvent::Requested { z } => ("requested", Some(z.clone())),
            CheckEvent::Answered { values } => ("answered", Some(values.to_string())),
            CheckEvent::Passed => ("passed", None),
            CheckEvent::Refused => ("refused", None),
            CheckEvent::NoValues => ("no_values", None),
        };
        if let Err(e) = pos_db::queries::scale::record_check(&db, event, detail.as_deref()) {
            tracing::warn!("scale check log: {e}");
        }
    }
}

/// The checks an order must pass before and after the scale is asked: it
/// may have changed during the weighing.
fn check_order(db: &rusqlite::Connection, order_id: i64) -> Result<OrderType, String> {
    let order = pos_db::queries::orders::get_order(db, order_id).map_err(errors::internal)?;
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }
    super::order::unchanged_while_pending(db, order_id)?;
    Ok(order.order_type)
}

/// Weigh a product and add its line to the order. Answers when the scale
/// gave a stable reading, or with why nothing was added. Any staff member;
/// [`do_scale_cancel_weighing`] ends it early.
pub async fn do_scale_start_weighing(
    state: &AppState,
    input: ScaleWeighInput,
) -> Result<OrderLine, String> {
    let operator = access::operator(state, Access::Staff)?;
    let (section, lang) = {
        let config = state.config.lock().map_err(errors::internal)?;
        let lang = pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language).code();
        (scale_section(&config)?, lang)
    };
    let driver = driver(state, &section)?;
    if driver.is_busy() {
        return Err(errors::SCALE_BUSY.into());
    }
    let product = {
        let db = state.db.lock().map_err(errors::internal)?;
        let order_type = check_order(&db, input.order_id)?;
        super::sold_out::refuse_if_sold_out(&db, input.menu_item_id)?;
        pos_db::queries::menu::resolve_catalogue_line(
            &db,
            input.menu_item_id,
            order_type == OrderType::Takeaway,
            lang,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => errors::coded(
                errors::PRODUCT_UNAVAILABLE,
                &[("product", &input.menu_item_id)],
            ),
            other => errors::internal(other),
        })?
    };
    if !product.sold_by_weight {
        return Err(errors::SCALE_NOT_SOLD_BY_WEIGHT.into());
    }
    let request = WeighRequest {
        unit_price_cents: product.price_cents,
        tare_grams: product.scale_tare_grams,
        text: scale_text(&product.name),
    };
    let events = state.events.clone();
    let outcome = driver
        .weigh(request, move |progress| {
            let stage = match progress {
                Progress::Waiting => "waiting",
                Progress::Stabilizing => "stabilizing",
                Progress::Checking => "checking",
            };
            events.emit(EngineEvent::ScaleProgress(ScaleProgress {
                stage: stage.to_string(),
            }));
        })
        .await;
    log_checks(state, &outcome.checks);
    let reading = outcome
        .result
        .map_err(|failure| failure_code(failure).to_string())?;

    let db = state.db.lock().map_err(errors::internal)?;
    check_order(&db, input.order_id)?;
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let line = pos_db::queries::orders::add_weighed_order_line(
        &tx,
        pos_db::queries::orders::NewOrderLine {
            order_id: input.order_id,
            menu_item_id: input.menu_item_id,
            quantity: 1,
            unit_price_cents: reading.unit_price_cents,
            vat_code: product.vat_code,
            name: &product.name,
            modifiers_json: None,
            department_id: &product.department_id,
            department_name: &product.department_name,
        },
        pos_db::queries::orders::Weighed {
            weight_grams: reading.weight_grams,
            amount_cents: reading.amount_cents,
        },
    )
    .map_err(errors::internal)?;
    pos_db::queries::scale::record_weighing(
        &tx,
        &pos_db::queries::scale::NewWeighing {
            line_id: line.id,
            order_id: input.order_id,
            weight_grams: reading.weight_grams,
            unit_price_cents: reading.unit_price_cents,
            amount_cents: reading.amount_cents,
            tare_grams: product.scale_tare_grams,
            operator_insz: &operator.insz,
        },
    )
    .map_err(errors::internal)?;
    super::courses::assign_new_line(&tx, line.id, line.order_id, line.menu_item_id)?;
    tx.commit().map_err(errors::internal)?;
    Ok(line)
}

/// End the weighing in progress, if any: it answers `scale_cancelled` and
/// nothing is added.
pub fn do_scale_cancel_weighing(state: &AppState) -> Result<(), String> {
    access::operator(state, Access::Staff)?;
    let section = state.config.lock().map_err(errors::internal)?.scale.clone();
    if let Some(section) = section {
        driver(state, &section)?.cancel();
    }
    Ok(())
}

/// Whether the scale answers. Runs its software check if it asks for it.
/// Never talks to it during a weighing.
pub async fn do_get_scale_status(state: &AppState) -> Result<ScaleStatus, String> {
    let section = state.config.lock().map_err(errors::internal)?.scale.clone();
    let Some(section) = section else {
        return Ok(ScaleStatus {
            configured: false,
            connected: false,
            busy: false,
            check: "unknown".to_string(),
        });
    };
    let driver = driver(state, &section)?;
    let busy = driver.is_busy();
    let connected = if busy {
        true
    } else {
        let probe = driver.probe().await;
        log_checks(state, &probe.checks);
        !matches!(probe.result, Err(Failure::Unreachable))
    };
    let check = {
        let db = state.db.lock().map_err(errors::internal)?;
        pos_db::queries::scale::last_check_verdict(&db).map_err(errors::internal)?
    };
    Ok(ScaleStatus {
        configured: true,
        connected,
        busy,
        check: match check.as_deref() {
            Some("passed") => "ok",
            Some("refused") => "refused",
            Some("no_values") => "no_values",
            _ => "unknown",
        }
        .to_string(),
    })
}
