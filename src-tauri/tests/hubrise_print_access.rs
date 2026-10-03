//! Webshop slips are reprinted by logged-in staff only.

#![cfg(feature = "hubrise")]

mod common;

use common::hubrise::connected_app;
use pili_pos::commands::errors;
use pili_pos::commands::print::{do_print_webshop_customer, do_print_webshop_kitchen};

#[tokio::test]
async fn webshop_slips_need_a_logged_in_staff_member() {
    let (app, _db, _mock) = connected_app().await;
    app.logout();

    assert_eq!(
        do_print_webshop_kitchen(&app.state, 1).await.unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    assert_eq!(
        do_print_webshop_customer(&app.state, 1).await.unwrap_err(),
        errors::NOT_LOGGED_IN
    );
}
