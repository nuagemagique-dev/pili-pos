use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosConfig {
    /// 14-character POS identifier (SCE 2.0)
    pub pos_id: String,
    /// Terminal identifier
    pub terminal_id: String,
    /// Device identifier
    pub device_id: String,
    /// Restaurant name
    pub restaurant_name: String,
    /// Restaurant address
    pub restaurant_address: String,
    /// Restaurant VAT number (BE format)
    pub restaurant_vat: String,
    /// Establishment number (10 digits, starts 2-8)
    pub restaurant_est_no: String,
    /// FDM endpoint URL (e.g. http://192.168.1.100:8080/graphql)
    pub fdm_url: String,
    /// Printer address (TCP, e.g. 192.168.1.200:9100)
    pub printer_address: Option<String>,
    /// Training mode flag
    pub training_mode: bool,
    /// Display language (fr, nl, en, ja)
    pub language: String,
    /// Directory for SQLite backup files (created after each Z-report).
    /// Defaults to `{data_dir}/backups/` if unset.
    pub backup_dir: Option<String>,
}

impl Default for PosConfig {
    fn default() -> Self {
        Self {
            pos_id: String::new(),
            terminal_id: String::new(),
            device_id: String::new(),
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: String::new(),
            restaurant_vat: String::new(),
            restaurant_est_no: String::new(),
            fdm_url: "http://localhost:8080/graphql".to_string(),
            printer_address: None,
            training_mode: false,
            language: "fr".to_string(),
            backup_dir: None,
        }
    }
}
