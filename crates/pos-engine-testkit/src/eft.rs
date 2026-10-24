//! A linked card payment terminal for the engine's tests: `pos.json`
//! gets a `payment_terminal` section and the engine asks a scripted
//! terminal (`pos_eft::scripted`) in place of a real driver.

use std::sync::Arc;

use pos_core::config::{PaymentTerminalConfig, PaymentTerminalKind};
use pos_core::order::OrderType;
use pos_db::queries::eft::EftPaymentRow;
use pos_eft::scripted::Script;
use pos_eft::{Approval, CardType, Outcome};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};

use super::harness::TestApp;

/// A terminal section that passes `check_config`.
pub fn terminal_config() -> PaymentTerminalConfig {
    PaymentTerminalConfig {
        kind: PaymentTerminalKind::CcvOpi,
        base_url: None,
        api_key_file: None,
        merchant_id: None,
        terminal_id: Some("WS1".into()),
        integrator_id: None,
        management_system_id: None,
        access_protocol: None,
        address: Some("127.0.0.1:4100".into()),
        device_port: None,
        timeout_seconds: 180,
    }
}

/// What a terminal approves `amount_cents` with.
pub fn approval(amount_cents: i64, brand: &str, card_type: CardType) -> Outcome {
    Outcome::Approved(Approval {
        amount_cents,
        card_type,
        brand: Some(brand.to_string()),
        terminal_reference: Some(format!("TXN-{amount_cents}")),
        authorisation_code: Some("AUTH01".into()),
        masked_pan: Some("XXXX XXXX XXXX 1234".into()),
        customer_receipt: None,
    })
}

impl TestApp {
    /// Link a scripted payment terminal: the answers are queued on the
    /// returned script, the calls it received are read back from it.
    pub fn link_terminal(&mut self) -> Arc<Script> {
        let script = Script::new();
        self.state.config.lock().unwrap().payment_terminal = Some(terminal_config());
        self.state.eft_terminal_override = Some(script.clone());
        script
    }

    /// An open takeaway order of one product at `price_cents`, in the
    /// name of the logged-in operator (who needs a valid INSZ: the
    /// default [`TEST_OPERATOR`] has none, log in as someone else first).
    pub fn order_of(&self, price_cents: i64) -> i64 {
        let operator = self
            .state
            .session
            .lock()
            .unwrap()
            .as_ref()
            .expect("a logged-in operator")
            .insz
            .clone();
        let item = {
            let db = self.state.db.lock().unwrap();
            db.execute(
                "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1)",
                [],
            )
            .unwrap();
            db.execute(
                "INSERT INTO menu_items (category_id, price_cents, vat_code, sort_order, active)
                 VALUES (1, ?1, 'B', 0, 1)",
                [price_cents],
            )
            .unwrap();
            db.last_insert_rowid()
        };
        let order = do_create_order(
            &self.state,
            CreateOrderInput {
                order_type: OrderType::Takeaway,
                table_number: None,
                user_insz: operator,
            },
        )
        .unwrap()
        .id;
        do_add_order_line(
            &self.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: item,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
        order
    }

    /// Every stored terminal payment, oldest first.
    pub fn eft_rows(&self) -> Vec<EftPaymentRow> {
        let db = self.state.db.lock().unwrap();
        let ids: Vec<i64> = db
            .prepare("SELECT id FROM eft_payments ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        ids.into_iter()
            .map(|id| pos_db::queries::eft::get(&db, id).unwrap().unwrap())
            .collect()
    }
}
