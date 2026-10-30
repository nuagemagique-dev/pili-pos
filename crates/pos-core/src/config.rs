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
    /// Paper roll of `printer_address`, in mm: 80 (default) or 58.
    #[serde(default)]
    pub printer_paper_mm: PaperWidth,
    /// Paper roll of `kitchen_printer_address`, in mm: 80 (default) or
    /// 58. Unused without a kitchen printer.
    #[serde(default)]
    pub kitchen_printer_paper_mm: PaperWidth,
    /// Hour (Belgian time, 0 to 12) at which a business day ends: a
    /// service running past midnight stays on its day until then. Once
    /// it has passed, a day left without a Z report is closed by an
    /// automatic Z (producers FAQ Q501c).
    #[serde(default = "default_day_end_hour")]
    pub day_end_hour: u32,
    /// Number of tables on the table plan, numbered from 1. Used when
    /// `tables` is absent.
    #[serde(default = "default_table_count")]
    pub table_count: u32,
    /// The table numbers as ranges ("1-12, 20-25", see
    /// [`crate::table_plan`]): the starting plan, which a manager can
    /// change in the settings. Absent: 1 to `table_count`.
    #[serde(default)]
    pub tables: Option<String>,
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
    /// Nightly export after each real Z report: an encrypted backup of
    /// the database, a statistics file without personal data and a
    /// manifest, sent to a per-installation HTTPS endpoint. Absent =
    /// no export.
    #[serde(default)]
    pub export: Option<ExportConfig>,
    /// The card payment terminal linked to the till (EFT): the till
    /// sends the amount and records the terminal's answer, payment lines
    /// `AUTOMATIC`. Absent: card payments are typed on a standalone
    /// terminal and confirmed by the operator (`MANUAL`).
    #[serde(default)]
    pub payment_terminal: Option<PaymentTerminalConfig>,
    /// The checkout scale (Dialog 06). Absent: no scale, and a product sold
    /// by weight cannot be sold.
    #[serde(default)]
    pub scale: Option<ScaleConfig>,
    /// Peppol delivery of B2B invoices (event I). Absent: invoices are
    /// still made and printed, but nothing is queued for delivery.
    #[serde(default)]
    pub peppol: Option<PeppolConfig>,
    /// The owner alerts' transport: where the Telegram bot token
    /// is, and the API base for tests. Absent: no alert can be sent
    /// (the switches and the chat id are owner settings, not here).
    #[serde(default)]
    pub alerts: Option<AlertsConfig>,
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
    /// The starting table plan in its canonical form: `tables` when it
    /// reads, else 1 to `table_count` (within the plan's limits).
    pub fn table_plan(&self) -> String {
        if let Some(text) = &self.tables
            && let Ok(plan) = crate::table_plan::normalize(text)
        {
            return plan;
        }
        let count = self.table_count.min(crate::table_plan::MAX_TABLES as u32);
        let numbers: Vec<u32> = (1..=count).collect();
        crate::table_plan::format(&numbers)
    }
}

impl PosConfig {
    /// The paper roll of the printer at `address`: the kitchen printer's
    /// when `address` is `kitchen_printer_address` (and not also
    /// `printer_address`), else the ticket printer's.
    pub fn paper_for(&self, address: &str) -> PaperWidth {
        let is = |a: &Option<String>| a.as_deref() == Some(address);
        if !is(&self.printer_address) && is(&self.kitchen_printer_address) {
            self.kitchen_printer_paper_mm
        } else {
            self.printer_paper_mm
        }
    }

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
    /// printer_paper_mm, kitchen_printer_paper_mm,
    /// training_mode, language, day_end_hour, backup_dir,
    /// digital_ticket, export, hubrise_enabled, hubrise and
    /// payment_terminal may be omitted.
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

    /// Make every relative file or directory path of the configuration
    /// absolute by joining it to `base`: `fdm_token_file`, `fdm_ca_file`,
    /// `fdm_client_cert_file`, `fdm_client_key_file`, `backup_dir`,
    /// `hubrise.client_secret_file`, `digital_ticket.api_key_file`,
    /// `export.token_file`, `payment_terminal.api_key_file` and
    /// `alerts.telegram_bot_token_file`.
    /// An absolute path, an empty value, a URL or any other field is left
    /// as it is. Used for a configuration bundle that names its files
    /// relative to its own directory.
    pub fn resolve_relative_to(&mut self, base: &Path) {
        for path in [
            &mut self.fdm_token_file,
            &mut self.fdm_ca_file,
            &mut self.fdm_client_cert_file,
            &mut self.fdm_client_key_file,
            &mut self.backup_dir,
        ]
        .into_iter()
        .flatten()
        {
            resolve_path(path, base);
        }
        if let Some(h) = &mut self.hubrise {
            resolve_path(&mut h.client_secret_file, base);
        }
        if let Some(d) = &mut self.digital_ticket {
            resolve_path(&mut d.api_key_file, base);
        }
        if let Some(e) = &mut self.export {
            resolve_path(&mut e.token_file, base);
        }
        if let Some(path) = self
            .payment_terminal
            .as_mut()
            .and_then(|t| t.api_key_file.as_mut())
        {
            resolve_path(path, base);
        }
        if let Some(path) = self.peppol.as_mut().and_then(|p| p.api_key_file.as_mut()) {
            resolve_path(path, base);
        }
        if let Some(path) = self
            .alerts
            .as_mut()
            .and_then(|a| a.telegram_bot_token_file.as_mut())
        {
            resolve_path(path, base);
        }
    }

    /// What the FDM would refuse in this identity (POS spec v2.2
    /// §2.2.2), or the ticket could not show. Empty when the
    /// configuration is usable. Checked at start-up and by
    /// `pili-pos --check-config`, so a typo in `pos.json` shows at
    /// installation, not as refused sales on opening day.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(text) = &self.tables
            && let Err(e) = crate::table_plan::parse(text)
        {
            out.push(format!(
                "tables: ranges such as \"1-12, 20-25\" expected: {e}"
            ));
        }
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
        if let Some(e) = &self.export {
            out.extend(e.problems());
        }
        if let Some(t) = &self.payment_terminal {
            out.extend(t.problems());
        }
        if let Some(s) = &self.scale {
            out.extend(s.problems());
        }
        if let Some(p) = &self.peppol {
            out.extend(p.problems());
        }
        if let Some(a) = &self.alerts {
            out.extend(a.problems());
        }
        out
    }
}

