use std::path::Path;

use serde::{Deserialize, Serialize};

/// Error loading the POS configuration file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid config file {path}: {source}")]
    Parse {
        path: String,
        source: serde_json::Error,
    },
}

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
    #[serde(default)]
    pub printer_address: Option<String>,
    /// Training mode flag
    #[serde(default)]
    pub training_mode: bool,
    /// Display language (fr, nl, de, en, zh). Printed fiscal mentions
    /// follow this key restricted to national languages (fr/nl/de);
    /// any other value prints FR.
    #[serde(default = "default_language")]
    pub language: String,
    /// Directory for SQLite backup files (created after each Z-report).
    /// Defaults to `{data_dir}/backups/` if unset.
    #[serde(default)]
    pub backup_dir: Option<String>,
}

fn default_language() -> String {
    "fr".to_string()
}

impl PosConfig {
    /// Load the configuration from the JSON file documented in the
    /// installation manual (`/etc/moyun-pos/pos.json` in production).
    ///
    /// Identity fields (pos_id, restaurant_vat, fdm_url, …) are
    /// required — a fiscal terminal must not boot with a partial
    /// identity. Only printer_address, training_mode, language and
    /// backup_dir may be omitted.
    pub fn load_from_file(path: &Path) -> Result<Self, ConfigError> {
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.display().to_string(),
            source,
        })
    }
}

impl Default for PosConfig {
    fn default() -> Self {
        Self {
            pos_id: String::new(),
            terminal_id: String::new(),
            device_id: String::new(),
            restaurant_name: "Restaurant Exemple".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The example from MANUEL_INSTALLATION_FR.md §4.2 must parse.
    #[test]
    fn parses_installation_manual_example() {
        let json = r#"{
            "pos_id": "CFOD0061234567",
            "terminal_id": "POS1",
            "device_id": "DEV001",
            "restaurant_name": "Tokyo Sushi Bar",
            "restaurant_address": "Rue Test 1, 1000 Bruxelles",
            "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "1234567890",
            "fdm_url": "http://192.168.1.20:8080/graphql",
            "printer_address": "192.168.1.10:9100",
            "training_mode": false,
            "language": "fr",
            "backup_dir": "/var/lib/moyun-pos/backups"
        }"#;
        let cfg: PosConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.pos_id, "CFOD0061234567");
        assert_eq!(cfg.printer_address.as_deref(), Some("192.168.1.10:9100"));
        assert!(!cfg.training_mode);
    }

    #[test]
    fn optional_fields_may_be_omitted() {
        let json = r#"{
            "pos_id": "CFOD0061234567",
            "terminal_id": "POS1",
            "device_id": "DEV001",
            "restaurant_name": "Tokyo Sushi Bar",
            "restaurant_address": "Rue Test 1, 1000 Bruxelles",
            "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "1234567890",
            "fdm_url": "http://192.168.1.20:8080/graphql"
        }"#;
        let cfg: PosConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.language, "fr");
        assert!(cfg.printer_address.is_none());
        assert!(!cfg.training_mode);
    }

    #[test]
    fn missing_identity_field_is_an_error() {
        // No pos_id — a fiscal terminal must not boot without one.
        let json = r#"{
            "terminal_id": "POS1",
            "device_id": "DEV001",
            "restaurant_name": "Tokyo Sushi Bar",
            "restaurant_address": "Rue Test 1, 1000 Bruxelles",
            "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "1234567890",
            "fdm_url": "http://192.168.1.20:8080/graphql"
        }"#;
        assert!(serde_json::from_str::<PosConfig>(json).is_err());
    }

    #[test]
    fn load_from_missing_file_is_io_error() {
        let err = PosConfig::load_from_file(Path::new("/nonexistent/pos.json")).unwrap_err();
        assert!(matches!(err, ConfigError::Io { .. }));
    }
}
