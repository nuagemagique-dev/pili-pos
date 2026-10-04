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
    /// Restaurant name: the commercial name at the top of the ticket
    pub restaurant_name: String,
    /// Legal name of the company that runs the restaurant ("Denomination
    /// legale" at the bottom of the ticket, AM of 29 April 2024 art. 20
    /// b). Absent when it is the commercial name.
    #[serde(default)]
    pub restaurant_legal_name: Option<String>,
    /// Restaurant address
    pub restaurant_address: String,
    /// Restaurant VAT number (BE format)
    pub restaurant_vat: String,
    /// Establishment number (10 digits, starts 2-8)
    pub restaurant_est_no: String,
    /// FDM endpoint URL (e.g. http://192.168.1.100:8080/graphql)
    pub fdm_url: String,
    /// File holding the bearer token of an FDM that requires one (a
    /// cloud FDM simulator used for tests). Absent for a LAN FDM. The
    /// token stays out of `pos.json`, like the e-mail API key.
    #[serde(default)]
    pub fdm_token_file: Option<String>,
    /// HTTPS to the FDM (`fdm_url` in https): the CA that signs the
    /// FDM's server certificate (PEM). module2 publishes one CA for all
    /// its FDMs and its simulator; their certificates carry no host
    /// name, so the FDM is trusted on that signature alone.
    #[serde(default)]
    pub fdm_ca_file: Option<String>,
    /// Client certificate and key (PEM) for an FDM that asks for mTLS
    /// (module2 on port 443). Both or neither.
    #[serde(default)]
    pub fdm_client_cert_file: Option<String>,
    #[serde(default)]
    pub fdm_client_key_file: Option<String>,
    /// Printer address (TCP, e.g. 192.168.1.200:9100)
    #[serde(default)]
    pub printer_address: Option<String>,
    /// Printer of the bar/kitchen order slips (TCP). Absent: the slips
    /// print on `printer_address`.
    #[serde(default)]
    pub kitchen_printer_address: Option<String>,
    /// Hour (Belgian time, 0 to 12) at which a business day ends: a
    /// service running past midnight stays on its day until then. Once
    /// it has passed, a day left without a Z report is closed by an
    /// automatic Z (producers FAQ Q501c).
    #[serde(default = "default_day_end_hour")]
    pub day_end_hour: u32,
    /// Number of tables on the table plan, numbered from 1.
    #[serde(default = "default_table_count")]
    pub table_count: u32,
    /// Training mode flag
    #[serde(default)]
    pub training_mode: bool,
    /// Whether the HubRise integration runs on this installation (its
    /// starting value: an admin can change it in Settings, every change
    /// logged). Off by default.
    #[serde(default)]
    pub hubrise_enabled: bool,
    /// How this installation connects to HubRise (its OAuth client).
    /// Absent: HubRise cannot be connected (release builds).
    #[serde(default)]
    pub hubrise: Option<HubriseConfig>,
    /// Display language (fr, nl, de, en, zh). Printed fiscal mentions
    /// follow this key restricted to national languages (fr/nl/de);
    /// any other value prints FR.
    #[serde(default = "default_language")]
    pub language: String,
    /// Directory for SQLite backup files (created after each Z-report).
    /// Defaults to `{data_dir}/backups/` if unset.
    #[serde(default)]
    pub backup_dir: Option<String>,
    /// Digital ticket by email (cert manual §III.H). Absent = the POS
    /// offers paper tickets only.
    #[serde(default)]
    pub digital_ticket: Option<DigitalTicketConfig>,
}

fn default_language() -> String {
    "fr".to_string()
}

fn default_day_end_hour() -> u32 {
    5
}

fn default_table_count() -> u32 {
    20
}

impl PosConfig {
    /// The legal name printed on tickets: the commercial name unless
    /// a distinct legal name is set.
    pub fn legal_name(&self) -> &str {
        self.restaurant_legal_name
            .as_deref()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or(&self.restaurant_name)
    }

    /// Load the configuration from the JSON file documented in the
    /// installation manual (`/etc/pili-pos/pos.json` in production).
    ///
    /// Identity fields (pos_id, restaurant_vat, fdm_url, …) are
    /// required - a fiscal terminal must not boot with a partial
    /// identity. Only printer_address, kitchen_printer_address,
    /// training_mode, language, day_end_hour, backup_dir,
    /// digital_ticket, hubrise_enabled and hubrise may be omitted.
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

