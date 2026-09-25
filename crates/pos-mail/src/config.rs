//! Digital ticket configuration — the knobs an operator (via
//! `src-tauri`'s POS config file) sets to enable the Scaleway TEM
//! integration.

use serde::{Deserialize, Serialize};

fn default_region() -> String {
    "fr-par".to_string()
}

/// Configuration for the digital-ticket-by-email feature (cert manual
/// §III.H). Deserialized from the POS config file; `src-tauri` is
/// responsible for loading it and wiring a [`crate::tem::TemClient`]
/// and the [`crate::outbox`] worker from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalTicketConfig {
    /// Scaleway region, e.g. `fr-par`.
    #[serde(default = "default_region")]
    pub region: String,
    /// Scaleway project id the TEM domain/API key belongs to.
    pub project_id: String,
    /// Sender address, must be a domain verified in Scaleway TEM.
    pub from_email: String,
    /// Sender display name (e.g. the restaurant name).
    pub from_name: String,
    /// Override for the Scaleway API base URL. Defaults to
    /// `https://api.scaleway.com`; only ever overridden in tests.
    #[serde(default)]
    pub api_base: Option<String>,
    /// Path to a file containing the Scaleway secret key, and nothing
    /// else (trailing whitespace is trimmed). Never put the key
    /// directly in the config file — this keeps it out of the POS
    /// config backups shipped in support bundles.
    pub api_key_file: String,
    /// When true, an operator may send a digital-only ticket (no
    /// paper printed at all) if the send succeeds. When false
    /// (default), every dine-in ticket is printed on paper regardless
    /// — email is always a bonus, never a replacement — which is the
    /// safer default for a fiscal terminal.
    #[serde(default)]
    pub allow_digital_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_the_minimal_shape() {
        let json = r#"{
            "project_id": "11111111-1111-1111-1111-111111111111",
            "from_email": "no-reply@tokyosushibarliege.be",
            "from_name": "Tokyo Sushi Bar",
            "api_key_file": "/etc/moyun-pos/tem.key"
        }"#;
        let cfg: DigitalTicketConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.region, "fr-par");
        assert!(cfg.api_base.is_none());
        assert!(!cfg.allow_digital_only);
    }

    #[test]
    fn deserializes_every_field_when_present() {
        let json = r#"{
            "region": "nl-ams",
            "project_id": "p1",
            "from_email": "a@b.com",
            "from_name": "A",
            "api_base": "https://example.test",
            "api_key_file": "/tmp/key",
            "allow_digital_only": true
        }"#;
        let cfg: DigitalTicketConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.region, "nl-ams");
        assert_eq!(cfg.api_base.as_deref(), Some("https://example.test"));
        assert!(cfg.allow_digital_only);
    }

    #[test]
    fn missing_required_field_fails_to_parse() {
        let json = r#"{ "from_email": "a@b.com" }"#;
        assert!(serde_json::from_str::<DigitalTicketConfig>(json).is_err());
    }
}