/// The stable code of one text of [`PosConfig::problems`] (without the
/// `config_` prefix the screens' texts carry): the setting it is about.
/// The texts are English, for the installer; screens show a translated
/// headline chosen by this code and keep the text as a detail. Every
/// problem names its setting first (`key: ...`); an unknown one is
/// `other`.
pub fn problem_code(problem: &str) -> &'static str {
    let key = problem.split([':', ' ']).next().unwrap_or_default();
    match key {
        "tables" => "tables",
        "pos_id" => "pos_id",
        "restaurant_vat" => "restaurant_vat",
        "restaurant_est_no" => "restaurant_est_no",
        "terminal_id" | "device_id" => "identity",
        "restaurant_name" | "restaurant_address" => "ticket_header",
        "fdm_url" if problem.starts_with("fdm_url is https") => "fdm_files",
        "fdm_url" => "fdm_url",
        "language" => "language",
        "fdm_token_file"
        | "fdm_ca_file"
        | "fdm_client_cert_file"
        | "fdm_client_key_file"
        | "FDM" => "fdm_files",
        "day_end_hour" => "day_end_hour",
        "no" if problem.starts_with("no configuration") => "missing",
        k if k.starts_with("hubrise.") => "hubrise",
        k if k.starts_with("digital_ticket.") => "digital_ticket",
        k if k.starts_with("export.") => "export",
        k if k.starts_with("payment_terminal.") => "payment_terminal",
        k if k.starts_with("scale.") => "scale",
        k if k.starts_with("peppol.") => "peppol",
        k if k.starts_with("alerts.") => "alerts",
        _ => "other",
    }
}

/// Every code [`problem_code`] can return, for the translation check.
pub const PROBLEM_CODES: &[&str] = &[
    "tables",
    "pos_id",
    "restaurant_vat",
    "restaurant_est_no",
    "identity",
    "ticket_header",
    "fdm_url",
    "fdm_files",
    "language",
    "day_end_hour",
    "missing",
    "hubrise",
    "digital_ticket",
    "export",
    "payment_terminal",
    "scale",
    "peppol",
    "alerts",
    "other",
];

/// Join `path` to `base` when it is a non-empty relative path.
fn resolve_path(path: &mut String, base: &Path) {
    if path.is_empty() || !Path::new(path.as_str()).is_relative() {
        return;
    }
    *path = base.join(path.as_str()).to_string_lossy().into_owned();
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
            tables: None,
            payment_terminal: None,
            scale: None,
            peppol: None,
            alerts: None,
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
            printer_paper_mm: PaperWidth::default(),
            kitchen_printer_paper_mm: PaperWidth::default(),
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
            export: None,
        }
    }
}

/// Width of a thermal printer's paper roll. Written in `pos.json` as
/// the number of millimetres, 80 or 58; any other number refuses the
/// configuration. An Epson TM-m30III takes either, 58 mm with the roll
/// guides shipped with it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub enum PaperWidth {
    #[default]
    Mm80,
    Mm58,
}

impl TryFrom<u16> for PaperWidth {
    type Error = String;

    fn try_from(mm: u16) -> Result<Self, Self::Error> {
        match mm {
            80 => Ok(Self::Mm80),
            58 => Ok(Self::Mm58),
            _ => Err(format!("paper width must be 80 or 58 (mm), got {mm}")),
        }
    }
}

impl From<PaperWidth> for u16 {
    fn from(paper: PaperWidth) -> Self {
        match paper {
            PaperWidth::Mm80 => 80,
            PaperWidth::Mm58 => 58,
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

fn default_keep_days() -> u32 {
    14
}

/// Nightly export (phase X1): where the encrypted backup, the
/// statistics file and the manifest go after each real Z report, and
/// for whom they are encrypted. Loaded from `pos.json` as `export`;
/// `pos-export` builds its settings from it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportConfig {
    /// Per-installation endpoint. `https://`; plain `http://` only to
    /// 127.0.0.1, localhost, [::1] or 10.0.2.2 (tests, emulator).
    pub url: String,
    /// File holding the bearer token and nothing else (whitespace is
    /// trimmed). The token stays out of `pos.json`.
    pub token_file: String,
    /// age X25519 public key (`age1...`) of the owner: can always
    /// decrypt the backup.
    pub owner_recipient: String,
    /// age X25519 public key of the producer (optional second
    /// recipient).
    #[serde(default)]
    pub producer_recipient: Option<String>,
    /// Days the exports stay in the local outbox (1 to 365).
    #[serde(default = "default_keep_days")]
    pub keep_days: u32,
}

/// Which protocol the linked payment terminal speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentTerminalKind {
    /// Worldline Terminal API (nexo Retailer 5.1 JSON over HTTPS, the
    /// terminal reached through the Worldline cloud): Worldline Android
    /// terminals running the Terminal API app.
    WorldlineTerminalApi,
    /// CCV Payment API, method `terminal` (HTTPS, through the CCV cloud).
    CcvPaymentApi,
    /// OPI (Open Payment Initiative XML) over the shop's LAN, as CCV
    /// terminals speak it: channel 0 to the terminal, channel 1 back.
    CcvOpi,
}

impl PaymentTerminalKind {
    /// The `pos.json` spelling, also stored with each terminal payment.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorldlineTerminalApi => "worldline_terminal_api",
            Self::CcvPaymentApi => "ccv_payment_api",
            Self::CcvOpi => "ccv_opi",
        }
    }
}

fn default_terminal_timeout_seconds() -> u64 {
    180
}

/// The card payment terminal linked to the till, from `pos.json` as
/// One checksum and correction value the scale wants in its software
/// check, four hex digits each.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScaleCheckValue {
    pub checksum: String,
    pub correction: String,
}

fn default_scale_price_digits() -> u8 {
    6
}

fn default_scale_weigh_timeout_ms() -> u64 {
    15_000
}

/// `scale`: the checkout scale, a Dibal DPOS-400 in its Dialog 06 protocol,
/// reached over TCP (a serial device server in front of its RS-232 port).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScaleConfig {
    /// `host:port`.
    pub tcp_address: String,
    /// Digits of the unit price field the scale is set up for, 5 or 6.
    #[serde(default = "default_scale_price_digits")]
    pub price_digits: u8,
    /// The answer to the scale's software check, at most 5 pairs. Empty:
    /// a scale that asks for the check stays unusable, there is no way
    /// around it.
    #[serde(default)]
    pub check_values: Vec<ScaleCheckValue>,
    /// Longest weighing, 3 to 120 seconds (15 s when absent).
    #[serde(default = "default_scale_weigh_timeout_ms")]
    pub weigh_timeout_ms: u64,
}