    /// What the FDM would refuse in this identity (POS spec v2.2
    /// §2.2.2), or the ticket could not show. Empty when the
    /// configuration is usable. Checked at start-up and by
    /// `pili-pos --check-config`, so a typo in `pos.json` shows at
    /// installation, not as refused sales on opening day.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let pos_id_ok = self.pos_id.len() == 14
            && self
                .pos_id
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        if !pos_id_ok {
            out.push(format!(
                "pos_id: 14 upper-case letters or digits expected, got {:?}",
                self.pos_id
            ));
        }
        if !valid_vat_no(&self.restaurant_vat) {
            out.push(format!(
                "restaurant_vat: BE followed by 10 digits starting with 0 or 1, valid modulo 97 \
                 (e.g. BE0499999960), got {:?}",
                self.restaurant_vat
            ));
        }
        if !valid_est_no(&self.restaurant_est_no) {
            out.push(format!(
                "restaurant_est_no: 10 digits starting with 2 to 8, valid modulo 97 \
                 (e.g. 8789456149), got {:?}",
                self.restaurant_est_no
            ));
        }
        for (key, value) in [
            ("terminal_id", &self.terminal_id),
            ("device_id", &self.device_id),
        ] {
            if value.trim().is_empty() || value.chars().count() > 600 {
                out.push(format!("{key}: 1 to 600 characters expected"));
            }
        }
        for (key, value) in [
            ("restaurant_name", &self.restaurant_name),
            ("restaurant_address", &self.restaurant_address),
        ] {
            if value.trim().is_empty() {
                out.push(format!("{key}: printed on every ticket, must not be empty"));
            }
        }
        if !(self.fdm_url.starts_with("http://") || self.fdm_url.starts_with("https://")) {
            out.push(format!(
                "fdm_url: http(s) URL expected, got {:?}",
                self.fdm_url
            ));
        }
        if !["fr", "nl", "de", "en", "zh"].contains(&self.language.as_str()) {
            out.push(format!(
                "language: one of fr, nl, de, en, zh expected, got {:?}",
                self.language
            ));
        }
        if let Some(path) = &self.fdm_token_file {
            match std::fs::read_to_string(path) {
                Ok(token) if !token.trim().is_empty() => {}
                Ok(_) => out.push(format!("fdm_token_file: {path:?} is empty")),
                Err(e) => out.push(format!("fdm_token_file: cannot read {path:?}: {e}")),
            }
        }
        for (key, path) in [
            ("fdm_ca_file", &self.fdm_ca_file),
            ("fdm_client_cert_file", &self.fdm_client_cert_file),
            ("fdm_client_key_file", &self.fdm_client_key_file),
        ] {
            if let Some(path) = path
                && let Err(e) = std::fs::metadata(path)
            {
                out.push(format!("{key}: cannot read {path:?}: {e}"));
            }
        }
        if self.fdm_client_cert_file.is_some() != self.fdm_client_key_file.is_some() {
            out.push("fdm_client_cert_file and fdm_client_key_file go together".into());
        }
        if self.fdm_url.starts_with("https://") && self.fdm_ca_file.is_none() {
            out.push("fdm_url is https: set fdm_ca_file to the FDM's CA".into());
        }
        if self.day_end_hour > 12 {
            out.push(format!(
                "day_end_hour: 0 to 12 expected, got {}",
                self.day_end_hour
            ));
        }
        if let Some(h) = &self.hubrise {
            out.extend(h.problems());
        }
        if let Some(d) = &self.digital_ticket {
            if d.project_id.trim().is_empty() {
                out.push("digital_ticket.project_id: must not be empty".to_string());
            }
            if !d.from_email.contains('@') {
                out.push(format!(
                    "digital_ticket.from_email: an e-mail address expected, got {:?}",
                    d.from_email
                ));
            }
        }
        out
    }
}

/// `true` when the last two digits are 97 minus the first eight
/// modulo 97 (Belgian company and establishment numbers).
fn mod97_ok(digits: &str) -> bool {
    let (Ok(base), Ok(check)) = (digits[..8].parse::<u64>(), digits[8..].parse::<u64>()) else {
        return false;
    };
    97 - base % 97 == check
}

/// `vatNo`: "BE" and 10 digits, the first 0 or 1, valid modulo 97.
pub fn valid_vat_no(s: &str) -> bool {
    let Some(digits) = s.strip_prefix("BE") else {
        return false;
    };
    digits.len() == 10
        && digits.chars().all(|c| c.is_ascii_digit())
        && matches!(digits.as_bytes()[0], b'0' | b'1')
        && mod97_ok(digits)
}

/// `estNo`: 10 digits, the first 2 to 8, valid modulo 97.
pub fn valid_est_no(s: &str) -> bool {
    s.len() == 10
        && s.chars().all(|c| c.is_ascii_digit())
        && (b'2'..=b'8').contains(&s.as_bytes()[0])
        && mod97_ok(s)
}

