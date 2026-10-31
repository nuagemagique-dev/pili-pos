//! Kitchen notes on order lines (migration 31, `do_set_line_note`):
//! the validation, the rule that a note is local data and never part of
//! any fiscal message or ticket (producers FAQ Q210 b, POS v2.2 8.1),
//! the note printed under its item on the bar / kitchen slip, and the
//! note slip for a note changed after the line went to the kitchen.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_list_cart_lines,
    do_remove_order_line, do_set_line_note, do_update_order_line_quantity,
};
use pos_engine::commands::table_transfer::do_transfer_table;
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};

const OPERATOR: &str = "12345678958";
const NOTE: &str = "sans wasabi";

/// Catalogue: 1 = Sake (bar), 2 = Maki (kitchen), 3 = Eau plate (off
/// the slip).
async fn app() -> (TestApp, LossyFdm) {
    let fdm = LossyFdm::start().await;
    let app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1),
                (2, 1, 1200, 'B', 1, 1, 1),
                (3, 1, 300, 'A', 0, 2, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake'),
                ('menu_item', 2, 'name', 'fr', 'Maki saumon'),
                ('menu_item', 3, 'name', 'fr', 'Eau plate');",
        )
        .unwrap();
    (app, fdm)
}

fn order(app: &TestApp, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id
}

fn add(app: &TestApp, order: i64, item: i64, quantity: i32) -> i64 {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: item,
            quantity,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap()
    .id
}

async fn note(app: &TestApp, line: i64, text: &str) -> Result<SlipOutcome, String> {
    do_set_line_note(&app.state, line, Some(text.to_string())).await
}

/// `(kitchen_note, kitchen_note_printed)` of a line.
fn stored(app: &TestApp, line: i64) -> (Option<String>, Option<String>) {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT kitchen_note, kitchen_note_printed FROM order_lines WHERE id = ?1",
            [line],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
}

fn rendered(slips: &[String]) -> Vec<String> {
    slips
        .iter()
        .map(|s| pos_print::render::render_plain_text(s.as_bytes()))
        .collect()
}

fn journal_len(app: &TestApp) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM order_entries", [], |r| r.get(0))
        .unwrap()
}

// ── Validation ─────────────────────────────────────────────────────

#[tokio::test]
async fn a_note_is_trimmed_and_an_empty_one_clears_it() {
    let (app, _fdm) = app().await;
    let o = order(&app, Some(1));
    let line = add(&app, o, 1, 1);
    let entries = journal_len(&app);

    assert_eq!(
        note(&app, line, "sans\twasabi").await,
        Err(errors::NOTE_UNPRINTABLE.to_string()),
        "a tab inside the text is a control character"
    );
    assert_eq!(
        note(&app, line, "   sans wasabi   ").await,
        Ok(SlipOutcome::NothingToPrint)
    );
    assert_eq!(stored(&app, line).0.as_deref(), Some(NOTE));
    // Blank text, then None, clear it.
    note(&app, line, "   ").await.unwrap();
    assert_eq!(stored(&app, line).0, None);
    note(&app, line, NOTE).await.unwrap();
    do_set_line_note(&app.state, line, None).await.unwrap();
    assert_eq!(stored(&app, line).0, None);
    // A note is not an order change: nothing is journalled or signed.
    assert_eq!(journal_len(&app), entries);
    assert!(app.fiscal_events().is_empty());
}

#[tokio::test]
async fn the_length_limit_is_60_characters_after_trimming() {
    let (app, _fdm) = app().await;
    let o = order(&app, Some(1));
    let line = add(&app, o, 1, 1);

    let sixty = "x".repeat(60);
    note(&app, line, &format!("  {sixty}  ")).await.unwrap();
    assert_eq!(stored(&app, line).0.as_deref(), Some(sixty.as_str()));
    assert_eq!(
        note(&app, line, &"x".repeat(61)).await,
        Err(errors::NOTE_TOO_LONG.to_string())
    );
    // Characters, not bytes: 60 accented letters fit.
    let accented = "é".repeat(60);
    note(&app, line, &accented).await.unwrap();
    // A refusal leaves the previous note alone.
    assert_eq!(stored(&app, line).0.as_deref(), Some(accented.as_str()));
}

