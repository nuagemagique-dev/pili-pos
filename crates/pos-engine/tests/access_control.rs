//! Who may do what (`PROFILS_UTILISATEURS_FR.md` §2–3), enforced by the
//! backend whatever the UI shows:
//!
//! - nobody logged in: every operator command is refused;
//! - apprentice (`waiter`): clock in/out, orders and lines, table order;
//! - server (`cashier`): + pre-bill, sale;
//! - manager (`admin`): + refund, copy, cash events, reports, training
//!   mode, users, backup;
//! - a command naming an operator must name the logged-in one;
//! - a refused command signs nothing: no FDM call, no archived event,
//!   no counter moved.
//!
//! Events are signed by the admission image's mock FDM.

use pos_engine_testkit as common;

use common::*;

use pos_engine::commands::errors;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN, ERR_NOT_THE_OPERATOR};
use pos_engine::commands::auth::{
    CreateUserInput, ERR_DEACTIVATE_SELF, ERR_PIN_ALREADY_SET, ERR_PIN_FORMAT,
    ERR_PIN_SETUP_REQUIRED, ERR_SYSTEM_USER, do_create_user, do_deactivate_user, do_login,
    do_login_with_new_pin, do_logout, do_set_pin,
};
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_remove_order_line,
    do_update_order_line_quantity,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::{
    do_backup_database, do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
};
use pos_engine::commands::settings::{ERR_OPEN_ORDERS, do_get_settings, do_set_training_mode};
use pos_engine::commands::social::{do_work_in, do_work_out};
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_engine::commands::unpaid::do_close_unpaid;

const MANAGER: &str = "94060242576";
const SERVER: &str = "84022899837";
const APPRENTICE: &str = "12345678958";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
            VALUES (1, 1, 1000, 'B', 0, 1);
         INSERT INTO translations (entity_type, entity_id, field, lang, value)
            VALUES ('menu_item', 1, 'name', 'fr', 'Burger');",
    )
    .unwrap();
    drop(conn);
    app
}

fn as_(app: &TestApp, insz: &str, role: UserRole) {
    app.login_as(insz, role);
}

/// An open table order with one line, created by the logged-in user.
fn table(app: &TestApp, insz: &str, table: i32) -> i64 {
    let id = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(table),
            user_insz: insz.into(),
        },
    )
    .unwrap()
    .id;
    add_line(app, id).unwrap();
    id
}

fn add_line(app: &TestApp, order_id: i64) -> Result<i64, String> {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .map(|l| l.id)
}

/// The manager's PIN (closing a sale unpaid asks for it).
const MANAGER_PIN: &str = "4321";
/// The table whose customer left without paying.
const LEFT_UNPAID: i32 = 21;

/// A paid sale, signed by a manager; returns its fiscal event id. The
/// manager also has a PIN and table 21 is left open, for the
/// unpaid-sale command.
async fn paid_sale(app: &TestApp) -> i64 {
    as_(app, MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), MANAGER_PIN.into(), None).unwrap();
    table(app, MANAGER, LEFT_UNPAID);
    let id = table(app, MANAGER, 20);
    let r = do_checkout(&app.state, id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

/// Every manager-only fiscal command, run as `insz`; each result is
/// `Err(message)` when refused.
async fn manager_commands(
    app: &TestApp,
    insz: &str,
    sale: i64,
) -> Vec<(&'static str, Result<(), String>)> {
    let refund = do_refund_checkout(
        &app.state,
        sale,
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        insz.into(),
    )
    .await
    .unwrap();
    let copy = do_copy_ticket(&app.state, sale, insz.into()).await.unwrap();
    let left_unpaid: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT id FROM orders WHERE table_number = ?1 AND status = 'open'",
            [LEFT_UNPAID],
            |r| r.get(0),
        )
        .unwrap();
    let unpaid = do_close_unpaid(&app.state, left_unpaid, insz, MANAGER_PIN)
        .await
        .unwrap();
    let flat = |r: Result<bool, String>| r.and_then(|ok| ok.then_some(()).ok_or("failed".into()));
    vec![
        (
            "refund",
            refund
                .success
                .then_some(())
                .ok_or(refund.error.unwrap_or_default()),
        ),
        (
            "copy",
            copy.success
                .then_some(())
                .ok_or(copy.error.unwrap_or_default()),
        ),
        (
            "unpaid",
            unpaid
                .success
                .then_some(())
                .ok_or(unpaid.error.unwrap_or_default()),
        ),
        (
            "money in",
            flat(
                do_money_in_out(
                    &app.state,
                    insz.into(),
                    1000,
                    MoneyDirection::In,
                    "Fond".into(),
                )
                .await
                .map(|r| r.success),
            ),
        ),
        (
            "drawer",
            flat(
                do_open_drawer_fiscal(&app.state, insz.into())
                    .await
                    .map(|r| r.success),
            ),
        ),
        (
            "X",
            flat(
                do_generate_x_report(&app.state, insz.into())
                    .await
                    .map(|r| r.success),
            ),
        ),
        (
            "user X",
            flat(
                do_generate_user_x_report(&app.state, insz.into())
                    .await
                    .map(|r| r.success),
            ),
        ),
        (
            "Z",
            flat(
                do_generate_z_report(&app.state, insz.into())
                    .await
                    .map(|r| r.success),
            ),
        ),
    ]
}