impl Default for PosConfig {
    fn default() -> Self {
        Self {
            table_count: default_table_count(),
            pos_id: String::new(),
            terminal_id: String::new(),
            device_id: String::new(),
            restaurant_name: "Restaurant Exemple".to_string(),
            restaurant_address: String::new(),
            restaurant_vat: String::new(),
            restaurant_est_no: String::new(),
            fdm_url: "http://localhost:8080/graphql".to_string(),
            printer_address: None,
            kitchen_printer_address: None,
            fdm_token_file: None,
            fdm_ca_file: None,
            fdm_client_cert_file: None,
            fdm_client_key_file: None,
            restaurant_legal_name: None,
            training_mode: false,
            hubrise_enabled: false,
            hubrise: None,
            language: "fr".to_string(),
            day_end_hour: default_day_end_hour(),
            backup_dir: None,
            digital_ticket: None,
        }
    }
}

fn default_hubrise_oauth_base_url() -> String {
    "https://manager.hubrise.com/oauth2/v1".to_string()
}

fn default_hubrise_api_base_url() -> String {
    "https://api.hubrise.com/v1".to_string()
}

fn default_hubrise_redirect_uri() -> String {
    // The installed-app flow: HubRise shows the code to paste back.
    "urn:ietf:wg:oauth:2.0:oob".to_string()
}

fn default_hubrise_scope() -> String {
    "location[orders.write,customer_list.read,catalog.read]".to_string()
}

/// The installation's HubRise OAuth client, from `pos.json` as
/// `hubrise`. Only the client id and the secret's file are required;
/// the rest defaults to the real HubRise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubriseConfig {
    /// OAuth client id (`<id>.clients.hubrise.com`).
    pub client_id: String,
    /// Path to a file holding the client secret and nothing else
    /// (trailing whitespace trimmed); never the secret itself in
    /// `pos.json`.
    pub client_secret_file: String,
    #[serde(default = "default_hubrise_oauth_base_url")]
    pub oauth_base_url: String,
    #[serde(default = "default_hubrise_api_base_url")]
    pub api_base_url: String,
    #[serde(default = "default_hubrise_redirect_uri")]
    pub redirect_uri: String,
    #[serde(default = "default_hubrise_scope")]
    pub scope: String,
}

impl HubriseConfig {
    /// The client secret, read from `client_secret_file`.
    pub fn client_secret(&self) -> Result<String, String> {
        let path = &self.client_secret_file;
        match std::fs::read_to_string(path) {
            Ok(s) if !s.trim().is_empty() => Ok(s.trim().to_string()),
            Ok(_) => Err(format!("hubrise.client_secret_file: {path:?} is empty")),
            Err(e) => Err(format!(
                "hubrise.client_secret_file: cannot read {path:?}: {e}"
            )),
        }
    }

    fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.client_id.trim().is_empty() {
            out.push("hubrise.client_id: must not be empty".to_string());
        }
        if let Err(e) = self.client_secret() {
            out.push(e);
        }
        for (key, url) in [
            ("hubrise.oauth_base_url", &self.oauth_base_url),
            ("hubrise.api_base_url", &self.api_base_url),
        ] {
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                out.push(format!("{key}: http(s) URL expected, got {url:?}"));
            }
        }
        out
    }
}