#[tokio::test]
async fn only_what_the_slip_printer_prints_is_accepted() {
    let (app, _fdm) = app().await;
    let o = order(&app, Some(1));
    let line = add(&app, o, 1, 1);

    for ok in [
        "crème brûlée, café",
        "bœuf haché",
        "l\u{2019}eau",
        "50% (sans)",
    ] {
        note(&app, line, ok)
            .await
            .unwrap_or_else(|e| panic!("{ok}: {e}"));
    }
    for bad in [
        "不要芥末",
        "line\nbreak",
        "\u{1b}@",
        "sushi \u{1F363}",
        "a\u{7f}b",
    ] {
        assert_eq!(
            note(&app, line, bad).await,
            Err(errors::NOTE_UNPRINTABLE.to_string()),
            "{bad:?}"
        );
    }
    assert_eq!(stored(&app, line).0.as_deref(), Some("50% (sans)"));
}

#[tokio::test]
async fn a_note_needs_a_line_of_an_open_till_order() {
    let (app, _fdm) = app().await;
    assert_eq!(
        note(&app, 9999, NOTE).await,
        Err(errors::LINE_NOT_FOUND.to_string())
    );

    // An online order.
    let online = order(&app, Some(2));
    let online_line = add(&app, online, 1, 1);
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE orders SET hubrise_order_id = 'ONLINE-1' WHERE id = ?1",
            [online],
        )
        .unwrap();
    assert_eq!(
        note(&app, online_line, NOTE).await,
        Err(errors::ORDER_NOT_OPEN.to_string())
    );

    // A closed order.
    let closed = order(&app, Some(3));
    let closed_line = add(&app, closed, 1, 1);
    assert!(
        do_checkout(&app.state, closed, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        note(&app, closed_line, NOTE).await,
        Err(errors::ORDER_NOT_OPEN.to_string())
    );
    assert_eq!(stored(&app, closed_line).0, None);
}

#[tokio::test]
async fn an_order_waiting_for_the_fdm_keeps_its_notes() {
    let (app, fdm) = app().await;
    fdm.lose_first("signSale");
    let o = order(&app, Some(1));
    let line = add(&app, o, 1, 1);
    let _ = do_checkout(&app.state, o, CheckoutPaymentMethod::Card).await;
    assert_eq!(
        note(&app, line, NOTE).await,
        Err(errors::ORDER_SIGN_PENDING.to_string())
    );
}

#[tokio::test]
async fn a_note_needs_a_logged_in_operator() {
    let (app, _fdm) = app().await;
    let o = order(&app, Some(1));
    let line = add(&app, o, 1, 1);
    *app.state.session.lock().unwrap() = None;
    assert_eq!(
        note(&app, line, NOTE).await,
        Err(errors::NOT_LOGGED_IN.to_string())
    );
}

#[tokio::test]
async fn the_cart_shows_the_note_and_a_removed_line_takes_it_away() {
    let (app, _fdm) = app().await;
    let o = order(&app, Some(1));
    let with = add(&app, o, 1, 1);
    let without = add(&app, o, 1, 1);
    note(&app, with, NOTE).await.unwrap();

    let cart = do_list_cart_lines(&app.state, o).unwrap();
    let find = |id: i64| cart.iter().find(|l| l.line.id == id).unwrap();
    assert_eq!(find(with).kitchen_note.as_deref(), Some(NOTE));
    assert_eq!(find(without).kitchen_note, None);
    // The serialized cart line carries it for the UIs.
    let json = serde_json::to_value(find(with)).unwrap();
    assert_eq!(json["kitchen_note"], NOTE);

    do_remove_order_line(&app.state, with).unwrap();
    assert!(
        do_list_cart_lines(&app.state, o)
            .unwrap()
            .iter()
            .all(|l| l.kitchen_note.is_none())
    );
}

// ── A note is never fiscal data ────────────────────────────────────

/// Every text column of every table, as `table.column`, that holds
/// `needle`.
fn columns_holding(app: &TestApp, needle: &str) -> Vec<String> {
    let db = app.state.db.lock().unwrap();
    let tables: Vec<String> = db
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut found = Vec::new();
    for table in tables {
        let columns: Vec<String> = db
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for column in columns {
            let hits: i64 = db
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM \"{table}\" WHERE CAST(\"{column}\" AS TEXT) LIKE ?1"
                    ),
                    [format!("%{needle}%")],
                    |r| r.get(0),
                )
                .unwrap();
            if hits > 0 {
                found.push(format!("{table}.{column}"));
            }
        }
    }
    found
}