impl ScaleConfig {
    /// What is wrong with this section, each text starting with its key.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let address_ok = self
            .tcp_address
            .rsplit_once(':')
            .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok());
        if !address_ok {
            out.push(format!(
                "scale.tcp_address: host:port expected, got {:?}",
                self.tcp_address
            ));
        }
        if !matches!(self.price_digits, 5 | 6) {
            out.push(format!(
                "scale.price_digits: 5 or 6 expected, got {}",
                self.price_digits
            ));
        }
        if self.check_values.len() > 5 {
            out.push(format!(
                "scale.check_values: at most 5 expected, got {}",
                self.check_values.len()
            ));
        }
        let hex = |v: &str| v.len() == 4 && v.bytes().all(|b| b.is_ascii_hexdigit());
        if self
            .check_values
            .iter()
            .any(|v| !hex(&v.checksum) || !hex(&v.correction))
        {
            out.push("scale.check_values: four hex digits expected in each".to_string());
        }
        if !(3_000..=120_000).contains(&self.weigh_timeout_ms) {
            out.push(format!(
                "scale.weigh_timeout_ms: 3000 to 120000 expected, got {}",
                self.weigh_timeout_ms
            ));
        }
        out
    }
}

/// `payment_terminal`. Which fields a terminal needs depends on `kind`
/// (see [`PaymentTerminalConfig::problems`]); secrets live in files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentTerminalConfig {
    pub kind: PaymentTerminalKind,
    /// Cloud kinds: the API base URL. Absent: the vendor's production
    /// URL. Set for the vendor's test environment, a relay that holds
    /// the API key, or the local mock.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Cloud kinds: file holding the API key and nothing else
    /// (whitespace trimmed). Absent when `base_url` is a relay that
    /// authenticates the till itself.
    #[serde(default)]
    pub api_key_file: Option<String>,
    /// Worldline: the merchant id (UMID).
    #[serde(default)]
    pub merchant_id: Option<String>,
    /// Worldline: the terminal id (UTID). CCV Payment API: the terminal
    /// id assigned by CCV. OPI: the workstation id sent to the terminal.
    #[serde(default)]
    pub terminal_id: Option<String>,
    /// Worldline: the integrator id Worldline issues to the producer.
    #[serde(default)]
    pub integrator_id: Option<String>,
    /// CCV Payment API: the terminal's management system id.
    #[serde(default)]
    pub management_system_id: Option<String>,
    /// CCV Payment API: the terminal's access protocol (`OPI_NL` when
    /// absent).
    #[serde(default)]
    pub access_protocol: Option<String>,
    /// OPI: the terminal on the LAN, `host:port` (channel 0, usually
    /// port 4100).
    #[serde(default)]
    pub address: Option<String>,
    /// OPI: the till's port for the terminal's channel 1 (display and
    /// printing requests), 4102 when absent.
    #[serde(default)]
    pub device_port: Option<u16>,
    /// Longest wait for the terminal's answer to one payment, 30 to 600
    /// seconds (180 when absent). Past it the outcome is asked again.
    #[serde(default = "default_terminal_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl PaymentTerminalConfig {
    /// The API key, read from `api_key_file`; `Ok(None)` without one.
    pub fn api_key(&self) -> Result<Option<String>, String> {
        let Some(path) = &self.api_key_file else {
            return Ok(None);
        };
        match std::fs::read_to_string(path) {
            Ok(s) if !s.trim().is_empty() => Ok(Some(s.trim().to_string())),
            Ok(_) => Err(format!("payment_terminal.api_key_file: {path:?} is empty")),
            Err(e) => Err(format!(
                "payment_terminal.api_key_file: cannot read {path:?}: {e}"
            )),
        }
    }

    /// What would make the terminal unusable. Each message starts with
    /// `payment_terminal.<field>: `.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let missing = |out: &mut Vec<String>, key: &str, v: &Option<String>| {
            if v.as_deref().is_none_or(|s| s.trim().is_empty()) {
                out.push(format!(
                    "payment_terminal.{key}: required for {}",
                    self.kind.as_str()
                ));
            }
        };
        match self.kind {
            PaymentTerminalKind::WorldlineTerminalApi => {
                missing(&mut out, "merchant_id", &self.merchant_id);
                missing(&mut out, "terminal_id", &self.terminal_id);
                missing(&mut out, "integrator_id", &self.integrator_id);
            }
            PaymentTerminalKind::CcvPaymentApi => {
                missing(&mut out, "terminal_id", &self.terminal_id);
                missing(&mut out, "management_system_id", &self.management_system_id);
            }
            PaymentTerminalKind::CcvOpi => {
                missing(&mut out, "address", &self.address);
                if let Some(a) = &self.address
                    && !a.trim().is_empty()
                    && a.rsplit_once(':')
                        .is_none_or(|(h, p)| h.is_empty() || p.parse::<u16>().is_err())
                {
                    out.push(format!(
                        "payment_terminal.address: host:port expected, got {a:?}"
                    ));
                }
            }
        }
        if self.kind != PaymentTerminalKind::CcvOpi {
            if let Some(url) = &self.base_url
                && !(url.starts_with("https://") || url.starts_with("http://"))
            {
                out.push(format!(
                    "payment_terminal.base_url: http(s) URL expected, got {url:?}"
                ));
            }
            if self.api_key_file.is_none() && self.base_url.is_none() {
                out.push(format!(
                    "payment_terminal.api_key_file: required for {} without a relay base_url",
                    self.kind.as_str()
                ));
            }
            if let Err(e) = self.api_key() {
                out.push(e);
            }
        }
        if !(30..=600).contains(&self.timeout_seconds) {
            out.push(format!(
                "payment_terminal.timeout_seconds: 30 to 600 expected, got {}",
                self.timeout_seconds
            ));
        }
        out
    }
}

fn default_supplier_country() -> String {
    "BE".to_string()
}

fn default_peppol_provider() -> String {
    "logging".to_string()
}

