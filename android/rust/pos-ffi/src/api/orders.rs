//! Facade of the menu, tables and orders commands (engine modules order, modifiers, menu, tables
//! and session): one async method per Tauri command, each one converting its arguments, making the
//! wrapper's single engine call and converting the answer. Plus the `vat_split` helper (PD62).

use pos_engine::commands::{
    discount, menu, modifiers, order, session, sold_out, table_plan, table_transfer, tables,
};

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::common::{SlipOutcome, VatCode};
use crate::records::orders::{
    AddOrderLineInput, AddWithModifiersInput, CartLine, Category, CreateOrderInput, DiscountInput,
    DiscountResult, LeaveReason, LockOutcome, MenuItem, ModifierGroup, Order, OrderLine,
    SoldOutItem, TableInfo, TablePlan, TableTransferResult, TakeawayInfo, VatLine, VatSplitRow,
};

#[uniffi::export]
impl PosEngine {
    /// Start an order (or get the open order of a busy table).
    pub async fn create_order(&self, input: CreateOrderInput) -> Result<Order, PosError> {
        let input: order::CreateOrderInput = convert::inp(&input)?;
        let engine_order = self
            .blocking(move |st| order::do_create_order(st, input))
            .await?;
        convert::out(&engine_order)
    }

    /// Add a product to an order.
    pub async fn add_order_line(&self, input: AddOrderLineInput) -> Result<OrderLine, PosError> {
        let input: order::AddOrderLineInput = convert::inp(&input)?;
        let line = self
            .blocking(move |st| order::do_add_order_line(st, input))
            .await?;
        convert::out(&line)
    }

    /// Remove a line from its order.
    pub async fn remove_order_line(&self, line_id: i64) -> Result<(), PosError> {
        self.blocking(move |st| order::do_remove_order_line(st, line_id))
            .await
    }

    /// Change the quantity of a line.
    pub async fn update_order_line_quantity(
        &self,
        line_id: i64,
        quantity: i32,
    ) -> Result<(), PosError> {
        self.blocking(move |st| order::do_update_order_line_quantity(st, line_id, quantity))
            .await
    }

    /// Set (or clear) the kitchen note of a line; a note changed after the line went to the
    /// kitchen prints a note slip.
    pub async fn set_line_note(
        &self,
        line_id: i64,
        note: Option<String>,
    ) -> Result<SlipOutcome, PosError> {
        let outcome = self
            .spawn(move |e| async move { order::do_set_line_note(&e, line_id, note).await })
            .await?;
        convert::out(&outcome)
    }

    /// An order by id.
    pub async fn get_order(&self, order_id: i64) -> Result<Order, PosError> {
        let engine_order = self
            .blocking(move |st| order::do_get_order(st, order_id))
            .await?;
        convert::out(&engine_order)
    }

    /// Every line of an order, ordered by id.
    pub async fn list_order_lines(&self, order_id: i64) -> Result<Vec<OrderLine>, PosError> {
        let lines = self
            .blocking(move |st| order::do_list_order_lines(st, order_id))
            .await?;
        convert::out(&lines)
    }

    /// The order's lines as the cart shows them.
    pub async fn list_cart_lines(&self, order_id: i64) -> Result<Vec<CartLine>, PosError> {
        let lines = self
            .blocking(move |st| order::do_list_cart_lines(st, order_id))
            .await?;
        convert::out(&lines)
    }