#[tokio::test]
async fn a_note_is_in_no_fiscal_message_no_ticket_and_no_other_table() {
    let (app, fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let secret = "zzsecretnote";

    // A table order: noted, registered (P), pre-billed, moved, paid (N).
    let o = order(&app, Some(4));
    let sake = add(&app, o, 1, 2);
    let maki = add(&app, o, 2, 1);
    note(&app, sake, secret).await.unwrap();
    note(&app, maki, "allergie arachide").await.unwrap();
    do_sign_table_order(&app.state, o).await.unwrap();
    do_sign_pre_bill(&app.state, o).await.unwrap();
    do_transfer_table(&app.state, o, 9).await.unwrap();
    let paid = do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(paid.success, "{:?}", paid.error);

    // A takeaway: the N is the only event.
    let t = order(&app, None);
    let tl = add(&app, t, 1, 1);
    note(&app, tl, secret).await.unwrap();
    assert!(
        do_checkout(&app.state, t, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );

    // All event kinds were signed: P (order), P (pre-bill), P (move), N.
    let operations: Vec<String> = fdm.sent().into_iter().map(|s| s.op).collect();
    for expected in [
        "signOrder",
        "signPreBill",
        "signCostCenterChange",
        "signSale",
    ] {
        assert!(
            operations.iter().any(|o| o == expected),
            "{expected} in {operations:?}"
        );
    }
    // Nothing the FDM was sent, nor what it answered, nor the archive.
    for sent in fdm.sent() {
        assert!(!sent.body.contains(secret), "{} request", sent.op);
        assert!(!sent.body.contains("arachide"), "{} request", sent.op);
    }
    for event in app.fiscal_events() {
        let request = event.event_data_json.as_deref().unwrap_or_default();
        let response = event.fdm_response_json.as_deref().unwrap_or_default();
        assert!(!request.contains(secret) && !request.contains("arachide"));
        assert!(!response.contains(secret) && !response.contains("arachide"));
    }
    // The only columns that hold a note are the line's two.
    let mut holders = columns_holding(&app, secret);
    holders.sort();
    assert_eq!(
        holders,
        [
            "order_lines.kitchen_note",
            "order_lines.kitchen_note_printed"
        ],
        "{holders:?}"
    );

    // Printed: bar slips carry it; no VAT ticket, pre-bill or other
    // receipt does.
    let slips = printer.settle_slips(3).await;
    assert!(
        rendered(&slips).iter().any(|s| s.contains(secret)),
        "{slips:?}"
    );
    let receipts = printer.settle(1).await;
    assert_ne!(receipts.len(), 0);
    for receipt in rendered(&receipts) {
        assert!(
            !receipt.contains(secret) && !receipt.contains("arachide"),
            "{receipt}"
        );
    }
    for document in printer.rendered() {
        if !document.contains("BON DE COMMANDE") {
            assert!(!document.contains(secret), "{document}");
        }
    }
}

// ── Printing ───────────────────────────────────────────────────────

#[tokio::test]
async fn the_note_prints_under_its_item_on_the_slip_after_the_order_is_sent() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(4));
    let with = add(&app, o, 1, 2);
    let without = add(&app, o, 2, 1);
    // Set before the line is sent: no slip of its own.
    assert_eq!(
        note(&app, with, NOTE).await,
        Ok(SlipOutcome::NothingToPrint)
    );
    assert_eq!(stored(&app, with), (Some(NOTE.into()), None));

    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = rendered(&printer.settle_slips(1).await);
    assert_eq!(slips.len(), 1, "{slips:?}");
    let lines: Vec<&str> = slips[0].lines().map(str::trim_end).collect();
    let at = lines.iter().position(|l| l.contains("2 x Sake")).unwrap();
    assert_eq!(lines[at + 1], "   > sans wasabi", "{}", slips[0]);
    let maki = lines
        .iter()
        .position(|l| l.contains("1 x Maki saumon"))
        .unwrap();
    assert!(!lines[maki + 1].contains('>'), "{}", slips[0]);
    assert!(!slips[0].contains("6,50") && !slips[0].contains("12,00"));
    // The printed copy follows, the other line has none.
    assert_eq!(stored(&app, with), (Some(NOTE.into()), Some(NOTE.into())));
    assert_eq!(stored(&app, without), (None, None));
}