/// Peppol delivery of B2B invoices, from `pos.json` as `peppol` (V11-D12,
/// V11-D13). `provider` is `storecove` (the Storecove API v2 shape, a real
/// access point) or `logging` (development: the document is logged, nothing is
/// sent). The API key stays in a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeppolConfig {
    /// `storecove` or `logging` (default).
    #[serde(default = "default_peppol_provider")]
    pub provider: String,
    /// Storecove API base URL, e.g. `https://api.storecove.com`. Required for
    /// `storecove`.
    #[serde(default)]
    pub base_url: Option<String>,
    /// File holding the API key and nothing else (whitespace trimmed).
    #[serde(default)]
    pub api_key_file: Option<String>,
    /// The Storecove legal entity the invoices are sent for.
    #[serde(default)]
    pub legal_entity_id: Option<i64>,
    /// The supplier's Peppol participant id: `0208:` and the 10-digit
    /// enterprise number.
    pub sender_id: String,
    pub supplier_street: String,
    pub supplier_postal_code: String,
    pub supplier_city: String,
    /// ISO 3166-1 alpha-2, `BE` when absent.
    #[serde(default = "default_supplier_country")]
    pub supplier_country: String,
}

impl PeppolConfig {
    /// The API key, read from `api_key_file`; `Ok(None)` without one.
    pub fn api_key(&self) -> Result<Option<String>, String> {
        let Some(path) = &self.api_key_file else {
            return Ok(None);
        };
        match std::fs::read_to_string(path) {
            Ok(s) if !s.trim().is_empty() => Ok(Some(s.trim().to_string())),
            Ok(_) => Err(format!("peppol.api_key_file: {path:?} is empty")),
            Err(e) => Err(format!("peppol.api_key_file: cannot read {path:?}: {e}")),
        }
    }

    /// What would make the delivery unusable. Each message starts with
    /// `peppol.<field>: `.
    ///
    /// The `logging` provider sends nothing, so a release build refuses it:
    /// it would show every B2B invoice as delivered (V11-D12).
    pub fn problems(&self) -> Vec<String> {
        self.problems_with(cfg!(debug_assertions))
    }

    fn problems_with(&self, allow_logging: bool) -> Vec<String> {
        let mut out = Vec::new();
        match self.provider.as_str() {
            "logging" if allow_logging => {}
            "logging" => out.push(
                "peppol.provider: logging sends nothing and is refused in a release build, \
                 use storecove"
                    .to_string(),
            ),
            "storecove" => {
                match &self.base_url {
                    Some(u) if u.starts_with("https://") || u.starts_with("http://") => {}
                    other => out.push(format!(
                        "peppol.base_url: http(s) URL expected for storecove, got {other:?}"
                    )),
                }
                if self.api_key_file.is_none() {
                    out.push("peppol.api_key_file: required for storecove".to_string());
                }
                if self.legal_entity_id.is_none() {
                    out.push("peppol.legal_entity_id: required for storecove".to_string());
                }
            }
            other => out.push(format!(
                "peppol.provider: storecove or logging expected, got {other:?}"
            )),
        }
        if self.api_key_file.is_some()
            && let Err(e) = self.api_key()
        {
            out.push(e);
        }
        let id_ok = self
            .sender_id
            .strip_prefix("0208:")
            .is_some_and(|n| n.len() == 10 && n.chars().all(|c| c.is_ascii_digit()) && mod97_ok(n));
        if !id_ok {
            out.push(format!(
                "peppol.sender_id: 0208: and a 10-digit enterprise number expected, got {:?}",
                self.sender_id
            ));
        }
        for (key, v) in [
            ("supplier_street", &self.supplier_street),
            ("supplier_postal_code", &self.supplier_postal_code),
            ("supplier_city", &self.supplier_city),
        ] {
            if v.trim().is_empty() {
                out.push(format!("peppol.{key}: must not be empty"));
            }
        }
        if self.supplier_country.len() != 2
            || !self
                .supplier_country
                .chars()
                .all(|c| c.is_ascii_uppercase())
        {
            out.push(format!(
                "peppol.supplier_country: two upper-case letters expected, got {:?}",
                self.supplier_country
            ));
        }
        out
    }
}

/// The Telegram Bot API the owner alerts talk to.
pub const DEFAULT_TELEGRAM_API_BASE: &str = "https://api.telegram.org";

/// The owner alerts' transport, from `pos.json` as `alerts`. The bot
/// token is a secret: it lives in a file, never in `pos.json`, never in
/// the database and never in a log. The chat id and the switches are
/// owner settings (`owner_settings`, key `alerts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AlertsConfig {
    /// File holding the bot token and nothing else (whitespace
    /// trimmed). Absent: alerts cannot be sent (`alerts_token_missing`).
    #[serde(default)]
    pub telegram_bot_token_file: Option<String>,
    /// The Bot API base URL. Absent: [`DEFAULT_TELEGRAM_API_BASE`]. Set
    /// for tests only.
    #[serde(default)]
    pub telegram_api_base: Option<String>,
}

impl AlertsConfig {
    /// The bot token, read from `telegram_bot_token_file`; `Ok(None)`
    /// without a file. The error text never contains the token.
    pub fn bot_token(&self) -> Result<Option<String>, String> {
        let Some(path) = &self.telegram_bot_token_file else {
            return Ok(None);
        };
        match std::fs::read_to_string(path) {
            Ok(s) if !s.trim().is_empty() => Ok(Some(s.trim().to_string())),
            Ok(_) => Err(format!("alerts.telegram_bot_token_file: {path:?} is empty")),
            Err(e) => Err(format!(
                "alerts.telegram_bot_token_file: cannot read {path:?}: {e}"
            )),
        }
    }

    /// The Bot API base URL without a trailing slash.
    pub fn api_base(&self) -> String {
        self.telegram_api_base
            .as_deref()
            .filter(|b| !b.trim().is_empty())
            .unwrap_or(DEFAULT_TELEGRAM_API_BASE)
            .trim_end_matches('/')
            .to_string()
    }

    /// What would make the alert transport unusable. Each message starts
    /// with `alerts.<field>: `.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Err(e) = self.bot_token() {
            out.push(e);
        }
        if let Some(base) = &self.telegram_api_base
            && let Some(p) = endpoint_problem("alerts.telegram_api_base", base)
        {
            out.push(p);
        }
        out
    }
}

/// Lowercase bech32 data alphabet (no 1, b, i, o).
const BECH32_CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// `true` when `s` looks like an age X25519 recipient: all lowercase,
/// `age1` then 58 bech32 characters (62 in all).
fn looks_like_age_recipient(s: &str) -> bool {
    s.len() == 62 && s.starts_with("age1") && s[4..].chars().all(|c| BECH32_CHARSET.contains(c))
}