    /// Every open order.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn list_open_orders(&self) -> Result<Vec<Order>, PosError> {
        let orders = self.blocking(|st| order::do_list_open_orders(st)).await?;
        convert::out(&orders)
    }

    /// The option groups of a product, names in `lang` (French when `None`).
    pub async fn get_item_modifiers(
        &self,
        menu_item_id: i64,
        lang: Option<String>,
    ) -> Result<Vec<ModifierGroup>, PosError> {
        let groups = self
            .blocking(move |st| {
                modifiers::do_get_item_modifiers_localized(st, menu_item_id, lang.as_deref())
            })
            .await?;
        convert::out(&groups)
    }

    /// Add a product with the options picked.
    pub async fn add_order_line_with_modifiers(
        &self,
        input: AddWithModifiersInput,
    ) -> Result<OrderLine, PosError> {
        let input: modifiers::AddWithModifiersInput = convert::inp(&input)?;
        let line = self
            .blocking(move |st| modifiers::do_add_order_line_with_modifiers(st, &input))
            .await?;
        convert::out(&line)
    }

    /// The menu categories, names in `lang`.
    pub async fn get_categories(&self, lang: Option<String>) -> Result<Vec<Category>, PosError> {
        let categories = self
            .blocking(move |st| menu::do_get_categories(st, lang))
            .await?;
        convert::out(&categories)
    }

    /// The menu items of a category (all when `None`), names in `lang`.
    pub async fn get_menu_items(
        &self,
        category_id: Option<i64>,
        lang: Option<String>,
    ) -> Result<Vec<MenuItem>, PosError> {
        let items = self
            .blocking(move |st| menu::do_get_menu_items(st, category_id, lang))
            .await?;
        convert::out(&items)
    }

    /// The products sold out now ("Épuisé"), names in `lang`.
    pub async fn list_sold_out(&self, lang: Option<String>) -> Result<Vec<SoldOutItem>, PosError> {
        let items = self
            .blocking(move |st| sold_out::do_list_sold_out(st, lang))
            .await?;
        convert::out(&items)
    }

    /// Mark a product sold out until the end of the business day, or until re-enabled.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "UniFFI export: the argument is part of the Kotlin API"
    )]
    pub async fn set_sold_out(
        &self,
        menu_item_id: i64,
        until_end_of_day: bool,
        lang: Option<String>,
    ) -> Result<Vec<SoldOutItem>, PosError> {
        let items = self
            .blocking(move |st| sold_out::do_set_sold_out(st, menu_item_id, until_end_of_day, lang))
            .await?;
        convert::out(&items)
    }

    /// Make a sold-out product available again.
    pub async fn clear_sold_out(
        &self,
        menu_item_id: i64,
        lang: Option<String>,
    ) -> Result<Vec<SoldOutItem>, PosError> {
        let items = self
            .blocking(move |st| sold_out::do_clear_sold_out(st, menu_item_id, lang))
            .await?;
        convert::out(&items)
    }

    /// The configured tables with their current order status.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn list_tables(&self) -> Result<Vec<TableInfo>, PosError> {
        let infos = self
            .blocking(|st| tables::do_list_configured_tables(st))
            .await?;
        convert::out(&infos)
    }

    /// The table plan (the table numbers in use) and its changes.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn get_table_plan(&self) -> Result<TablePlan, PosError> {
        let plan = self
            .blocking(|st| table_plan::do_get_table_plan(st))
            .await?;
        convert::out(&plan)
    }

    /// Set the table plan ("1-12, 20-25"); managers only.
    pub async fn set_table_plan(&self, plan: String) -> Result<TablePlan, PosError> {
        let value = self
            .blocking(move |st| table_plan::do_set_table_plan(st, &plan))
            .await?;
        convert::out(&value)
    }

    /// The takeaways left open.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn list_open_takeaways(&self) -> Result<Vec<TakeawayInfo>, PosError> {
        let infos = self
            .blocking(|st| tables::do_list_open_takeaways(st))
            .await?;
        convert::out(&infos)
    }

    /// Register what was ordered at a table since the last registration (event P).
    pub async fn sign_table_order(&self, order_id: i64) -> Result<SlipOutcome, PosError> {
        let outcome = self
            .spawn(move |e| async move { tables::do_sign_table_order(&e, order_id).await })
            .await?;
        convert::out(&outcome)
    }

    /// Send a signPreBill (provisional bill, event P).
    pub async fn sign_pre_bill(&self, order_id: i64) -> Result<SlipOutcome, PosError> {
        let outcome = self
            .spawn(move |e| async move { tables::do_sign_pre_bill(&e, order_id).await })
            .await?;
        convert::out(&outcome)
    }

    /// Move the open order of a table to a free table (event P, cost center change).
    pub async fn transfer_table(
        &self,
        order_id: i64,
        to_table: i32,
    ) -> Result<TableTransferResult, PosError> {
        let result = self
            .spawn(move |e| async move {
                table_transfer::do_transfer_table(&e, order_id, to_table).await
            })
            .await?;
        convert::out(&result)
    }

    /// Give a discount on an open till order (order discount or one free line); a manager
    /// confirms with the PIN.
    pub async fn apply_discount(
        &self,
        order_id: i64,
        input: DiscountInput,
        insz: String,
        pin: String,
    ) -> Result<DiscountResult, PosError> {
        let input: discount::DiscountInput = convert::inp(&input)?;
        let result = self
            .spawn(move |e| async move {
                discount::do_apply_discount(&e, order_id, input, insz, pin).await
            })
            .await?;
        convert::out(&result)
    }

    /// Remove the order discount (`line_id` `None`) or the free line `line_id`; a manager confirms
    /// with the PIN.
    pub async fn remove_discount(
        &self,
        order_id: i64,
        line_id: Option<i64>,
        insz: String,
        pin: String,
    ) -> Result<DiscountResult, PosError> {
        let result = self
            .spawn(move |e| async move {
                discount::do_remove_discount(&e, order_id, line_id, insz, pin).await
            })
            .await?;
        convert::out(&result)
    }

    /// Tell the engine which order the screen shows (`None`: none).
    pub async fn set_active_order(&self, order_id: Option<i64>) -> Result<(), PosError> {
        self.blocking(move |st| session::do_set_active_order(st, order_id))
            .await
    }

    /// The operator leaves an order: register what was ordered unless it is a takeaway or no
    /// longer open.
    pub async fn leave_order(
        &self,
        order_id: i64,
        reason: LeaveReason,
    ) -> Result<SlipOutcome, PosError> {
        let reason: session::LeaveReason = convert::inp(&reason)?;
        let outcome = self
            .spawn(move |e| async move { session::do_leave_order(&e, order_id, reason).await })
            .await?;
        convert::out(&outcome)
    }

    /// Lock the screen: register the active order, then close the session.
    pub async fn lock_session(&self) -> Result<LockOutcome, PosError> {
        let outcome = self
            .spawn(move |e| async move { session::do_lock_session(&e).await })
            .await?;
        convert::out(&outcome)
    }

    /// The operator touched the screen: restart the idle lock time.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn user_activity(&self) -> Result<(), PosError> {
        self.blocking(|st| session::do_user_activity(st)).await
    }
}