/// Configuration for the digital ticket sent by email through the
/// Scaleway Transactional Email API. Loaded from `pos.json` as
/// `digital_ticket`; `pos-mail` builds its client from it.
fn default_region() -> String {
    "fr-par".to_string()
}

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
    /// directly in the config file - this keeps it out of the POS
    /// config backups shipped in support bundles.
    pub api_key_file: String,
    /// When true, an operator may send a digital-only ticket (no
    /// paper printed at all) if the send succeeds. When false
    /// (default), every dine-in ticket is printed on paper regardless
    /// (email is always a bonus, never a replacement), which is the
    /// safer default for a fiscal terminal.
    #[serde(default)]
    pub allow_digital_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> PosConfig {
        PosConfig {
            pos_id: "CFOD0061234567".into(),
            terminal_id: "POS1".into(),
            device_id: "00:1b:44:11:3a:b7".into(),
            restaurant_name: "Restaurant Exemple".into(),
            restaurant_address: "Rue de l'Exemple 1, 4000 Liège".into(),
            restaurant_vat: "BE0499999960".into(),
            restaurant_est_no: "8789456149".into(),
            fdm_url: "http://192.168.1.20:8081/graphql".into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_complete_identity_has_no_problem() {
        assert_eq!(valid().problems(), Vec::<String>::new());
        // The SPF test setup's identity is valid too.
        let spf = PosConfig {
            restaurant_vat: "BE0000000097".into(),
            restaurant_est_no: "2000000042".into(),
            ..valid()
        };
        assert!(spf.problems().is_empty(), "{:?}", spf.problems());
    }

    /// The key a problem must name, and the typo that causes it.
    type Typo = (&'static str, Box<dyn Fn(&mut PosConfig)>);

    #[test]
    fn every_typo_an_installer_can_make_is_reported() {
        let cases: Vec<Typo> = vec![
            ("pos_id", Box::new(|c| c.pos_id = "cfod0061234567".into())),
            ("pos_id", Box::new(|c| c.pos_id = "CFOD006123456".into())),
            (
                "restaurant_vat",
                Box::new(|c| c.restaurant_vat = "BE 0499.999.960".into()),
            ),
            (
                "restaurant_vat",
                Box::new(|c| c.restaurant_vat = "BE0499999961".into()),
            ),
            (
                "restaurant_vat",
                Box::new(|c| c.restaurant_vat = "0499999960".into()),
            ),
            (
                "restaurant_vat",
                Box::new(|c| c.restaurant_vat = "BE2499999960".into()),
            ),
            (
                "restaurant_est_no",
                Box::new(|c| c.restaurant_est_no = "878945614".into()),
            ),
            (
                "restaurant_est_no",
                Box::new(|c| c.restaurant_est_no = "8789456148".into()),
            ),
            (
                "restaurant_est_no",
                Box::new(|c| c.restaurant_est_no = "1234567890".into()),
            ),
            ("terminal_id", Box::new(|c| c.terminal_id = " ".into())),
            ("device_id", Box::new(|c| c.device_id = "x".repeat(601))),
            (
                "restaurant_name",
                Box::new(|c| c.restaurant_name = String::new()),
            ),
            (
                "restaurant_address",
                Box::new(|c| c.restaurant_address = "  ".into()),
            ),
            (
                "fdm_url",
                Box::new(|c| c.fdm_url = "192.168.1.20:8081".into()),
            ),
            ("language", Box::new(|c| c.language = "FR".into())),
        ];
        for (key, broken) in cases {
            let mut c = valid();
            broken(&mut c);
            let problems = c.problems();
            assert_eq!(problems.len(), 1, "{key}: {problems:?}");
            assert!(problems[0].starts_with(key), "{key}: {problems:?}");
        }
    }

    #[test]
    fn a_broken_email_setup_is_reported() {
        let mut c = valid();
        c.digital_ticket = Some(DigitalTicketConfig {
            region: "fr-par".into(),
            project_id: " ".into(),
            from_email: "tickets.example.com".into(),
            from_name: "Restaurant".into(),
            api_base: None,
            api_key_file: "/etc/pili-pos/secrets/tem.key".into(),
            allow_digital_only: false,
        });
        let problems = c.problems();
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn modulo_97_numbers() {
        assert!(valid_vat_no("BE0499999960"));
        assert!(valid_vat_no("BE0000000097"));
        assert!(!valid_vat_no("BE049999996"));
        assert!(!valid_vat_no("BEO499999960"));
        assert!(valid_est_no("8789456149"));
        assert!(valid_est_no("2000000042"));
        assert!(valid_est_no("2123456791"));
        assert!(!valid_est_no("2123456789"));
        assert!(!valid_est_no("9789456149"), "starts with 2 to 8");
    }

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
            "restaurant_est_no": "2123456791",
            "fdm_url": "http://192.168.1.20:8080/graphql",
            "printer_address": "192.168.1.10:9100",
            "training_mode": false,
            "language": "fr",
            "backup_dir": "/var/lib/pili-pos/backups"
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
            "restaurant_est_no": "2123456791",
            "fdm_url": "http://192.168.1.20:8080/graphql"
        }"#;
        let cfg: PosConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.language, "fr");
        assert!(cfg.printer_address.is_none());
        assert!(!cfg.training_mode);
    }

    #[test]
    fn missing_identity_field_is_an_error() {
        // No pos_id - a fiscal terminal must not boot without one.
        let json = r#"{
            "terminal_id": "POS1",
            "device_id": "DEV001",
            "restaurant_name": "Tokyo Sushi Bar",
            "restaurant_address": "Rue Test 1, 1000 Bruxelles",
            "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "2123456791",
            "fdm_url": "http://192.168.1.20:8080/graphql"
        }"#;
        assert!(serde_json::from_str::<PosConfig>(json).is_err());
    }

    #[test]
    fn load_from_missing_file_is_io_error() {
        let err = PosConfig::load_from_file(Path::new("/nonexistent/pos.json")).unwrap_err();
        assert!(matches!(err, ConfigError::Io { .. }));
    }

    #[test]
    fn load_from_malformed_json_is_parse_error() {
        let dir = std::env::temp_dir().join(format!("pili-pos-config-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pos.json");
        std::fs::write(&path, "{ not valid json").unwrap();

        let err = PosConfig::load_from_file(&path).unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn default_config_is_never_used_as_a_live_identity() {
        // The Default impl exists for ergonomic test fixtures; assert
        // it is obviously not a real POS identity (empty pos_id) so a
        // future refactor can't accidentally make it "boot-ready".
        let cfg = PosConfig::default();
        assert!(cfg.pos_id.is_empty());
        assert!(cfg.restaurant_vat.is_empty());
        assert_eq!(cfg.language, "fr");
    }

    #[test]
    fn unknown_language_code_still_parses_the_config() {
        // Language restriction to fr/nl/de is enforced downstream
        // (ReceiptLanguage::from_code in pos-fiscal), not by the
        // config parser itself - any string is accepted here.
        let json = r#"{
            "pos_id": "CFOD0061234567",
            "terminal_id": "POS1",
            "device_id": "DEV001",
            "restaurant_name": "Tokyo Sushi Bar",
            "restaurant_address": "Rue Test 1, 1000 Bruxelles",
            "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "2123456791",
            "fdm_url": "http://192.168.1.20:8080/graphql",
            "language": "zh"
        }"#;
        let cfg: PosConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.language, "zh");
    }

    #[test]
    fn deserializes_the_minimal_shape() {
        let json = r#"{
            "project_id": "11111111-1111-1111-1111-111111111111",
            "from_email": "tickets@restaurant-exemple.be",
            "from_name": "Restaurant Exemple",
            "api_key_file": "/etc/pili-pos/tem.key"
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

    #[test]
    fn hubrise_needs_only_its_client_id_and_secret_file() {
        let json = r#"{ "client_id": "123.clients.hubrise.com", "client_secret_file": "/etc/pili-pos/hubrise.secret" }"#;
        let h: HubriseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(h.oauth_base_url, "https://manager.hubrise.com/oauth2/v1");
        assert_eq!(h.api_base_url, "https://api.hubrise.com/v1");
        assert_eq!(h.redirect_uri, "urn:ietf:wg:oauth:2.0:oob");
        assert!(h.scope.starts_with("location["));
        // Without its secret file, the section does not parse.
        assert!(serde_json::from_str::<HubriseConfig>(r#"{ "client_id": "x" }"#).is_err());
        assert!(PosConfig::default().hubrise.is_none());
    }

    #[test]
    fn a_hubrise_section_is_checked_with_its_secret() {
        let dir = tempfile_dir("hubrise");
        let secret = dir.join("hubrise.secret");
        let config = |file: &std::path::Path| PosConfig {
            hubrise: Some(HubriseConfig {
                client_id: "123.clients.hubrise.com".into(),
                client_secret_file: file.display().to_string(),
                oauth_base_url: default_hubrise_oauth_base_url(),
                api_base_url: default_hubrise_api_base_url(),
                redirect_uri: default_hubrise_redirect_uri(),
                scope: default_hubrise_scope(),
            }),
            ..valid()
        };

        let missing = config(&secret).problems();
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(missing[0].starts_with("hubrise.client_secret_file"));

        std::fs::write(&secret, "  \n").unwrap();
        let empty = config(&secret).problems();
        assert!(empty[0].contains("is empty"), "{empty:?}");

        std::fs::write(&secret, "s3cret\n").unwrap();
        let ok = config(&secret);
        assert!(ok.problems().is_empty(), "{:?}", ok.problems());
        assert_eq!(ok.hubrise.unwrap().client_secret().unwrap(), "s3cret");

        let mut bad = config(&secret);
        let h = bad.hubrise.as_mut().unwrap();
        h.client_id = " ".into();
        h.api_base_url = "api.hubrise.com".into();
        let problems = bad.problems();
        assert_eq!(problems.len(), 2, "{problems:?}");
        // A problem never shows the secret.
        assert!(problems.iter().all(|p| !p.contains("s3cret")));
        std::fs::remove_dir_all(&dir).ok();
    }

    fn tempfile_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pos-core-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn digital_ticket_defaults_to_none() {
        let cfg = PosConfig::default();
        assert!(cfg.digital_ticket.is_none());
    }
}