/// Why `url` is not an acceptable endpoint for the setting `field`
/// (`export.url`, `alerts.telegram_api_base`), or `None`.
fn endpoint_problem(field: &str, url: &str) -> Option<String> {
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Some(format!("{field}: an https:// URL expected"));
    };
    if url.contains('?') || url.contains('#') {
        return Some(format!("{field}: no query or fragment allowed"));
    }
    let authority = rest.split('/').next().unwrap_or("");
    if authority.contains('@') {
        return Some(format!("{field}: no user info allowed"));
    }
    // host[:port], with [::1] in brackets.
    let (host, port) = if let Some(after) = authority.strip_prefix('[') {
        match after.split_once(']') {
            Some((h, tail)) => (format!("[{h}]"), tail.strip_prefix(':')),
            None => return Some(format!("{field}: malformed host")),
        }
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), Some(p)),
            None => (authority.to_string(), None),
        }
    };
    if host.is_empty() || host == "[]" {
        return Some(format!("{field}: a host is required"));
    }
    if let Some(p) = port
        && (p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return Some(format!("{field}: malformed port"));
    }
    if !secure && !["127.0.0.1", "localhost", "[::1]", "10.0.2.2"].contains(&host.as_str()) {
        return Some(format!(
            "{field}: plain http is only allowed to 127.0.0.1, localhost, [::1] or 10.0.2.2"
        ));
    }
    None
}