/// `Math.round(n / d)` of JavaScript for an exact fraction with `d > 0`: ties go up
/// (towards +infinity), also for negative values. `floor(x + 1/2) = floor((2n + d) / 2d)`.
fn js_round_div(n: i128, d: i128) -> i128 {
    (2 * n + d).div_euclid(2 * d)
}

/// The VAT included in a total, per code, as `ui/src/components/VatSummary.vue` computes it
/// (PD62): total per code is the sum of `unit_price_cents * quantity`; rows come in the order A, B,
/// C, D, only for codes present in `lines` (X is never shown); the rates are 21, 12, 6 and 0; the
/// VAT of a code with a rate is `total - round(total * 10000 / (10000 + rate * 100))` with the
/// rounding of JavaScript (half up, also for negative totals). Display only: it can differ by one
/// cent from the truncating inclusive VAT of the engine; the ticket VAT comes from the FDM.
#[uniffi::export]
#[expect(
    clippy::unused_async,
    reason = "UniFFI exports it as a Kotlin suspend fun: the signature is the contract"
)]
pub async fn vat_split(lines: Vec<VatLine>) -> Vec<VatSplitRow> {
    // Totals of A, B, C and D; `None` when no line has the code. X is never shown.
    let mut totals: [Option<i128>; 4] = [None; 4];
    for line in &lines {
        let slot = match line.vat_code {
            VatCode::A => 0,
            VatCode::B => 1,
            VatCode::C => 2,
            VatCode::D => 3,
            VatCode::X => continue,
        };
        let total = i128::from(line.unit_price_cents) * i128::from(line.quantity);
        *totals[slot].get_or_insert(0) += total;
    }
    let codes = [
        (VatCode::A, 21),
        (VatCode::B, 12),
        (VatCode::C, 6),
        (VatCode::D, 0),
    ];
    codes
        .into_iter()
        .zip(totals)
        .filter_map(|((vat_code, rate_percent), total)| {
            let total = total?;
            let vat = if rate_percent > 0 {
                let d = 10_000 + i128::from(rate_percent) * 100;
                total - js_round_div(total * 10_000, d)
            } else {
                0
            };
            Some(VatSplitRow {
                vat_code,
                rate_percent,
                total_incl_cents: clamp_i64(total),
                vat_cents: clamp_i64(vat),
            })
        })
        .collect()
}

