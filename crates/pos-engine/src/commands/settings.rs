use serde::Serialize;

use crate::commands::access::{self, Access};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PosSettings {
    pub training_mode: bool,
    pub printer_address: Option<String>,
    pub fdm_url: String,
    pub pos_id: String,
    pub terminal_id: String,
    pub restaurant_name: String,
    pub language: String,
    /// What the FDM would refuse in `pos.json` ([`PosConfig::problems`]):
    /// shown as a banner so a bad identity is fixed before opening. Each
    /// is a coded error (`config_<setting>|detail=<English text>`), shown
    /// in the screen's language with the text as detail.
    ///
    /// [`PosConfig::problems`]: pos_core::config::PosConfig::problems
    pub config_problems: Vec<String>,
    /// The software version (`posSwVersion`, for example
    /// `pili-pos/1.0.0`): shown on the login screen, AM Art. 47.
    pub pos_sw_version: String,
    /// The kind of the linked card payment terminal, as in `pos.json`
    /// (`worldline_terminal_api`, `ccv_payment_api`, `ccv_opi`); absent
    /// when cards are typed by hand on a standalone terminal. The
    /// checkout asks the terminal for the amount when it is there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terminal: Option<String>,
}

pub fn do_get_settings(state: &AppState) -> Result<PosSettings, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(PosSettings {
        training_mode: config.training_mode,
        printer_address: config.printer_address.clone(),
        fdm_url: config.fdm_url.clone(),
        pos_id: config.pos_id.clone(),
        terminal_id: config.terminal_id.clone(),
        restaurant_name: config.restaurant_name.clone(),
        language: config.language.clone(),
        config_problems: pos_fiscal::fdm_client::FdmClient::config_problems(&config)
            .iter()
            .map(crate::commands::errors::config_problem)
            .collect(),
        pos_sw_version: state.identity.as_str().to_string(),
        payment_terminal: config
            .payment_terminal
            .as_ref()
            .map(|t| t.kind.as_str().to_string()),
    })
}

pub const ERR_OPEN_ORDERS: &str = crate::commands::errors::OPEN_ORDERS_TRAINING;

/// Switch training mode. Managers only, and never while an order with
/// lines is open: a real table would otherwise be paid as a training
/// sale (T, never counted), or a training order paid as a real one.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "IPC/FFI contract: mirrors the bool argument of the Tauri command and the uniffi export"
)]
pub fn do_set_training_mode(state: &AppState, enabled: bool) -> Result<bool, String> {
    access::operator(state, Access::Admin)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let busy: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM orders o
                 WHERE o.status = 'open'
                   AND EXISTS (SELECT 1 FROM order_lines l WHERE l.order_id = o.id)",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        if config.training_mode == enabled {
            return Ok(enabled);
        }
        if busy > 0 {
            return Err(ERR_OPEN_ORDERS.to_string());
        }
        config.training_mode = enabled;
    }
    Ok(enabled)
}