impl ExportConfig {
    /// What would make the export unusable. Each message starts with
    /// `export.<field>: `.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(p) = endpoint_problem("export.url", &self.url) {
            out.push(p);
        }
        if self.token_file.trim().is_empty() {
            out.push("export.token_file: must not be empty".to_string());
        }
        if !looks_like_age_recipient(&self.owner_recipient) {
            out.push("export.owner_recipient: an age public key (age1...) expected".to_string());
        }
        if let Some(r) = &self.producer_recipient
            && !looks_like_age_recipient(r)
        {
            out.push("export.producer_recipient: an age public key (age1...) expected".to_string());
        }
        if !(1..=365).contains(&self.keep_days) {
            out.push(format!(
                "export.keep_days: 1 to 365 expected, got {}",
                self.keep_days
            ));
        }
        out
    }
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

    /// A configuration with every path field set to a relative path.
    fn with_relative_paths() -> PosConfig {
        PosConfig {
            fdm_token_file: Some("secrets/fdm.token".into()),
            fdm_ca_file: Some("tls/ca.pem".into()),
            fdm_client_cert_file: Some("tls/client.pem".into()),
            fdm_client_key_file: Some("tls/client.key".into()),
            backup_dir: Some("backups".into()),
            hubrise: Some(HubriseConfig {
                client_id: "123.clients.hubrise.com".into(),
                client_secret_file: "secrets/hubrise.secret".into(),
                oauth_base_url: default_hubrise_oauth_base_url(),
                api_base_url: default_hubrise_api_base_url(),
                redirect_uri: default_hubrise_redirect_uri(),
                scope: default_hubrise_scope(),
            }),
            digital_ticket: Some(DigitalTicketConfig {
                region: "fr-par".into(),
                project_id: "p".into(),
                from_email: "tickets@example.com".into(),
                from_name: "Restaurant".into(),
                api_base: Some("https://api.example.com".into()),
                api_key_file: "secrets/tem.key".into(),
                allow_digital_only: false,
            }),
            ..valid()
        }
    }

    #[test]
    fn resolve_relative_to_joins_every_relative_path_field() {
        let base = Path::new("/bundle/one");
        let mut c = with_relative_paths();
        c.resolve_relative_to(base);
        let join = |p: &str| base.join(p).to_string_lossy().into_owned();
        assert_eq!(c.fdm_token_file, Some(join("secrets/fdm.token")));
        assert_eq!(c.fdm_ca_file, Some(join("tls/ca.pem")));
        assert_eq!(c.fdm_client_cert_file, Some(join("tls/client.pem")));
        assert_eq!(c.fdm_client_key_file, Some(join("tls/client.key")));
        assert_eq!(c.backup_dir, Some(join("backups")));
        assert_eq!(
            c.hubrise.as_ref().unwrap().client_secret_file,
            join("secrets/hubrise.secret")
        );
        assert_eq!(
            c.digital_ticket.as_ref().unwrap().api_key_file,
            join("secrets/tem.key")
        );
    }

    #[test]
    fn resolve_relative_to_leaves_absolute_paths_and_other_fields_alone() {
        let mut c = with_relative_paths();
        c.fdm_token_file = Some("/abs/fdm.token".into());
        c.fdm_ca_file = Some("/abs/ca.pem".into());
        c.fdm_client_cert_file = Some("/abs/client.pem".into());
        c.fdm_client_key_file = Some("/abs/client.key".into());
        c.backup_dir = Some("/abs/backups".into());
        c.hubrise.as_mut().unwrap().client_secret_file = "/abs/hubrise.secret".into();
        c.digital_ticket.as_mut().unwrap().api_key_file = "/abs/tem.key".into();
        c.printer_address = Some("192.168.1.200:9100".into());
        c.kitchen_printer_address = Some("kitchen.local:9100".into());
        let before = serde_json::to_value(&c).unwrap();
        c.resolve_relative_to(Path::new("/bundle"));
        assert_eq!(serde_json::to_value(&c).unwrap(), before);

        // Relative paths move, nothing else does: URLs, addresses, names.
        let mut r = with_relative_paths();
        r.printer_address = Some("192.168.1.200:9100".into());
        let mut expected = serde_json::to_value(&r).unwrap();
        r.resolve_relative_to(Path::new("/bundle"));
        let after = serde_json::to_value(&r).unwrap();
        for key in [
            "fdm_token_file",
            "fdm_ca_file",
            "fdm_client_cert_file",
            "fdm_client_key_file",
            "backup_dir",
        ] {
            assert_ne!(after[key], expected[key], "{key} should move");
            expected[key] = after[key].clone();
        }
        expected["hubrise"]["client_secret_file"] = after["hubrise"]["client_secret_file"].clone();
        expected["digital_ticket"]["api_key_file"] =
            after["digital_ticket"]["api_key_file"].clone();
        assert_eq!(after, expected);
        assert_eq!(r.fdm_url, "http://192.168.1.20:8081/graphql");
        assert_eq!(
            r.digital_ticket.as_ref().unwrap().api_base.as_deref(),
            Some("https://api.example.com")
        );
    }

    #[test]
    fn resolve_relative_to_ignores_absent_and_empty_values() {
        let mut c = valid();
        c.resolve_relative_to(Path::new("/bundle"));
        assert!(c.fdm_token_file.is_none() && c.backup_dir.is_none());
        assert!(c.hubrise.is_none() && c.digital_ticket.is_none());

        c.backup_dir = Some(String::new());
        c.resolve_relative_to(Path::new("/bundle"));
        assert_eq!(c.backup_dir.as_deref(), Some(""));
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

    #[test]
    fn every_problem_text_has_a_known_code() {
        for (text, code) in [
            ("tables: ranges such as \"1-12\" expected: x", "tables"),
            ("pos_id: 14 upper-case letters", "pos_id"),
            ("restaurant_vat: BE followed by 10 digits", "restaurant_vat"),
            ("restaurant_est_no: 10 digits", "restaurant_est_no"),
            ("terminal_id: 1 to 600 characters expected", "identity"),
            ("device_id: 1 to 600 characters expected", "identity"),
            ("restaurant_name: printed on every ticket", "ticket_header"),
            (
                "restaurant_address: printed on every ticket",
                "ticket_header",
            ),
            ("fdm_url: http(s) URL expected", "fdm_url"),
            (
                "fdm_url is https: set fdm_ca_file to the FDM's CA",
                "fdm_files",
            ),
            ("language: one of fr", "language"),
            ("fdm_token_file: \"x\" is empty", "fdm_files"),
            ("fdm_ca_file: cannot read", "fdm_files"),
            (
                "fdm_client_cert_file and fdm_client_key_file go together",
                "fdm_files",
            ),
            ("FDM HTTPS: bad CA", "fdm_files"),
            ("day_end_hour: 0 to 12 expected", "day_end_hour"),
            ("hubrise.client_id: must not be empty", "hubrise"),
            (
                "digital_ticket.project_id: must not be empty",
                "digital_ticket",
            ),
            ("export.keep_days: 1 to 365", "export"),
            ("alerts.telegram_bot_token_file: cannot read", "alerts"),
            (
                "payment_terminal.address: host:port expected",
                "payment_terminal",
            ),
            ("no configuration: the development identity", "missing"),
            ("expected value at line 1", "other"),
        ] {
            assert_eq!(problem_code(text), code, "{text}");
            assert!(PROBLEM_CODES.contains(&code), "{code}");
        }
    }

    #[test]
    fn the_problems_of_a_very_bad_config_all_have_a_code() {
        let bad = PosConfig {
            pos_id: "x".into(),
            restaurant_vat: "x".into(),
            restaurant_est_no: "x".into(),
            terminal_id: String::new(),
            restaurant_name: String::new(),
            fdm_url: "ftp://x".into(),
            language: "xx".into(),
            day_end_hour: 20,
            ..valid()
        };
        let problems = bad.problems();
        assert!(problems.len() >= 8, "{problems:?}");
        for p in &problems {
            assert_ne!(problem_code(p), "other", "{p}");
        }
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
        assert_eq!(cfg.printer_paper_mm, PaperWidth::Mm80);
        assert_eq!(cfg.kitchen_printer_paper_mm, PaperWidth::Mm80);
    }

    fn with_printers(extra: &str) -> Result<PosConfig, serde_json::Error> {
        serde_json::from_str(&format!(
            r#"{{
                "pos_id": "CFOD0061234567",
                "terminal_id": "POS1",
                "device_id": "DEV001",
                "restaurant_name": "Restaurant",
                "restaurant_address": "Rue Test 1, 1000 Bruxelles",
                "restaurant_vat": "BE0499999960",
                "restaurant_est_no": "2123456791",
                "fdm_url": "http://192.168.1.20:8080/graphql",
                "printer_address": "192.168.1.30:9100",
                "kitchen_printer_address": "192.168.1.31:9100"
                {extra}
            }}"#
        ))
    }

    #[test]
    fn paper_widths_parse_as_millimetres() {
        let cfg =
            with_printers(r#", "printer_paper_mm": 58, "kitchen_printer_paper_mm": 80"#).unwrap();
        assert_eq!(cfg.printer_paper_mm, PaperWidth::Mm58);
        assert_eq!(cfg.kitchen_printer_paper_mm, PaperWidth::Mm80);
        let back = serde_json::to_value(&cfg).unwrap();
        assert_eq!(back["printer_paper_mm"], 58);
        assert_eq!(back["kitchen_printer_paper_mm"], 80);
    }

    #[test]
    fn any_other_paper_width_refuses_the_configuration() {
        for bad in [
            r#", "printer_paper_mm": 57"#,
            r#", "kitchen_printer_paper_mm": "58""#,
        ] {
            assert!(with_printers(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn paper_for_follows_the_printer_address() {
        let cfg = with_printers(r#", "printer_paper_mm": 58"#).unwrap();
        assert_eq!(cfg.paper_for("192.168.1.30:9100"), PaperWidth::Mm58);
        assert_eq!(cfg.paper_for("192.168.1.31:9100"), PaperWidth::Mm80);
        assert_eq!(cfg.paper_for("10.0.0.1:9100"), PaperWidth::Mm58);

        // One printer named twice: the ticket printer's roll.
        let mut same = cfg.clone();
        same.kitchen_printer_address = same.printer_address.clone();
        same.kitchen_printer_paper_mm = PaperWidth::Mm80;
        assert_eq!(same.paper_for("192.168.1.30:9100"), PaperWidth::Mm58);
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

    const RECIPIENT: &str = "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p";

    fn export_cfg() -> ExportConfig {
        ExportConfig {
            url: "https://export.example.com/v1/inst-42".into(),
            token_file: "secrets/export.token".into(),
            owner_recipient: RECIPIENT.into(),
            producer_recipient: None,
            keep_days: 14,
        }
    }

    fn export_problems(e: ExportConfig) -> Vec<String> {
        PosConfig {
            export: Some(e),
            ..valid()
        }
        .problems()
    }

    #[test]
    fn a_valid_export_section_has_no_problem() {
        assert!(export_problems(export_cfg()).is_empty());
        let mut e = export_cfg();
        e.producer_recipient = Some(RECIPIENT.into());
        e.keep_days = 365;
        assert!(export_problems(e).is_empty());
    }

    #[test]
    fn an_absent_export_section_is_off_and_no_problem() {
        let c = valid();
        assert!(c.export.is_none());
        assert!(c.problems().is_empty());
        let json = r#"{"pos_id":"CFOD0061234567","terminal_id":"POS1","device_id":"d",
            "restaurant_name":"R","restaurant_address":"A","restaurant_vat":"BE0499999960",
            "restaurant_est_no":"8789456149","fdm_url":"http://h/graphql"}"#;
        assert!(
            serde_json::from_str::<PosConfig>(json)
                .unwrap()
                .export
                .is_none()
        );
    }

    #[test]
    fn export_url_rules() {
        for ok in [
            "https://export.example.com",
            "https://export.example.com:8443/p/a",
            "http://127.0.0.1:9000/x",
            "http://localhost/x",
            "http://[::1]:9000/x",
            "http://10.0.2.2:9300",
        ] {
            let mut e = export_cfg();
            e.url = ok.into();
            assert!(export_problems(e).is_empty(), "{ok} must be accepted");
        }
        for bad in [
            "",
            "ftp://export.example.com",
            "export.example.com",
            "https://",
            "https:///path",
            "https://:443/x",
            "http://export.example.com/x",
            "http://192.168.1.5/x",
            "https://user:pw@export.example.com",
            "https://export.example.com/x?a=b",
            "https://export.example.com/x#frag",
            "https://export.example.com:port/x",
            "http://[::1/x",
        ] {
            let mut e = export_cfg();
            e.url = bad.into();
            let p = export_problems(e);
            assert_eq!(p.len(), 1, "{bad} must be rejected: {p:?}");
            assert!(p[0].starts_with("export.url: "), "{p:?}");
        }
    }

    #[test]
    fn export_token_file_must_not_be_empty() {
        let mut e = export_cfg();
        e.token_file = "  ".into();
        let p = export_problems(e);
        assert_eq!(p.len(), 1);
        assert!(p[0].starts_with("export.token_file: "));
    }

    #[test]
    fn export_recipients_must_look_like_age_keys() {
        let upper = RECIPIENT.to_uppercase();
        let short = &RECIPIENT[..61];
        let long = format!("{RECIPIENT}q");
        let wrong_prefix = format!("agf1{}", &RECIPIENT[4..]);
        let bad_char = format!("{}b", &RECIPIENT[..61]);
        let secret = "AGE-SECRET-KEY-1QQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ";
        for bad in [
            "",
            upper.as_str(),
            short,
            long.as_str(),
            wrong_prefix.as_str(),
            bad_char.as_str(),
            secret,
        ] {
            let mut e = export_cfg();
            e.owner_recipient = bad.into();
            let p = export_problems(e);
            assert_eq!(p.len(), 1, "owner {bad:?}: {p:?}");
            assert!(p[0].starts_with("export.owner_recipient: "));

            let mut e = export_cfg();
            e.producer_recipient = Some(bad.into());
            let p = export_problems(e);
            assert_eq!(p.len(), 1, "producer {bad:?}: {p:?}");
            assert!(p[0].starts_with("export.producer_recipient: "));
        }
    }

    #[test]
    fn export_keep_days_is_bounded() {
        for (days, ok) in [(0, false), (1, true), (14, true), (365, true), (366, false)] {
            let mut e = export_cfg();
            e.keep_days = days;
            let p = export_problems(e);
            assert_eq!(p.is_empty(), ok, "keep_days {days}: {p:?}");
            if !ok {
                assert!(p[0].starts_with("export.keep_days: "));
            }
        }
    }

    #[test]
    fn export_keep_days_defaults_to_14() {
        let e: ExportConfig = serde_json::from_str(&format!(
            r#"{{"url":"https://x.example.com","token_file":"t","owner_recipient":"{RECIPIENT}"}}"#
        ))
        .unwrap();
        assert_eq!(e.keep_days, 14);
        assert!(e.producer_recipient.is_none());
    }

    #[test]
    fn export_token_file_is_resolved_relative_to_the_base() {
        let mut c = PosConfig {
            export: Some(export_cfg()),
            ..valid()
        };
        c.resolve_relative_to(Path::new("/etc/pili"));
        assert_eq!(
            c.export.as_ref().unwrap().token_file,
            "/etc/pili/secrets/export.token"
        );
        let mut e = export_cfg();
        e.token_file = "/abs/export.token".into();
        let mut c = PosConfig {
            export: Some(e),
            ..valid()
        };
        c.resolve_relative_to(Path::new("/etc/pili"));
        assert_eq!(c.export.as_ref().unwrap().token_file, "/abs/export.token");
    }

    #[test]
    fn a_full_export_section_survives_a_json_round_trip() {
        let mut e = export_cfg();
        e.producer_recipient = Some(RECIPIENT.into());
        e.keep_days = 30;
        let c = PosConfig {
            export: Some(e.clone()),
            ..valid()
        };
        let back: PosConfig = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.export, Some(e));
    }

    fn peppol() -> PeppolConfig {
        PeppolConfig {
            provider: "logging".into(),
            base_url: None,
            api_key_file: None,
            legal_entity_id: None,
            sender_id: "0208:0499999960".into(),
            supplier_street: "Rue de l'Exemple 1".into(),
            supplier_postal_code: "4000".into(),
            supplier_city: "Liège".into(),
            supplier_country: "BE".into(),
        }
    }

    #[test]
    fn the_logging_provider_is_refused_in_a_release_build() {
        let mut p = PeppolConfig {
            provider: "logging".into(),
            base_url: None,
            api_key_file: None,
            legal_entity_id: None,
            sender_id: "0208:0308357159".into(),
            supplier_street: "Rue 1".into(),
            supplier_postal_code: "4000".into(),
            supplier_city: "Liege".into(),
            supplier_country: "BE".into(),
        };
        let release = p.problems_with(false);
        assert!(
            release.iter().any(|m| m.starts_with("peppol.provider:")),
            "{release:?}"
        );
        assert!(
            p.problems_with(true)
                .iter()
                .all(|m| !m.starts_with("peppol.provider:"))
        );
        p.provider = "nope".into();
        assert!(!p.problems_with(true).is_empty());
    }

    #[test]
    fn a_peppol_section_defaults_to_the_logging_provider_and_belgium() {
        let json = r#"{"sender_id":"0208:0499999960","supplier_street":"a","supplier_postal_code":"4000","supplier_city":"Liège"}"#;
        let p: PeppolConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            (p.provider.as_str(), p.supplier_country.as_str()),
            ("logging", "BE")
        );
        assert!(p.problems().is_empty(), "{:?}", p.problems());
    }

    #[test]
    fn scale_problems_name_their_setting() {
        let scale = ScaleConfig {
            tcp_address: "192.168.1.60:4001".into(),
            price_digits: 6,
            check_values: vec![ScaleCheckValue {
                checksum: "4711".into(),
                correction: "F336".into(),
            }],
            weigh_timeout_ms: 15_000,
        };
        assert_eq!(scale.problems(), Vec::<String>::new());

        let bad = ScaleConfig {
            tcp_address: "nowhere".into(),
            price_digits: 4,
            check_values: vec![ScaleCheckValue {
                checksum: "47".into(),
                correction: "XYZW".into(),
            }],
            weigh_timeout_ms: 0,
        };
        let problems = bad.problems();
        assert!(problems.len() >= 3, "{problems:?}");
        for problem in &problems {
            assert!(problem.starts_with("scale."), "{problem}");
            assert_eq!(problem_code(problem), "scale", "{problem}");
        }

        let mut config = valid();
        config.scale = Some(bad);
        assert!(config.problems().iter().any(|p| p.starts_with("scale.")));
    }

    #[test]
    fn peppol_problems_name_their_setting() {
        let mut p = peppol();
        p.sender_id = "0208:BE0499999960".into();
        p.supplier_city = " ".into();
        p.supplier_country = "be".into();
        p.provider = "other".into();
        let problems = p.problems();
        for key in [
            "peppol.sender_id",
            "peppol.supplier_city",
            "peppol.supplier_country",
            "peppol.provider",
        ] {
            assert!(
                problems.iter().any(|t| t.starts_with(key)),
                "{key}: {problems:?}"
            );
        }
        // A sender id with a wrong check digit pair is refused.
        let mut p = peppol();
        p.sender_id = "0208:0499999961".into();
        assert!(
            p.problems()
                .iter()
                .any(|t| t.starts_with("peppol.sender_id"))
        );
    }

    #[test]
    fn storecove_needs_its_url_key_and_legal_entity() {
        let mut p = peppol();
        p.provider = "storecove".into();
        let problems = p.problems();
        for key in [
            "peppol.base_url",
            "peppol.api_key_file",
            "peppol.legal_entity_id",
        ] {
            assert!(
                problems.iter().any(|t| t.starts_with(key)),
                "{key}: {problems:?}"
            );
        }
        let dir = std::env::temp_dir().join(format!("pos-core-peppol-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let key_file = dir.join("key");
        std::fs::write(&key_file, "  secret \n").unwrap();
        p.base_url = Some("https://api.storecove.com".into());
        p.api_key_file = Some(key_file.to_string_lossy().into_owned());
        p.legal_entity_id = Some(7);
        assert!(p.problems().is_empty(), "{:?}", p.problems());
        assert_eq!(p.api_key().unwrap().as_deref(), Some("secret"));
        std::fs::write(&key_file, "\n").unwrap();
        assert!(p.problems().iter().any(|t| t.contains("is empty")));
        p.api_key_file = Some(dir.join("missing").to_string_lossy().into_owned());
        assert!(p.problems().iter().any(|t| t.contains("cannot read")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_peppol_key_file_resolves_against_the_bundle_and_has_a_problem_code() {
        let mut c = valid();
        let mut p = peppol();
        p.api_key_file = Some("peppol.key".into());
        c.peppol = Some(p);
        c.resolve_relative_to(Path::new("/bundle"));
        assert_eq!(
            c.peppol.as_ref().unwrap().api_key_file.as_deref(),
            Some("/bundle/peppol.key")
        );
        assert_eq!(problem_code("peppol.sender_id: x"), "peppol");
        assert!(PROBLEM_CODES.contains(&"peppol"));
    }

    #[test]
    fn the_alerts_section_is_optional_and_resolves_its_token_file() {
        let mut c = valid();
        assert!(c.alerts.is_none());
        c.alerts = Some(AlertsConfig {
            telegram_bot_token_file: Some("secrets/tg.token".into()),
            telegram_api_base: Some("http://127.0.0.1:1".into()),
        });
        c.resolve_relative_to(Path::new("/bundle"));
        let a = c.alerts.unwrap();
        assert_eq!(
            a.telegram_bot_token_file.as_deref(),
            Some("/bundle/secrets/tg.token")
        );
        // A URL is not a path.
        assert_eq!(a.telegram_api_base.as_deref(), Some("http://127.0.0.1:1"));
    }

    #[test]
    fn the_alerts_section_is_checked_and_never_names_the_token() {
        let dir = std::env::temp_dir().join(format!("pos-core-alerts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("good.token");
        std::fs::write(&good, "123:SECRET\n").unwrap();
        let empty = dir.join("empty.token");
        std::fs::write(&empty, " \n").unwrap();

        let mut a = AlertsConfig {
            telegram_bot_token_file: Some(good.to_string_lossy().into_owned()),
            telegram_api_base: None,
        };
        assert!(a.problems().is_empty());
        assert_eq!(a.bot_token().unwrap().as_deref(), Some("123:SECRET"));
        assert_eq!(a.api_base(), "https://api.telegram.org");

        a.telegram_bot_token_file = Some(empty.to_string_lossy().into_owned());
        assert!(a.problems()[0].starts_with("alerts.telegram_bot_token_file: "));
        a.telegram_bot_token_file = Some(dir.join("absent").to_string_lossy().into_owned());
        assert!(a.problems()[0].contains("cannot read"));
        a.telegram_bot_token_file = None;
        assert_eq!(a.bot_token().unwrap(), None);

        a.telegram_api_base = Some("http://example.com/".into());
        assert!(a.problems()[0].starts_with("alerts.telegram_api_base: plain http"));
        a.telegram_api_base = Some("http://127.0.0.1:8080/".into());
        assert!(a.problems().is_empty());
        assert_eq!(a.api_base(), "http://127.0.0.1:8080");

        let mut c = valid();
        c.alerts = Some(AlertsConfig {
            telegram_bot_token_file: Some(empty.to_string_lossy().into_owned()),
            telegram_api_base: None,
        });
        let problems = c.problems();
        assert_eq!(problems.len(), 1);
        assert_eq!(problem_code(&problems[0]), "alerts");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