/// An amount back to `i64`; sums of realistic order lines never come near the bounds.
fn clamp_i64(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_round_ties_go_up() {
        assert_eq!(js_round_div(25, 2), 13);
        assert_eq!(js_round_div(-25, 2), -12);
        assert_eq!(js_round_div(24, 2), 12);
        assert_eq!(js_round_div(-24, 2), -12);
        assert_eq!(js_round_div(-1, 3), 0);
        assert_eq!(js_round_div(-2, 3), -1);
        assert_eq!(js_round_div(1, 3), 0);
        assert_eq!(js_round_div(2, 3), 1);
    }
}

/// The logged-in dine-in flow through the facade. No FFI login exists before the auth card, so
/// this test opens the session through the engine's own state (`blocking` hands it out); the
/// integration tests of `tests/ffi/orders.rs` cover the reads and the refusals without a session.
#[cfg(test)]
mod flow {
    use std::sync::Arc;

    use super::*;
    use crate::core::EngineOptions;
    use crate::records::orders::{DiscountKind, DiscountReason, DiscountTarget, OrderType};

    const MANAGER_INSZ: &str = "94060242576";

    /// An engine on the seed of the SPF test machine, with its FDM and printer mocks, and the
    /// manager logged in.
    async fn open_logged_in() -> (Arc<PosEngine>, tempfile::TempDir) {
        let fdm = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(fdm, pos_fdm_mock::app()).await.unwrap();
        });
        let printer_mock =
            pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
        let printer = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer_mock)
            .await
            .unwrap()
            .to_string();

        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("bundle");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(
            bundle.join("pos.json"),
            format!(
                r#"{{
                "pos_id": "CPOS0031234567", "terminal_id": "POS-2-ANDROID", "device_id": "bundle-device",
                "restaurant_name": "Restaurant Test", "restaurant_address": "Rue de l'Exemple 1, 4000 Liege",
                "restaurant_vat": "BE0499999960", "restaurant_est_no": "2123456791",
                "fdm_url": "{fdm_url}", "printer_address": "{printer}", "table_count": 25
            }}"#
            ),
        )
        .unwrap();
        let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../src-tauri/seed_spf_test.sql");
        pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed))
            .expect("seed the database");
        let engine = PosEngine::open(EngineOptions {
            data_dir: dir.path().join("data").to_string_lossy().into_owned(),
            config_bundle: bundle.join("pos.json").to_string_lossy().into_owned(),
            device_id: Some("UNIT-ORDERS-0001".into()),
            timers: None,
        })
        .unwrap_or_else(|e| panic!("open: {e}"));
        engine
            .blocking(|st| {
                st.db
                    .lock()
                    .map_err(|e| e.to_string())?
                    .execute_batch(
                        "INSERT INTO modifier_groups (id, min_select, max_select, menu_item_id) VALUES (1, 1, 1, 22001);
                         INSERT INTO modifiers (id, group_id, price_cents, sort_order) VALUES (1, 1, 0, 0), (2, 1, 150, 1);
                         INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                            ('modifier_group', 1, 'name', 'fr', 'Cuisson'),
                            ('modifier', 1, 'name', 'fr', 'Saignant'),
                            ('modifier', 2, 'name', 'fr', 'A point');",
                    )
                    .map_err(|e| e.to_string())?;
                *st.session.lock().map_err(|e| e.to_string())? = Some(pos_core::user::User {
                    insz: MANAGER_INSZ.into(),
                    name: "Gerant Test".into(),
                    role: pos_core::user::UserRole::Admin,
                    active: true,
                    has_pin: true,
                    presence_only: false,
                });
                Ok(())
            })
            .await
            .unwrap();
        (engine, dir)
    }

    fn coded(err: PosError) -> String {
        match err {
            PosError::Coded { code, .. } => code,
            other => panic!("expected a coded error, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_discount_is_applied_and_removed_through_the_facade() {
        const PIN: &str = "4321";
        let (e, _dir) = open_logged_in().await;
        e.blocking(|st| {
            pos_engine::commands::auth::do_set_pin(st, MANAGER_INSZ.into(), PIN.into(), None)
        })
        .await
        .unwrap();
        let order = e
            .create_order(CreateOrderInput {
                order_type: OrderType::Takeaway,
                table_number: None,
                user_insz: MANAGER_INSZ.into(),
            })
            .await
            .unwrap();
        e.add_order_line(AddOrderLineInput {
            order_id: order.id,
            menu_item_id: 10006,
            quantity: 2,
            modifiers_json: None,
            unit_price_cents: None,
        })
        .await
        .unwrap();
        let percent = |value: i64| DiscountInput {
            target: DiscountTarget::Order,
            kind: DiscountKind::Percent,
            value: Some(value),
            reason_code: DiscountReason::CommercialGesture,
            reason_text: None,
        };

        // A wrong PIN is refused with its code and changes nothing.
        let wrong = e
            .apply_discount(order.id, percent(50), MANAGER_INSZ.into(), "0000".into())
            .await
            .unwrap_err();
        assert_eq!(coded(wrong), "pin_wrong");
        assert_eq!(e.get_order(order.id).await.unwrap().total_cents, 2400);

        // 50 % of 2 x 12,00.
        let done = e
            .apply_discount(order.id, percent(50), MANAGER_INSZ.into(), PIN.into())
            .await
            .unwrap();
        assert_eq!(done.order.id, order.id);
        assert_eq!(done.order.total_cents, 1200);
        assert_eq!(done.registration_error, None);
        let cart = e.list_cart_lines(order.id).await.unwrap();
        assert_eq!(cart[0].discount_cents, -1200);
        assert_eq!(cart[0].price_changes.len(), 1);
        assert_eq!(cart[0].price_changes[0].id, "TILL-DISCOUNT");
        assert_eq!(cart[0].price_changes[0].scope, "EVENT");

        // A second discount waits for the removal.
        let twice = e
            .apply_discount(order.id, percent(10), MANAGER_INSZ.into(), PIN.into())
            .await
            .unwrap_err();
        assert_eq!(coded(twice), "discount_already_applied");

        // The removal takes the PIN too, and gives the full price back.
        let wrong = e
            .remove_discount(order.id, None, MANAGER_INSZ.into(), "0000".into())
            .await
            .unwrap_err();
        assert_eq!(coded(wrong), "pin_wrong");
        let removed = e
            .remove_discount(order.id, None, MANAGER_INSZ.into(), PIN.into())
            .await
            .unwrap();
        assert_eq!(removed.order.total_cents, 2400);
        let cart = e.list_cart_lines(order.id).await.unwrap();
        assert_eq!(cart[0].discount_cents, 0);
        // Nothing left to remove.
        let none = e
            .remove_discount(order.id, None, MANAGER_INSZ.into(), PIN.into())
            .await
            .unwrap_err();
        assert_eq!(coded(none), "discount_none");
        e.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_dine_in_flow_reaches_the_engine_at_every_step() {
        let (e, _dir) = open_logged_in().await;
        let input = CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(7),
            user_insz: MANAGER_INSZ.into(),
        };
        let order = e.create_order(input.clone()).await.unwrap();
        assert_eq!(order.table_number, Some(7));
        assert_eq!(order.user_insz, MANAGER_INSZ);
        assert_eq!(order.status, crate::records::orders::OrderStatus::Open,);
        // A busy table answers its own order.
        let again = e.create_order(input).await.unwrap();
        assert_eq!(again.id, order.id);

        let martini = e
            .add_order_line(AddOrderLineInput {
                order_id: order.id,
                menu_item_id: 10006,
                quantity: 2,
                modifiers_json: None,
                unit_price_cents: None,
            })
            .await
            .unwrap();
        assert_eq!(martini.order_id, order.id);
        assert_eq!(martini.quantity, 2);
        assert_eq!(martini.unit_price_cents, 1200);
        assert_eq!(martini.vat_code, VatCode::A);
        // The optional price reaches the engine: a free-price item takes it.
        let dish = e
            .add_order_line(AddOrderLineInput {
                order_id: order.id,
                menu_item_id: 22010,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: Some(1550),
            })
            .await
            .unwrap();
        assert_eq!(dish.unit_price_cents, 1550);
        // A refusal keeps its code.
        let refused = e
            .add_order_line(AddOrderLineInput {
                order_id: order.id,
                menu_item_id: 10006,
                quantity: 0,
                modifiers_json: None,
                unit_price_cents: None,
            })
            .await
            .unwrap_err();
        assert_eq!(coded(refused), "quantity_positive");

        // Options: the picked option is priced and named by the engine.
        let groups = e.get_item_modifiers(22001, None).await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].options.len(), 2);
        let burger = e
            .add_order_line_with_modifiers(AddWithModifiersInput {
                order_id: order.id,
                menu_item_id: 22001,
                quantity: 1,
                modifier_ids: vec![2],
            })
            .await
            .unwrap();
        assert_eq!(burger.unit_price_cents, 2800 + 150);
        assert!(burger.name.ends_with("(A point)"), "{}", burger.name);
        assert!(burger.modifiers_json.is_some());
        let missing = e
            .add_order_line_with_modifiers(AddWithModifiersInput {
                order_id: order.id,
                menu_item_id: 22001,
                quantity: 1,
                modifier_ids: vec![],
            })
            .await
            .unwrap_err();
        assert_eq!(coded(missing), "modifier_count");

        // The cart shows the three lines, all still to send.
        let cart = e.list_cart_lines(order.id).await.unwrap();
        assert_eq!(cart.len(), 3);
        for line in &cart {
            assert_eq!(line.unsent_quantity, i64::from(line.quantity));
        }
        assert_eq!(cart[0].id, martini.id);

        // Change and remove lines.
        e.update_order_line_quantity(martini.id, 5).await.unwrap();
        assert_eq!(
            coded(
                e.update_order_line_quantity(martini.id, 0)
                    .await
                    .unwrap_err()
            ),
            "quantity_positive"
        );
        // A kitchen note is stored and shown in the cart; the engine validates it.
        assert_eq!(
            e.set_line_note(martini.id, Some("  sans glaçon ".into()))
                .await
                .unwrap(),
            SlipOutcome::NothingToPrint
        );
        let cart = e.list_cart_lines(order.id).await.unwrap();
        assert_eq!(cart[0].kitchen_note.as_deref(), Some("sans glaçon"));
        assert_eq!(cart[1].kitchen_note, None);
        assert_eq!(
            coded(
                e.set_line_note(martini.id, Some("x".repeat(61)))
                    .await
                    .unwrap_err()
            ),
            "note_too_long"
        );
        assert_eq!(
            coded(
                e.set_line_note(martini.id, Some("不要".into()))
                    .await
                    .unwrap_err()
            ),
            "note_unprintable"
        );
        e.set_line_note(martini.id, None).await.unwrap();
        let cart = e.list_cart_lines(order.id).await.unwrap();
        assert_eq!(cart[0].kitchen_note, None);
        e.remove_order_line(dish.id).await.unwrap();
        let lines = e.list_order_lines(order.id).await.unwrap();
        assert_eq!(
            lines.iter().map(|l| l.quantity).collect::<Vec<_>>(),
            vec![5, 1]
        );
        let total = e.get_order(order.id).await.unwrap().total_cents;
        assert_eq!(total, 5 * 1200 + 2950);

        // The table and the open orders show it.
        let tables = e.list_tables().await.unwrap();
        assert_eq!(tables[6].order_id, Some(order.id));
        assert_eq!(tables[6].line_count, 2);
        assert!(!tables[6].pre_bill_printed);
        assert!(
            e.list_open_orders()
                .await
                .unwrap()
                .iter()
                .any(|o| o.id == order.id)
        );

        // The screen shows the order; an unknown one is refused.
        e.set_active_order(Some(order.id)).await.unwrap();
        assert!(e.set_active_order(Some(99_999)).await.is_err());
        e.user_activity().await.unwrap();

        // Leaving registers the lines (event P) and the slip goes to the printer mock.
        let slip = e
            .leave_order(order.id, LeaveReason::Navigate)
            .await
            .unwrap();
        assert_eq!(slip, SlipOutcome::Printed);
        let cart = e.list_cart_lines(order.id).await.unwrap();
        assert!(cart.iter().all(|l| l.unsent_quantity == 0), "{cart:?}");
        // Nothing new: nothing to print, by either door.
        assert_eq!(
            e.sign_table_order(order.id).await.unwrap(),
            SlipOutcome::NothingToPrint
        );
        e.sign_pre_bill(order.id).await.unwrap();
        let tables = e.list_tables().await.unwrap();
        assert!(tables[6].pre_bill_printed);

        // Moving the order to a free table: refusals are coded, the move keeps the order.
        assert_eq!(
            coded(e.transfer_table(order.id, 7).await.unwrap_err()),
            "table_transfer_same_table"
        );
        assert_eq!(
            coded(e.transfer_table(order.id, 99).await.unwrap_err()),
            "table_transfer_target_invalid"
        );
        let moved = e.transfer_table(order.id, 20).await.unwrap();
        assert_eq!(
            (moved.order_id, moved.from_table, moved.to_table),
            (order.id, 7, 20)
        );
        assert_eq!(moved.kitchen_slip, SlipOutcome::NothingToPrint);
        assert_eq!(moved.transfer_slip, SlipOutcome::Printed);
        let tables = e.list_tables().await.unwrap();
        assert_eq!(tables[6].status, "free");
        assert_eq!(tables[19].order_id, Some(order.id));
        assert!(tables[19].pre_bill_printed);

        // A takeaway is a direct sale: leaving it registers nothing.
        let takeaway = e
            .create_order(CreateOrderInput {
                order_type: OrderType::Takeaway,
                table_number: None,
                user_insz: MANAGER_INSZ.into(),
            })
            .await
            .unwrap();
        e.add_order_line(AddOrderLineInput {
            order_id: takeaway.id,
            menu_item_id: 12007,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        })
        .await
        .unwrap();
        let takeaways = e.list_open_takeaways().await.unwrap();
        assert_eq!(takeaways.len(), 1);
        assert_eq!(takeaways[0].order_id, takeaway.id);
        assert_eq!(
            e.leave_order(takeaway.id, LeaveReason::SwitchOrder)
                .await
                .unwrap(),
            SlipOutcome::NothingToPrint
        );

        // Lock: the active order is registered, the session closes, the next call needs a login.
        e.set_active_order(Some(order.id)).await.unwrap();
        let outcome = e.lock_session().await.unwrap();
        assert_eq!(outcome.order_error, None);
        let after = e
            .create_order(CreateOrderInput {
                order_type: OrderType::Takeaway,
                table_number: None,
                user_insz: MANAGER_INSZ.into(),
            })
            .await
            .unwrap_err();
        assert_eq!(coded(after), "not_logged_in");
        e.shutdown().await.unwrap();
    }
}