#[tokio::test]
async fn more_of_a_noted_line_prints_the_item_with_its_note_again() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(2));
    let line = add(&app, o, 1, 1);
    note(&app, line, NOTE).await.unwrap();
    do_sign_table_order(&app.state, o).await.unwrap();

    do_update_order_line_quantity(&app.state, line, 3).unwrap();
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = rendered(&printer.settle_slips(2).await);
    assert_eq!(slips.len(), 2, "{slips:?}");
    assert!(
        slips[1].contains("2 x Sake") && slips[1].contains("> sans wasabi"),
        "{}",
        slips[1]
    );

    // Taking some away prints the correction without the note.
    do_update_order_line_quantity(&app.state, line, 2).unwrap();
    do_sign_table_order(&app.state, o).await.unwrap();
    let slips = rendered(&printer.settle_slips(3).await);
    assert!(slips[2].contains("-1 x Sake"), "{}", slips[2]);
    assert!(!slips[2].contains("wasabi"), "{}", slips[2]);
}

#[tokio::test]
async fn a_note_changed_after_sending_prints_one_note_slip_and_none_when_unchanged() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(7));
    let line = add(&app, o, 2, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    // New note on a line already in the kitchen: one note slip.
    assert_eq!(note(&app, line, NOTE).await, Ok(SlipOutcome::Printed));
    let slips = rendered(&printer.settle_slips(2).await);
    assert_eq!(slips.len(), 2, "{slips:?}");
    let slip = &slips[1];
    for expected in [
        "MODIFICATION",
        "Table 7",
        "Maki saumon",
        "Note : sans wasabi",
        "CECI N'EST PAS UN TICKET DE CAISSE",
    ] {
        assert!(slip.contains(expected), "{expected:?} in {slip}");
    }
    assert!(!slip.contains(" x "), "{slip}");
    assert!(
        !slip.contains("12,00") && !slip.contains("EUR"),
        "no amount: {slip}"
    );
    assert_eq!(stored(&app, line), (Some(NOTE.into()), Some(NOTE.into())));

    // The same note again: nothing printed.
    assert_eq!(
        note(&app, line, NOTE).await,
        Ok(SlipOutcome::NothingToPrint)
    );
    assert_eq!(
        note(&app, line, "  sans wasabi ").await,
        Ok(SlipOutcome::NothingToPrint)
    );
    assert_eq!(printer.settle_slips(3).await.len(), 2);

    // A different note: one more slip.
    assert_eq!(
        note(&app, line, "extra gingembre").await,
        Ok(SlipOutcome::Printed)
    );
    // A removal says so, once.
    assert_eq!(
        do_set_line_note(&app.state, line, None).await,
        Ok(SlipOutcome::Printed)
    );
    assert_eq!(
        do_set_line_note(&app.state, line, None).await,
        Ok(SlipOutcome::NothingToPrint)
    );
    let slips = rendered(&printer.settle_slips(4).await);
    assert_eq!(slips.len(), 4, "{slips:?}");
    assert!(slips[2].contains("Note : extra gingembre"), "{}", slips[2]);
    assert!(slips[3].contains("Note supprimee"), "{}", slips[3]);
    assert_eq!(stored(&app, line), (None, None));
    // No fiscal event for any of it: just the P of the first sending.
    assert_eq!(app.fiscal_events().len(), 1);
}

#[tokio::test]
async fn a_failed_note_slip_is_returned_and_confirming_again_retries() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(5));
    let line = add(&app, o, 1, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    // The printer is gone: the note is stored, the failure is returned
    // and the printed copy stays stale.
    app.set_printer(DEAD_PRINTER);
    assert_eq!(note(&app, line, NOTE).await, Ok(SlipOutcome::Failed));
    assert_eq!(stored(&app, line), (Some(NOTE.into()), None));

    // Back: confirming the same note again prints it.
    app.set_printer(&printer.addr);
    assert_eq!(note(&app, line, NOTE).await, Ok(SlipOutcome::Printed));
    assert_eq!(stored(&app, line), (Some(NOTE.into()), Some(NOTE.into())));
    assert_eq!(
        note(&app, line, NOTE).await,
        Ok(SlipOutcome::NothingToPrint)
    );
    let slips = rendered(&printer.settle_slips(2).await);
    assert_eq!(slips.len(), 2, "{slips:?}");
    assert!(slips[1].contains("Note : sans wasabi"), "{}", slips[1]);
}

