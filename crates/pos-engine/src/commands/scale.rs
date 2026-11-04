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
//! A product with options (a malatang bowl and its broth) takes them with
//! the weighing, picked before the scale is asked: the bowl stays a weighed
//! line on its own, each paid option becomes a line of its own sold by the
//! piece and linked to its bowl (FAQ Q201 b, migration 81), and every option
//! is kept on the bowl for the kitchen. All of it is written in the same
//! transaction, so there is never a bowl without its broth.
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

impl std::fmt::Debug for ScaleSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScaleSlot").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScaleWeighInput {
    pub order_id: i64,
    pub menu_item_id: i64,
    /// The options picked for the product (its broth, its spice level),
    /// checked against its option groups before the scale is asked.
    #[serde(default)]
    pub modifier_ids: Vec<i64>,
}

/// An option picked with a weighed product.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BowlOption {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
}

/// The options `ids` names among the product's option groups, each group's
/// count within its minimum and maximum (`modifiers::pick`).
fn bowl_options(
    db: &rusqlite::Connection,
    menu_item_id: i64,
    ids: &[i64],
    lang: &str,
) -> Result<Vec<BowlOption>, String> {
    let groups = pos_db::queries::menu::get_modifier_groups(db, menu_item_id, lang)
        .map_err(errors::internal)?;
    Ok(super::modifiers::pick(&groups, ids)?
        .into_iter()
        .map(|p| BowlOption {
            id: p.id,
            name: p.name.to_string(),
            price_cents: p.price_cents,
        })
        .collect())
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
        return Ok(Arc::clone(driver));
    }
    let driver = Arc::new(ScaleDriver::new(
        driver_config(config),
        config.tcp_address.clone(),
    ));
    *slot = Some((config.clone(), Arc::clone(&driver)));
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
    let (product, options) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let order_type = check_order(&db, input.order_id)?;
        super::sold_out::refuse_if_sold_out(&db, input.menu_item_id)?;
        let product = pos_db::queries::menu::resolve_catalogue_line(
            &db,
            input.menu_item_id,
            pos_db::queries::menu::Service::from(order_type == OrderType::Takeaway),
            lang,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => errors::coded(
                errors::PRODUCT_UNAVAILABLE,
                &[("product", &input.menu_item_id)],
            ),
            other => errors::internal(other),
        })?;
        let options = bowl_options(&db, input.menu_item_id, &input.modifier_ids, lang)?;
        (product, options)
    };
    if !product.sold_by_weight {
        return Err(errors::SCALE_NOT_SOLD_BY_WEIGHT.into());
    }
    let request = WeighRequest {
        unit_price_cents: product.price_cents,
        tare_grams: product.scale_tare_grams,
        text: scale_text(&product.name),
    };
    let events = Arc::clone(&state.events);
    let outcome = driver
        .weigh(request, move |progress| {
            let step = match progress {
                Progress::Waiting => "waiting",
                Progress::Stabilizing => "stabilizing",
                Progress::Checking => "checking",
            };
            events.emit(EngineEvent::ScaleProgress(ScaleProgress {
                stage: step.to_string(),
            }));
        })
        .await;
    log_checks(state, &outcome.checks);
    let reading = outcome
        .result
        .map_err(|failure| failure_code(failure).to_string())?;

    let db = state.db.lock().map_err(errors::internal)?;
    check_order(&db, input.order_id)?;
    let modifiers_json = options_json(&options)?;
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
            modifiers_json: modifiers_json.as_deref(),
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
    add_bowl_options(&tx, &line, &product, &options)?;
    tx.commit().map_err(errors::internal)?;
    Ok(line)
}

/// The options picked with a weighed product as its line keeps them, or
/// `None` without any.
fn options_json(options: &[BowlOption]) -> Result<Option<String>, String> {
    (!options.is_empty())
        .then(|| serde_json::to_string(options))
        .transpose()
        .map_err(errors::internal)
}

/// The paid options of the weighed `bowl`: each a line of its own, by the
/// piece, at the bowl's VAT (FAQ Q201 b), linked to the bowl and in its
/// course. A free one (a spice level) stays on the bowl only.
fn add_bowl_options(
    tx: &rusqlite::Connection,
    bowl: &OrderLine,
    product: &pos_db::queries::menu::CatalogueLine,
    options: &[BowlOption],
) -> Result<(), String> {
    for option in options.iter().filter(|o| o.price_cents > 0) {
        let line = pos_db::queries::orders::add_order_line(
            tx,
            pos_db::queries::orders::NewOrderLine {
                order_id: bowl.order_id,
                menu_item_id: bowl.menu_item_id,
                quantity: 1,
                unit_price_cents: option.price_cents,
                vat_code: product.vat_code,
                name: &option.name,
                modifiers_json: None,
                department_id: &product.department_id,
                department_name: &product.department_name,
            },
        )
        .map_err(errors::internal)?;
        pos_db::queries::bowls::link_option(tx, line.id, bowl.id, bowl.order_id)
            .map_err(errors::internal)?;
        super::courses::assign_new_line(tx, line.id, bowl.order_id, bowl.menu_item_id)?;
    }
    Ok(())
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