// ── Nobody logged in ───────────────────────────────────────────

#[tokio::test]
async fn nothing_is_signed_without_a_logged_in_operator() {
    let app = app().await;
    let sale = paid_sale(&app).await;
    as_(&app, MANAGER, UserRole::Admin);
    let open = table(&app, MANAGER, 1);
    let before = app.fiscal_events().len();
    app.logout();

    for (what, result) in manager_commands(&app, MANAGER, sale).await {
        assert_eq!(result.unwrap_err(), ERR_NOT_LOGGED_IN, "{what}");
    }
    let r = do_checkout(&app.state, open, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert_eq!(r.error.as_deref(), Some(ERR_NOT_LOGGED_IN));
    assert_eq!(
        do_sign_table_order(&app.state, open).await.unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_sign_pre_bill(&app.state, open).await.unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_work_in(&app.state, MANAGER.into()).await.unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_work_out(&app.state, MANAGER.into()).await.unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(add_line(&app, open).unwrap_err(), ERR_NOT_LOGGED_IN);
    assert_eq!(
        do_remove_order_line(&app.state, 1).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_update_order_line_quantity(&app.state, 1, 2).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_set_training_mode(&app.state, true).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_backup_database(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );

    assert_eq!(app.fiscal_events().len(), before, "nothing archived");
    assert!(!do_get_settings(&app.state).unwrap().training_mode);
}

// ── Apprentice ─────────────────────────────────────────────────

#[tokio::test]
async fn an_apprentice_takes_orders_but_never_payments() {
    let app = app().await;
    let sale = paid_sale(&app).await;
    as_(&app, APPRENTICE, UserRole::Waiter);

    do_work_in(&app.state, APPRENTICE.into()).await.unwrap();
    let open = table(&app, APPRENTICE, 1);
    let line = add_line(&app, open).unwrap();
    do_update_order_line_quantity(&app.state, line, 3).unwrap();
    do_sign_table_order(&app.state, open).await.unwrap();
    let before = app.fiscal_events().len();

    assert_eq!(
        do_sign_pre_bill(&app.state, open).await.unwrap_err(),
        ERR_FORBIDDEN
    );
    let r = do_checkout(&app.state, open, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!r.success);
    assert_eq!(r.error.as_deref(), Some(ERR_FORBIDDEN));
    for (what, result) in manager_commands(&app, APPRENTICE, sale).await {
        assert_eq!(result.unwrap_err(), ERR_FORBIDDEN, "{what}");
    }
    assert_eq!(
        do_set_training_mode(&app.state, true).unwrap_err(),
        ERR_FORBIDDEN
    );
    assert_eq!(do_backup_database(&app.state).unwrap_err(), ERR_FORBIDDEN);
    assert_eq!(app.fiscal_events().len(), before, "refusals sign nothing");
    assert_eq!(app.order_status(open).as_deref(), Some("open"));

    do_work_out(&app.state, APPRENTICE.into()).await.unwrap();
    let labels: Vec<_> = app
        .fiscal_events()
        .iter()
        .filter(|e| e.user_insz == APPRENTICE)
        .map(|e| e.event_type.clone())
        .collect();
    assert_eq!(labels, ["S", "P", "S"]);
}

// ── Server ─────────────────────────────────────────────────────

#[tokio::test]
async fn a_server_bills_and_takes_payment_but_nothing_more() {
    let app = app().await;
    let sale = paid_sale(&app).await;
    as_(&app, SERVER, UserRole::Cashier);

    let open = table(&app, SERVER, 1);
    do_sign_pre_bill(&app.state, open).await.unwrap();
    let r = do_checkout(&app.state, open, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let before = app.fiscal_events().len();

    for (what, result) in manager_commands(&app, SERVER, sale).await {
        assert_eq!(result.unwrap_err(), ERR_FORBIDDEN, "{what}");
    }
    assert_eq!(
        do_set_training_mode(&app.state, true).unwrap_err(),
        ERR_FORBIDDEN
    );
    assert_eq!(app.fiscal_events().len(), before);
}

// ── Manager ────────────────────────────────────────────────────

#[tokio::test]
async fn a_manager_may_do_everything() {
    let app = app().await;
    let sale = paid_sale(&app).await;
    for (what, result) in manager_commands(&app, MANAGER, sale).await {
        assert!(result.is_ok(), "{what}: {result:?}");
    }
    do_backup_database(&app.state).ok(); // no backup dir in tests: only the access matters
    assert_ne!(
        do_backup_database(&app.state).err().as_deref(),
        Some(ERR_FORBIDDEN)
    );
}

// ── Acting for someone else ────────────────────────────────────

#[tokio::test]
async fn nobody_signs_in_someone_elses_name() {
    let app = app().await;
    let sale = paid_sale(&app).await;
    as_(&app, MANAGER, UserRole::Admin);
    let before = app.fiscal_events().len();

    assert_eq!(
        do_work_in(&app.state, SERVER.into()).await.unwrap_err(),
        ERR_NOT_THE_OPERATOR
    );
    assert_eq!(
        do_work_out(&app.state, SERVER.into()).await.unwrap_err(),
        ERR_NOT_THE_OPERATOR
    );
    assert_eq!(
        do_create_order(
            &app.state,
            CreateOrderInput {
                order_type: OrderType::Takeaway,
                table_number: None,
                user_insz: SERVER.into(),
            },
        )
        .unwrap_err(),
        ERR_NOT_THE_OPERATOR
    );
    for (what, result) in manager_commands(&app, SERVER, sale).await {
        assert_eq!(result.unwrap_err(), ERR_NOT_THE_OPERATOR, "{what}");
    }
    assert_eq!(app.fiscal_events().len(), before);
}

#[tokio::test]
async fn a_sale_is_signed_by_the_cashier_who_takes_the_payment() {
    let app = app().await;
    as_(&app, APPRENTICE, UserRole::Waiter);
    let open = table(&app, APPRENTICE, 3);
    do_sign_table_order(&app.state, open).await.unwrap();

    as_(&app, SERVER, UserRole::Cashier);
    let r = do_checkout(&app.state, open, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    let events = app.fiscal_events();
    let p = events.iter().find(|e| e.event_type == "P").unwrap();
    let n = events.iter().find(|e| e.event_type == "N").unwrap();
    assert_eq!(
        p.user_insz, APPRENTICE,
        "the order was registered by the apprentice"
    );
    assert_eq!(n.user_insz, SERVER, "the sale is the server's");
    let request: serde_json::Value =
        serde_json::from_str(n.event_data_json.as_deref().unwrap()).unwrap();
    assert_eq!(request["employeeId"], SERVER, "{request}");
    assert_eq!(r.ticket.unwrap().operator, format!("Test {SERVER}"));
}

// ── Sessions ───────────────────────────────────────────────────

#[tokio::test]
async fn login_opens_the_session_and_logout_ends_it() {
    let app = app().await;
    app.logout();
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(&format!(
            "INSERT INTO users (insz, name, role, active) VALUES
                ('{SERVER}', 'Serveur', 'cashier', 1),
                ('{APPRENTICE}', 'Parti', 'waiter', 0);"
        ))
        .unwrap();
    }
    assert!(
        do_login(&app.state, APPRENTICE.into(), None).is_err(),
        "inactive user"
    );
    assert_eq!(
        do_work_in(&app.state, APPRENTICE.into()).await.unwrap_err(),
        ERR_NOT_LOGGED_IN
    );

    let user = do_login(&app.state, SERVER.into(), None).unwrap();
    assert_eq!(user.role, UserRole::Cashier);
    do_work_in(&app.state, SERVER.into()).await.unwrap();

    do_logout(&app.state).unwrap();
    assert_eq!(
        do_work_out(&app.state, SERVER.into()).await.unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
}

#[tokio::test]
async fn a_failed_login_keeps_the_previous_session_closed() {
    let app = app().await;
    app.logout();
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO users (insz, name, role, active) VALUES (?1, 'Gérant', 'admin', 1)",
            [MANAGER],
        )
        .unwrap();
    }
    // Give the manager a PIN (as themselves), then log out.
    as_(&app, MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), "4321".into(), None).unwrap();
    do_logout(&app.state).unwrap();

    for pin in [None, Some("0000".to_string())] {
        assert!(do_login(&app.state, MANAGER.into(), pin).is_err());
        assert_eq!(
            do_backup_database(&app.state).unwrap_err(),
            ERR_NOT_LOGGED_IN
        );
    }
    do_login(&app.state, MANAGER.into(), Some("4321".into())).unwrap();
    assert_ne!(
        do_backup_database(&app.state).err().as_deref(),
        Some(ERR_NOT_LOGGED_IN)
    );
}

// ── Users and PINs ─────────────────────────────────────────────

#[tokio::test]
async fn only_a_manager_creates_users() {
    let app = app().await;
    let input = |insz: &str, name: &str, role: &str| CreateUserInput {
        insz: insz.into(),
        name: name.into(),
        role: role.into(),
        ..Default::default()
    };
    for role in [UserRole::Waiter, UserRole::Cashier] {
        as_(&app, SERVER, role);
        assert_eq!(
            do_create_user(&app.state, input(APPRENTICE, "Nouveau", "admin")).unwrap_err(),
            ERR_FORBIDDEN,
            "{role:?}"
        );
    }
    as_(&app, MANAGER, UserRole::Admin);
    assert!(
        do_create_user(&app.state, input(APPRENTICE, "  ", "waiter")).is_err(),
        "name required"
    );
    let user = do_create_user(&app.state, input(APPRENTICE, "Nouveau", "waiter")).unwrap();
    assert_eq!(user.role, UserRole::Waiter);
}

#[tokio::test]
async fn a_pin_is_changed_by_its_owner_or_a_manager() {
    let app = app().await;
    as_(&app, APPRENTICE, UserRole::Waiter);
    as_(&app, SERVER, UserRole::Cashier);
    // The server sets their own PIN, not the apprentice's.
    do_set_pin(&app.state, SERVER.into(), "1234".into(), None).unwrap();
    assert_eq!(
        do_set_pin(&app.state, APPRENTICE.into(), "1234".into(), None).unwrap_err(),
        ERR_FORBIDDEN
    );
    // A manager resets anyone's, confirming with their own PIN.
    as_(&app, MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), "4321".into(), None).unwrap();
    assert_eq!(
        do_set_pin(&app.state, APPRENTICE.into(), "5678".into(), None).unwrap_err(),
        "pin_required",
        "a manager session left open does not reset another PIN on its own"
    );
    assert_eq!(
        do_set_pin(
            &app.state,
            APPRENTICE.into(),
            "5678".into(),
            Some("0000".into())
        )
        .unwrap_err(),
        "pin_wrong"
    );
    do_set_pin(
        &app.state,
        APPRENTICE.into(),
        "5678".into(),
        Some("4321".into()),
    )
    .unwrap();
    app.logout();
    assert!(do_login(&app.state, APPRENTICE.into(), Some("5678".into())).is_ok());
    assert_eq!(
        do_set_pin(&app.state, MANAGER.into(), "0000".into(), None).unwrap_err(),
        ERR_FORBIDDEN,
        "the apprentice, now logged in, cannot change the manager's PIN"
    );
}

// ── Training mode ──────────────────────────────────────────────

#[tokio::test]
async fn training_mode_never_switches_under_an_open_order() {
    let app = app().await;
    as_(&app, MANAGER, UserRole::Admin);
    let open = table(&app, MANAGER, 1);
    assert_eq!(
        do_set_training_mode(&app.state, true).unwrap_err(),
        ERR_OPEN_ORDERS
    );
    assert!(!do_get_settings(&app.state).unwrap().training_mode);
    // Asking for the current mode is not a switch.
    assert!(!do_set_training_mode(&app.state, false).unwrap());

    // Once paid, the switch is allowed - and back again only when the
    // training order is paid too.
    let r = do_checkout(&app.state, open, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    // An empty order (a table tapped, nothing ordered) does not block.
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(2),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap();
    assert!(do_set_training_mode(&app.state, true).unwrap());
    let training = table(&app, MANAGER, 3);
    assert_eq!(
        do_set_training_mode(&app.state, false).unwrap_err(),
        ERR_OPEN_ORDERS
    );
    let r = do_checkout(&app.state, training, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert!(!do_set_training_mode(&app.state, false).unwrap());

    let labels: Vec<_> = app
        .fiscal_events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();
    assert_eq!(
        labels,
        ["N", "T"],
        "the real sale stayed real, the training one T"
    );
}

// ── The examiners' credentials ─────────────────────────────────

/// The logins REPONSES_MANUEL_CERT_FR.md gives the SPF agents, as
/// seeded on the admission image: each one logs in with its PIN and
/// gets its role.
#[tokio::test]
async fn the_admission_seed_logs_in_every_documented_profile() {
    let app = TestApp::new().await;
    {
        let conn = app.state.db.lock().unwrap();
        pos_engine::install::apply_seed(
            &conn,
            include_str!("../../../src-tauri/seed_spf_test.sql"),
        )
        .expect("seed");
    }
    app.logout();
    for (insz, pin, role) in [
        ("00000000097", Some("0097"), UserRole::Admin),
        ("94060242576", Some("1111"), UserRole::Admin),
        ("84022899837", Some("2222"), UserRole::Cashier),
        ("99012345667", Some("3333"), UserRole::Waiter),
        // The SPF test setup's staff: John, Elisa (above) and Sammy.
        ("75061189702", Some("4444"), UserRole::Cashier),
        ("80113078968", Some("5555"), UserRole::Cashier),
    ] {
        assert!(
            pos_core::user::validate_insz(insz),
            "{insz} is a valid INSZ"
        );
        if pin.is_some() {
            assert_eq!(
                do_login(&app.state, insz.into(), Some("9999".into())).unwrap_err(),
                "pin_wrong",
                "{insz}"
            );
        }
        let user = do_login(&app.state, insz.into(), pin.map(String::from))
            .unwrap_or_else(|e| panic!("{insz}: {e}"));
        assert_eq!(user.role, role, "{insz}");
        assert_eq!(user.has_pin, pin.is_some(), "{insz}");
        do_logout(&app.state).unwrap();
    }
    // The robot of the seed is the system identity, not a login.
    assert_eq!(
        do_login(&app.state, "00000000029".into(), None).unwrap_err(),
        ERR_SYSTEM_USER
    );
    let listed = pos_engine::commands::auth::do_list_users(&app.state).unwrap();
    assert!(listed.iter().all(|u| u.insz != "00000000029"));
    for insz in [
        "00000000097",
        "94060242576",
        "84022899837",
        "99012345667",
        "75061189702",
        "80113078968",
    ] {
        assert!(
            listed.iter().any(|u| u.insz == insz),
            "{insz} has a login tile"
        );
    }
}

// ── The system identity and the first manager PIN ──────────────

const ROBOT: &str = "00000000029";

fn insert_user(app: &TestApp, insz: &str, role: &str) {
    let conn = app.state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO users (insz, name, role, active) VALUES (?1, 'Nouveau', ?2, 1)",
        [insz, role],
    )
    .unwrap();
}

#[tokio::test]
async fn the_system_identity_never_logs_in_nor_gets_a_pin() {
    let app = app().await;
    insert_user(&app, ROBOT, "admin");
    app.logout();
    for pin in [None, Some("1234".to_string())] {
        assert_eq!(
            do_login(&app.state, ROBOT.into(), pin).unwrap_err(),
            ERR_SYSTEM_USER
        );
    }
    assert_eq!(
        do_login_with_new_pin(&app.state, ROBOT.into(), "1234".into()).unwrap_err(),
        ERR_SYSTEM_USER
    );
    assert_eq!(
        do_backup_database(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN,
        "no session was opened"
    );
    as_(&app, MANAGER, UserRole::Admin);
    assert_eq!(
        do_set_pin(&app.state, ROBOT.into(), "1234".into(), None).unwrap_err(),
        ERR_SYSTEM_USER
    );
    assert_eq!(
        do_create_user(
            &app.state,
            CreateUserInput {
                insz: ROBOT.into(),
                name: "Robot".into(),
                role: "admin".into(),
                ..Default::default()
            },
        )
        .unwrap_err(),
        ERR_SYSTEM_USER
    );
}

#[tokio::test]
async fn a_manager_without_pin_must_choose_one_to_log_in() {
    let app = app().await;
    insert_user(&app, MANAGER, "admin");
    app.logout();

    assert_eq!(
        do_login(&app.state, MANAGER.into(), None).unwrap_err(),
        ERR_PIN_SETUP_REQUIRED
    );
    assert_eq!(
        do_backup_database(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );

    for bad in ["", "123", "12345", "12a4"] {
        assert_eq!(
            do_login_with_new_pin(&app.state, MANAGER.into(), bad.into()).unwrap_err(),
            ERR_PIN_FORMAT,
            "{bad:?}"
        );
    }
    let user = do_login_with_new_pin(&app.state, MANAGER.into(), "4321".into()).unwrap();
    assert!(user.has_pin);
    assert_eq!(user.role, UserRole::Admin);
    assert_ne!(
        do_backup_database(&app.state).err().as_deref(),
        Some(ERR_NOT_LOGGED_IN),
        "the session is open"
    );

    // Once chosen, the PIN is required and cannot be chosen again.
    do_logout(&app.state).unwrap();
    assert_eq!(
        do_login_with_new_pin(&app.state, MANAGER.into(), "0000".into()).unwrap_err(),
        ERR_PIN_ALREADY_SET
    );
    assert_eq!(
        do_login(&app.state, MANAGER.into(), None).unwrap_err(),
        "pin_required"
    );
    assert!(do_login(&app.state, MANAGER.into(), Some("4321".into())).is_ok());
}

/// Once a manager has a PIN, another manager tile without one is not
/// claimable at the login screen.
#[tokio::test]
async fn a_second_manager_tile_is_not_claimed_at_the_login_screen() {
    let app = app().await;
    insert_user(&app, MANAGER, "admin");
    insert_user(&app, SERVER, "admin");
    app.logout();
    do_login_with_new_pin(&app.state, MANAGER.into(), "4321".into()).unwrap();
    do_logout(&app.state).unwrap();

    assert_eq!(
        do_login(&app.state, SERVER.into(), None).unwrap_err(),
        errors::FIRST_PIN_NEEDS_MANAGER
    );
    assert_eq!(
        do_login_with_new_pin(&app.state, SERVER.into(), "9999".into()).unwrap_err(),
        errors::FIRST_PIN_NEEDS_MANAGER
    );
    assert!(
        app.state.session.lock().unwrap().is_none(),
        "no session opened"
    );

    // The manager sets it, confirming with their own PIN.
    do_login(&app.state, MANAGER.into(), Some("4321".into())).unwrap();
    do_set_pin(
        &app.state,
        SERVER.into(),
        "2468".into(),
        Some("4321".into()),
    )
    .unwrap();
    do_logout(&app.state).unwrap();
    assert!(do_login(&app.state, SERVER.into(), Some("2468".into())).is_ok());
}

#[tokio::test]
async fn a_new_manager_gets_a_pin_confirmed_by_the_creating_manager() {
    let app = app().await;
    insert_user(&app, MANAGER, "admin");
    app.logout();
    do_login_with_new_pin(&app.state, MANAGER.into(), "4321".into()).unwrap();
    let new_manager = |initial: Option<&str>, operator: Option<&str>| CreateUserInput {
        insz: SERVER.into(),
        name: "Gerant 2".into(),
        role: "admin".into(),
        initial_pin: initial.map(Into::into),
        operator_pin: operator.map(Into::into),
    };

    assert_eq!(
        do_create_user(&app.state, new_manager(None, Some("4321"))).unwrap_err(),
        ERR_PIN_FORMAT
    );
    assert_eq!(
        do_create_user(&app.state, new_manager(Some("2468"), None)).unwrap_err(),
        "pin_required"
    );
    assert_eq!(
        do_create_user(&app.state, new_manager(Some("2468"), Some("0000"))).unwrap_err(),
        "pin_wrong"
    );
    let user = do_create_user(&app.state, new_manager(Some("2468"), Some("4321"))).unwrap();
    assert!(user.has_pin);
    do_logout(&app.state).unwrap();
    assert!(do_login(&app.state, SERVER.into(), Some("2468".into())).is_ok());
}

#[tokio::test]
async fn a_first_pin_is_refused_for_an_unknown_or_departed_user() {
    let app = app().await;
    app.logout();
    assert_eq!(
        do_login_with_new_pin(&app.state, APPRENTICE.into(), "1234".into()).unwrap_err(),
        errors::USER_UNKNOWN
    );
    insert_user(&app, APPRENTICE, "waiter");
    as_(&app, MANAGER, UserRole::Admin);
    do_deactivate_user(&app.state, APPRENTICE.into()).unwrap();
    app.logout();
    assert_eq!(
        do_login_with_new_pin(&app.state, APPRENTICE.into(), "1234".into()).unwrap_err(),
        errors::USER_UNKNOWN
    );
    assert_eq!(
        do_login_with_new_pin(&app.state, "123".into(), "1234".into()).unwrap_err(),
        errors::INSZ_INVALID
    );
}

#[tokio::test]
async fn a_server_or_apprentice_without_pin_still_logs_in() {
    let app = app().await;
    insert_user(&app, SERVER, "cashier");
    insert_user(&app, APPRENTICE, "waiter");
    app.logout();
    for insz in [SERVER, APPRENTICE] {
        let user = do_login(&app.state, insz.into(), None).unwrap();
        assert!(!user.has_pin, "{insz}");
        do_logout(&app.state).unwrap();
    }
}

#[tokio::test]
async fn a_manager_deactivates_a_departed_employee_but_never_themselves() {
    let app = app().await;
    as_(&app, SERVER, UserRole::Cashier);
    assert_eq!(
        do_deactivate_user(&app.state, APPRENTICE.into()).unwrap_err(),
        ERR_FORBIDDEN
    );
    as_(&app, APPRENTICE, UserRole::Waiter);
    as_(&app, MANAGER, UserRole::Admin);
    assert_eq!(
        do_deactivate_user(&app.state, MANAGER.into()).unwrap_err(),
        ERR_DEACTIVATE_SELF
    );
    do_deactivate_user(&app.state, APPRENTICE.into()).unwrap();
    assert!(
        do_deactivate_user(&app.state, APPRENTICE.into()).is_err(),
        "already gone"
    );
    app.logout();
    assert!(do_login(&app.state, APPRENTICE.into(), None).is_err());
    // Re-hired: the same INSZ comes back, with its new role.
    as_(&app, MANAGER, UserRole::Admin);
    do_create_user(
        &app.state,
        CreateUserInput {
            insz: APPRENTICE.into(),
            name: "De retour".into(),
            role: "cashier".into(),
            ..Default::default()
        },
    )
    .unwrap();
    app.logout();
    assert_eq!(
        do_login(&app.state, APPRENTICE.into(), None).unwrap().role,
        UserRole::Cashier
    );
}

#[tokio::test]
async fn wrong_pins_lock_the_user_out_even_for_the_right_pin() {
    let app = app().await;
    insert_user(&app, MANAGER, "admin");
    insert_user(&app, SERVER, "cashier");
    app.logout();
    do_login_with_new_pin(&app.state, MANAGER.into(), MANAGER_PIN.into()).unwrap();
    do_set_pin(
        &app.state,
        SERVER.into(),
        "0040".into(),
        Some(MANAGER_PIN.into()),
    )
    .unwrap();
    do_logout(&app.state).unwrap();

    let login = |pin: &str| do_login(&app.state, SERVER.into(), Some(pin.into()));
    for n in 0..4 {
        assert_eq!(login(&format!("{n:04}")).unwrap_err(), errors::PIN_WRONG);
    }
    assert_eq!(login("0004").unwrap_err(), "pin_locked|seconds=30");
    assert!(login("0040").unwrap_err().starts_with("pin_locked|"));
    assert!(app.state.session.lock().unwrap().is_none());

    // Per user: the manager still logs in, and their PIN confirmations
    // count on their own attempts.
    do_login(&app.state, MANAGER.into(), Some(MANAGER_PIN.into())).unwrap();
    for _ in 0..5 {
        do_set_pin(
            &app.state,
            SERVER.into(),
            "1111".into(),
            Some("0000".into()),
        )
        .unwrap_err();
    }
    assert!(
        do_set_pin(
            &app.state,
            SERVER.into(),
            "1111".into(),
            Some(MANAGER_PIN.into())
        )
        .unwrap_err()
        .starts_with("pin_locked|")
    );
}

#[tokio::test]
async fn only_a_manager_touches_the_owner_alerts() {
    use pos_engine::commands::alerts::{
        AlertKinds, OwnerAlertsInput, do_get_owner_alerts, do_list_owner_alert_log,
        do_send_test_alert, do_set_owner_alerts,
    };
    let app = app().await;
    let input = || OwnerAlertsInput {
        enabled: false,
        chat_id: "123".into(),
        label: String::new(),
        language: "fr".into(),
        kinds: AlertKinds::default(),
        z_deadline_hour: 6,
        online_wait_minutes: 5,
        reminder_minutes: 60,
    };
    app.logout();
    assert_eq!(
        do_get_owner_alerts(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    for (insz, role) in [(SERVER, UserRole::Cashier), (APPRENTICE, UserRole::Waiter)] {
        as_(&app, insz, role);
        assert_eq!(do_get_owner_alerts(&app.state).unwrap_err(), ERR_FORBIDDEN);
        assert_eq!(
            do_set_owner_alerts(&app.state, input()).unwrap_err(),
            ERR_FORBIDDEN
        );
        assert_eq!(
            do_send_test_alert(&app.state).await.unwrap_err(),
            ERR_FORBIDDEN
        );
        assert_eq!(
            do_list_owner_alert_log(&app.state, 5).unwrap_err(),
            ERR_FORBIDDEN
        );
    }
    as_(&app, MANAGER, UserRole::Admin);
    assert!(!do_get_owner_alerts(&app.state).unwrap().enabled);
    assert!(do_set_owner_alerts(&app.state, input()).is_ok());
    assert!(do_list_owner_alert_log(&app.state, 5).unwrap().is_empty());
}

#[tokio::test]
async fn any_operator_reads_the_allergens_and_only_a_manager_edits_them() {
    use pos_engine::commands::allergens::{
        SetAllergensInput, do_get_allergen_sheet, do_list_allergens, do_print_allergen_sheet,
        do_set_allergens,
    };
    let app = app().await;
    let edit = || SetAllergensInput {
        entity_type: "menu_item".into(),
        entity_id: 1,
        declared: true,
        contains: vec!["milk".into()],
        may_contain: vec![],
    };
    app.logout();
    assert_eq!(
        do_list_allergens(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_get_allergen_sheet(&app.state, None, None).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_print_allergen_sheet(&app.state, None, None)
            .await
            .unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_set_allergens(&app.state, edit()).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    for (insz, role) in [(SERVER, UserRole::Cashier), (APPRENTICE, UserRole::Waiter)] {
        as_(&app, insz, role);
        assert_eq!(do_list_allergens(&app.state).unwrap().len(), 14);
        assert!(do_get_allergen_sheet(&app.state, None, None).is_ok());
        assert_eq!(
            do_set_allergens(&app.state, edit()).unwrap_err(),
            ERR_FORBIDDEN
        );
    }
    as_(&app, MANAGER, UserRole::Admin);
    assert_eq!(do_set_allergens(&app.state, edit()), Ok(true));
}

#[tokio::test]
async fn any_operator_works_the_courses_and_only_a_manager_sets_them() {
    use pos_engine::commands::courses::{
        CourseSettings, do_get_course_settings, do_list_held_courses, do_reprint_course_slip,
        do_send_next_course, do_set_course_settings, do_set_line_course,
    };
    let app = app().await;
    as_(&app, MANAGER, UserRole::Admin);
    let order = table(&app, MANAGER, 12);
    app.logout();
    assert_eq!(
        do_get_course_settings(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_set_course_settings(&app.state, CourseSettings::default()).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_set_line_course(&app.state, 1, 1).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_list_held_courses(&app.state, order).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_send_next_course(&app.state, order).await.unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_reprint_course_slip(&app.state, order, 1)
            .await
            .unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    for (insz, role) in [(SERVER, UserRole::Cashier), (APPRENTICE, UserRole::Waiter)] {
        as_(&app, insz, role);
        assert!(do_get_course_settings(&app.state).is_ok());
        assert_eq!(
            do_set_course_settings(&app.state, CourseSettings::default()).unwrap_err(),
            ERR_FORBIDDEN
        );
        // Allowed to try: the refusals below come from the rules, not the role.
        assert_eq!(
            do_set_line_course(&app.state, 1, 1).unwrap_err(),
            errors::COURSES_DISABLED
        );
        assert!(do_list_held_courses(&app.state, order).unwrap().is_empty());
        assert_eq!(
            do_send_next_course(&app.state, order).await.unwrap_err(),
            errors::COURSES_DISABLED
        );
        assert_eq!(
            do_reprint_course_slip(&app.state, order, 1)
                .await
                .unwrap_err(),
            errors::NOTHING_PRINTED
        );
    }
    as_(&app, MANAGER, UserRole::Admin);
    assert!(do_set_course_settings(&app.state, CourseSettings::default()).is_ok());
}