#[tokio::test]
async fn without_a_printer_the_note_is_stored_and_the_slip_reports_it() {
    let (app, _fdm) = app().await;
    let o = order(&app, Some(5));
    let line = add(&app, o, 1, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(note(&app, line, NOTE).await, Ok(SlipOutcome::NoPrinter));
    assert_eq!(stored(&app, line), (Some(NOTE.into()), None));
}

#[tokio::test]
async fn a_failed_order_slip_leaves_the_note_unprinted_for_a_retry() {
    let (app, _fdm) = app().await;
    app.set_printer(DEAD_PRINTER);
    let o = order(&app, Some(5));
    let line = add(&app, o, 1, 1);
    note(&app, line, NOTE).await.unwrap();
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Failed
    );
    // The order is sent but the note never reached the kitchen: the
    // printed copy is still empty, so confirming it prints the note.
    assert_eq!(stored(&app, line), (Some(NOTE.into()), None));
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    assert_eq!(note(&app, line, NOTE).await, Ok(SlipOutcome::Printed));
}

#[tokio::test]
async fn a_product_off_the_kitchen_has_no_note_slip() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(5));
    let line = add(&app, o, 3, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(
        note(&app, line, NOTE).await,
        Ok(SlipOutcome::NothingToPrint)
    );
    assert_eq!(printer.settle_slips(1).await.len(), 0);
}

#[tokio::test]
async fn a_takeaway_note_prints_with_the_slip_of_the_sale() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let t = order(&app, None);
    let line = add(&app, t, 1, 1);
    add(&app, t, 2, 1);
    // Nothing was sent: no note slip, the sale's slip carries it.
    assert_eq!(
        note(&app, line, NOTE).await,
        Ok(SlipOutcome::NothingToPrint)
    );
    let r = do_checkout(&app.state, t, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(r.kitchen_slip, Some(SlipOutcome::Printed));

    let slips = rendered(&printer.settle_slips(1).await);
    assert_eq!(slips.len(), 1, "{slips:?}");
    assert!(slips[0].contains("Vente directe"), "{}", slips[0]);
    let lines: Vec<&str> = slips[0].lines().map(str::trim_end).collect();
    let at = lines.iter().position(|l| l.contains("1 x Sake")).unwrap();
    assert_eq!(lines[at + 1], "   > sans wasabi", "{}", slips[0]);
    assert_eq!(stored(&app, line), (Some(NOTE.into()), Some(NOTE.into())));
}

#[tokio::test]
async fn the_note_slip_follows_the_ticket_language_and_training() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    {
        let mut config = app.state.config.lock().unwrap();
        config.language = "nl".into();
        config.training_mode = true;
    }
    let o = order(&app, Some(5));
    let line = add(&app, o, 1, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(
        note(&app, line, "zonder wasabi").await,
        Ok(SlipOutcome::Printed)
    );
    let slips = rendered(&printer.settle_slips(2).await);
    assert_eq!(slips.len(), 2, "{slips:?}");
    assert!(
        slips[1].contains("WIJZIGING") && slips[1].contains("Tafel 5"),
        "{}",
        slips[1]
    );
    assert!(
        slips[1].contains("Opmerking: zonder wasabi"),
        "{}",
        slips[1]
    );
    assert!(slips[1].contains("TRAINING"), "{}", slips[1]);
}

// ── Access ─────────────────────────────────────────────────────────

#[tokio::test]
async fn a_waiter_sets_a_note_and_the_note_slip_names_the_waiter() {
    const WAITER: &str = "85073003328";
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(7));
    let line = add(&app, o, 2, 1);
    app.login_as(WAITER, pos_core::user::UserRole::Waiter);

    // Before the first sending: stored, nothing to print.
    assert_eq!(
        note(&app, line, NOTE).await,
        Ok(SlipOutcome::NothingToPrint)
    );
    assert_eq!(stored(&app, line).0.as_deref(), Some(NOTE));

    // After the sending, a change by the waiter prints a note slip.
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);
    assert_eq!(
        note(&app, line, "extra gingembre").await,
        Ok(SlipOutcome::Printed)
    );
    let slips = rendered(&printer.settle_slips(2).await);
    assert_eq!(slips.len(), 2, "{slips:?}");
    assert!(slips[1].contains("MODIFICATION"), "{}", slips[1]);
    assert!(slips[1].contains("Note : extra gingembre"), "{}", slips[1]);
    assert!(
        slips[1].contains(&format!("Test {WAITER}")),
        "the waiter is the operator: {}",
        slips[1]
    );
}
