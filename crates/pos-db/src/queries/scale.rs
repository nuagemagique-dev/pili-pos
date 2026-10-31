//! The local audit of the scale: what it returned for each weighed line,
//! and the steps of its software check. Never part of an FDM message.

use rusqlite::{Connection, params};

/// What the scale returned for a line.
#[derive(Debug)]
pub struct NewWeighing<'a> {
    pub line_id: i64,
    pub order_id: i64,
    pub weight_grams: i64,
    pub unit_price_cents: i64,
    pub amount_cents: i64,
    pub tare_grams: Option<i64>,
    pub operator_insz: &'a str,
}

pub fn record_weighing(conn: &Connection, w: &NewWeighing<'_>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO scale_weighings (line_id, order_id, weight_grams, unit_price_cents,
                                      amount_cents, tare_grams, operator_insz)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            w.line_id,
            w.order_id,
            w.weight_grams,
            w.unit_price_cents,
            w.amount_cents,
            w.tare_grams,
            w.operator_insz,
        ],
    )?;
    Ok(())
}

/// One step of the software check: `requested`, `answered`, `passed`,
/// `refused` or `no_values`.
pub fn record_check(conn: &Connection, event: &str, detail: Option<&str>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO scale_check_log (event, detail) VALUES (?1, ?2)",
        params![event, detail],
    )?;
    Ok(())
}

/// The latest verdict of the software check, for the status light.
pub fn last_check_verdict(conn: &Connection) -> rusqlite::Result<Option<String>> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT event FROM scale_check_log WHERE event IN ('passed','refused','no_values')
         ORDER BY id DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()
}
