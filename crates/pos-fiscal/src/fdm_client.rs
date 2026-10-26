use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use crate::http_tls::HttpTls;
use crate::inputs::*;
use crate::response::*;
use crate::response_check::{ExpectedEvent, PlaceholderSignatures, check_sign_result};

/// Parse one sign result out of the GraphQL `data` object and check it
/// against the request it answers before anyone can archive it.
fn parse_checked<D: Serialize>(
    sink: Option<&SignMessageSink>,
    placeholders: PlaceholderSignatures,
    data: &serde_json::Value,
    mutation_name: &str,
    request: &D,
    is_training: bool,
) -> Result<SignResult, FdmError> {
    let value = data
        .get(mutation_name)
        .ok_or_else(|| FdmError::BadResponse(format!("Missing {mutation_name} in response")))?;
    let result: SignResult = serde_json::from_value(value.clone())
        .map_err(|e| FdmError::BadResponse(format!("Failed to parse {mutation_name}: {e}")))?;
    let expected = ExpectedEvent::for_mutation(mutation_name, is_training)
        .ok_or_else(|| FdmError::Serialization(format!("Unknown sign mutation {mutation_name}")))?;
    let request = serde_json::to_value(request)
        .map_err(|e| FdmError::Serialization(format!("{mutation_name}: {e}")))?;
    check_sign_result(&request, expected, &result, placeholders).map_err(|mismatch| {
        tracing::error!(
            mutation = mutation_name,
            %mismatch,
            response = %value,
            "FDM response does not match its request; event not archived"
        );
        FdmError::ResponseMismatch(mismatch.to_string())
    })?;
    report_sign_messages(sink, mutation_name, &request, &result);
    Ok(result)
}

/// The messages the FDM attached to a signed event, minus the ones it
/// marks `showPos: NEVER`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignMessages {
    pub mutation: String,
    /// The request's `bookingDate` (`reportBookingDate` for a report).
    pub booking_date: Option<String>,
    pub warnings: Vec<MessageItem>,
    pub informations: Vec<MessageItem>,
}

/// Where one client reports the messages of its signed events.
pub type SignMessageSink = Arc<dyn Fn(&SignMessages) + Send + Sync>;

fn report_sign_messages(
    sink: Option<&SignMessageSink>,
    mutation_name: &str,
    request: &serde_json::Value,
    result: &SignResult,
) {
    let Some(sink) = sink else {
        return;
    };
    let shown = |items: &Option<Vec<MessageItem>>| -> Vec<MessageItem> {
        items
            .iter()
            .flatten()
            .filter(|m| m.extensions.show_pos != crate::types::ShowPos::Never)
            .cloned()
            .collect()
    };
    let messages = SignMessages {
        mutation: mutation_name.to_string(),
        booking_date: ["bookingDate", "reportBookingDate"]
            .iter()
            .find_map(|key| request.get(key).and_then(|v| v.as_str()))
            .map(str::to_string),
        warnings: shown(&result.warnings),
        informations: shown(&result.informations),
    };
    if !messages.warnings.is_empty() || !messages.informations.is_empty() {
        sink(&messages);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FdmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// A status query the FDM did not answer.
    #[error("FDM status query failed: {0}")]
    GraphQl(String),
    /// The FDM answered a sign request with `errors[]` ("CODE: message").
    #[error("FDM refused: {message}")]
    Refused { codes: Vec<String>, message: String },
    /// The FDM answered, but not with a usable result.
    #[error("FDM unusable response: {0}")]
    BadResponse(String),
    /// The request could not be built; nothing was sent.
    #[error("request not sent: {0}")]
    Serialization(String),
    #[error("FDM not connected")]
    NotConnected,
    /// The secure connection failed on a certificate: the FDM's is not
    /// signed by `fdm_ca_file`, or the FDM wants the client certificate.
    #[error("FDM_CERTIFICATE_REFUSED: {0}")]
    CertificateRefused(String),
    #[error("FDM not operational: {0}")]
    NotOperational(String),
    /// The FDM answered, but its response does not match the request
    /// (see [`crate::response_check`]). Nothing may be archived.
    #[error("FDM_RESPONSE_MISMATCH: {0}")]
    ResponseMismatch(String),
}

/// What a failed sign request says about the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignOutcome {
    /// Never reached the FDM.
    NotSent,
    /// The FDM refused it: nothing was signed.
    Refused,
    /// The FDM may have signed it: only the same request, sent again,
    /// tells (POS description v2.2, duplicate requests).
    Unknown,
}

/// FDM error codes that mean the event was not signed.
const NOT_SIGNED_CODES: &[&str] = &[
    "BUFFER_FULL",
    "FDM_LOCKED",
    "UNAUTHORIZED",
    "INVALID_REQUEST",
    "FDM_NOT_OPERATIONAL",
    "UNKNOWN_POS",
    "CLIENT_CERT_EXPIRED",
];

impl FdmError {
    /// Whether the event may have been signed. Anything not known to be
    /// unsigned is [`SignOutcome::Unknown`]: sending the same request
    /// again is always safe, losing a signed event is not.
    pub fn outcome(&self) -> SignOutcome {
        match self {
            Self::NotConnected | Self::CertificateRefused(_) | Self::Serialization(_) => {
                SignOutcome::NotSent
            }
            Self::Refused { codes, .. }
                if !codes.is_empty()
                    && codes.iter().all(|c| NOT_SIGNED_CODES.contains(&c.as_str())) =>
            {
                SignOutcome::Refused
            }
            _ => SignOutcome::Unknown,
        }
    }

    /// The FDM's own word for the failure, to show the operator what it
    /// said: the first code of its refusal that does not mean "not
    /// signed" (the one that makes the outcome unknown), else the first
    /// code, or `FDM_RESPONSE_MISMATCH` for an answer that does not echo
    /// the request.
    pub fn fdm_code(&self) -> Option<String> {
        match self {
            Self::Refused { codes, .. } => codes
                .iter()
                .find(|c| !NOT_SIGNED_CODES.contains(&c.as_str()))
                .or_else(|| codes.first())
                .cloned(),
            Self::ResponseMismatch(_) => Some("FDM_RESPONSE_MISMATCH".to_string()),
            _ => None,
        }
    }
}

/// A sign mutation; the Z pair of M184 is one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignMutation {
    Sale,
    Order,
    PreBill,
    WorkIn,
    WorkOut,
    Copy,
    MoneyInOut,
    DrawerOpen,
    PaymentCorrection,
    Invoice,
    CostCenterChange,
    ReportTurnoverX,
    ReportTurnoverZ,
    ReportUserX,
    ReportUserZ,
    ReportTurnoverZUserZ,
}

impl SignMutation {
    pub const ALL: [SignMutation; 16] = [
        Self::Sale,
        Self::Order,
        Self::PreBill,
        Self::WorkIn,
        Self::WorkOut,
        Self::Copy,
        Self::MoneyInOut,
        Self::DrawerOpen,
        Self::PaymentCorrection,
        Self::Invoice,
        Self::CostCenterChange,
        Self::ReportTurnoverX,
        Self::ReportTurnoverZ,
        Self::ReportUserX,
        Self::ReportUserZ,
        Self::ReportTurnoverZUserZ,
    ];

    /// Stored name.
    pub fn name(self) -> &'static str {
        match self {
            Self::ReportTurnoverZUserZ => "signReportTurnoverZUserZ",
            single => single.fields()[0].0,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    /// `(field, input type, variable)` of each event it signs.
    fn fields(self) -> &'static [(&'static str, &'static str, &'static str)] {
        match self {
            Self::Sale => &[("signSale", "SaleInput", "data")],
            Self::Order => &[("signOrder", "OrderInput", "data")],
            Self::PreBill => &[("signPreBill", "PreBillInput", "data")],
            Self::WorkIn => &[("signWorkIn", "WorkInOutInput", "data")],
            Self::WorkOut => &[("signWorkOut", "WorkInOutInput", "data")],
            Self::Copy => &[("signCopy", "CopyInput", "data")],
            Self::MoneyInOut => &[("signMoneyInOut", "MoneyInOutInput", "data")],
            Self::DrawerOpen => &[("signDrawerOpen", "DrawerOpenInput", "data")],
            Self::PaymentCorrection => {
                &[("signPaymentCorrection", "PaymentCorrectionInput", "data")]
            }
            Self::Invoice => &[("signInvoice", "InvoiceInput", "data")],
            Self::CostCenterChange => &[("signCostCenterChange", "CostCenterChangeInput", "data")],
            Self::ReportTurnoverX => &[("signReportTurnoverX", "ReportTurnoverXInput", "data")],
            Self::ReportTurnoverZ => &[("signReportTurnoverZ", "ReportTurnoverZInput", "data")],
            Self::ReportUserX => &[("signReportUserX", "ReportUserXInput", "data")],
            Self::ReportUserZ => &[("signReportUserZ", "ReportUserZInput", "data")],
            Self::ReportTurnoverZUserZ => &[
                ("signReportTurnoverZ", "ReportTurnoverZInput", "turnover"),
                ("signReportUserZ", "ReportUserZInput", "user"),
            ],
        }
    }

    fn query(self) -> String {
        let sel = FdmClient::SIGN_RESULT_SELECTION;
        let vars: Vec<String> = self
            .fields()
            .iter()
            .map(|(_, input, var)| format!("${var}: {input}!"))
            .collect();
        let calls: Vec<String> = self
            .fields()
            .iter()
            .map(|(field, _, var)| format!("{field}(data: ${var}, isTraining: $isTraining) {sel}"))
            .collect();
        format!(
            "mutation({}, $isTraining: Boolean!) {{ {} }}",
            vars.join(", "),
            calls.join(" ")
        )
    }
}

/// A sign request as sent on the wire. Kept byte for byte, so a request
/// whose answer was lost is sent again identically and the FDM returns
/// its stored answer instead of signing twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignRequest {
    mutation: SignMutation,
    is_training: bool,
    body: String,
}

impl SignRequest {
    /// A single-event request.
    pub fn new<D: Serialize>(
        mutation: SignMutation,
        data: &D,
        is_training: bool,
    ) -> Result<Self, FdmError> {
        Self::build(mutation, &[data as &dyn erased::Input], is_training)
    }

    /// The end-of-day Z pair (M184).
    pub fn turnover_z_user_z<T: Serialize, U: Serialize>(
        turnover: &T,
        user: &U,
        is_training: bool,
    ) -> Result<Self, FdmError> {
        Self::build(
            SignMutation::ReportTurnoverZUserZ,
            &[turnover as &dyn erased::Input, user],
            is_training,
        )
    }

    fn build(
        mutation: SignMutation,
        inputs: &[&dyn erased::Input],
        is_training: bool,
    ) -> Result<Self, FdmError> {
        let fields = mutation.fields();
        if fields.len() != inputs.len() {
            return Err(FdmError::Serialization(format!(
                "{} takes {} input(s)",
                mutation.name(),
                fields.len()
            )));
        }
        let mut variables = serde_json::Map::new();
        for ((_, _, var), input) in fields.iter().zip(inputs) {
            variables.insert((*var).to_string(), input.to_json()?);
        }
        variables.insert("isTraining".into(), is_training.into());
        let body = crate::canonical::to_canonical(&GqlRequest {
            query: mutation.query(),
            variables: serde_json::Value::Object(variables),
        })
        .map_err(|e| FdmError::Serialization(format!("canonical serialization: {e}")))?;
        Ok(Self {
            mutation,
            is_training,
            body,
        })
    }

    /// A request stored before it was sent, checked to be one.
    pub fn from_stored(mutation: &str, is_training: bool, body: String) -> Result<Self, FdmError> {
        let mutation = SignMutation::from_name(mutation)
            .ok_or_else(|| FdmError::Serialization(format!("unknown mutation {mutation}")))?;
        let request = Self {
            mutation,
            is_training,
            body,
        };
        let parsed: serde_json::Value = serde_json::from_str(&request.body)
            .map_err(|e| FdmError::Serialization(format!("stored request: {e}")))?;
        if parsed["query"].as_str() != Some(mutation.query().as_str())
            || parsed["variables"]["isTraining"].as_bool() != Some(is_training)
        {
            return Err(FdmError::Serialization(
                "stored request does not match its mutation".into(),
            ));
        }
        request.inputs()?;
        Ok(request)
    }

    pub fn mutation(&self) -> SignMutation {
        self.mutation
    }

    pub fn is_training(&self) -> bool {
        self.is_training
    }

    /// The exact bytes sent.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// The input of each event, in order.
    pub fn inputs(&self) -> Result<Vec<serde_json::Value>, FdmError> {
        let parsed: serde_json::Value = serde_json::from_str(&self.body)
            .map_err(|e| FdmError::Serialization(format!("request: {e}")))?;
        self.mutation
            .fields()
            .iter()
            .map(|(_, _, var)| match &parsed["variables"][*var] {
                serde_json::Value::Null => {
                    Err(FdmError::Serialization(format!("request without ${var}")))
                }
                input => Ok(input.clone()),
            })
            .collect()
    }
}

mod erased {
    /// A sign input, whatever its type.
    pub trait Input {
        fn to_json(&self) -> Result<serde_json::Value, super::FdmError>;
    }

    impl<T: serde::Serialize> Input for T {
        fn to_json(&self) -> Result<serde_json::Value, super::FdmError> {
            serde_json::to_value(self).map_err(|e| super::FdmError::Serialization(e.to_string()))
        }
    }
}

/// Client for communicating with the FDM over GraphQL/HTTP.
pub struct FdmClient {
    http: reqwest::Client,
    url: String,
    /// Sent as `Authorization: Bearer ...` when the FDM requires it (a
    /// cloud simulator); never logged.
    bearer_token: Option<String>,
    /// Where the messages of signed events go ([`FdmClient::with_sign_messages`]).
    sign_messages: Option<SignMessageSink>,
    /// Whether this endpoint may answer with the simulator's placeholder
    /// signature (decided from the URL).
    placeholders: PlaceholderSignatures,
}

#[derive(Serialize)]
struct GqlRequest<V: Serialize> {
    query: String,
    variables: V,
}

impl FdmClient {
    pub fn new(fdm_url: &str) -> Self {
        Self::with_timeouts(fdm_url, Duration::from_secs(5), Duration::from_secs(10))
    }

    /// Same as [`FdmClient::new`] but with explicit connect/total
    /// timeouts. Production code always goes through `new` (the SCE
    /// 2.0-appropriate 5s/10s defaults); this exists so tests can
    /// exercise a real timeout in milliseconds instead of waiting out
    /// the full 10s production budget.
    pub fn with_timeouts(fdm_url: &str, connect_timeout: Duration, timeout: Duration) -> Self {
        Self::build(fdm_url, connect_timeout, timeout, None, &HttpTls::Platform)
    }

    /// `tls` is the preconfigured mTLS setup of the FDM's own files
    /// ([`crate::fdm_tls`]); without it the client trusts what
    /// `http_tls` says.
    ///
    /// # Panics
    ///
    /// When `http_tls` cannot make a client (no roots, an invalid
    /// certificate). The engine checks its `HttpTls` when it opens, so
    /// that never happens past that point.
    fn build(
        fdm_url: &str,
        connect_timeout: Duration,
        timeout: Duration,
        tls: Option<rustls::ClientConfig>,
        http_tls: &HttpTls,
    ) -> Self {
        let http = match tls {
            // The FDM's own CA and client certificate: unchanged.
            Some(tls) => reqwest::Client::builder().tls_backend_preconfigured(tls),
            None => http_tls.client_builder().expect("valid HttpTls"),
        };
        let http = http.connect_timeout(connect_timeout).timeout(timeout);
        Self {
            http: http.build().expect("reqwest client builder"),
            url: fdm_url.to_string(),
            bearer_token: None,
            sign_messages: None,
            placeholders: PlaceholderSignatures::for_url(fdm_url),
        }
    }

    /// The client of the configured FDM: its URL and, when
    /// `fdm_token_file` is set, its bearer token. An unreadable token
    /// file is logged and the request goes without one: the FDM then
    /// refuses it (UNAUTHORIZED), which the operator sees.
    pub fn from_config(config: &pos_core::config::PosConfig) -> Self {
        Self::from_config_with(config, &HttpTls::Platform)
    }

    /// [`FdmClient::from_config`], trusting `http_tls` when the FDM's
    /// own TLS files are not configured. `http_tls` must be usable
    /// ([`HttpTls::client_builder`]).
    pub fn from_config_with(config: &pos_core::config::PosConfig, http_tls: &HttpTls) -> Self {
        let token =
            config
                .fdm_token_file
                .as_deref()
                .and_then(|path| match std::fs::read_to_string(path) {
                    Ok(raw) if !raw.trim().is_empty() => Some(raw.trim().to_string()),
                    Ok(_) => {
                        tracing::error!(path, "FDM token file is empty");
                        None
                    }
                    Err(e) => {
                        tracing::error!(path, error = %e, "FDM token file unreadable");
                        None
                    }
                });
        Self::build(
            &config.fdm_url,
            Duration::from_secs(5),
            Duration::from_secs(10),
            Self::tls_from_config(config),
            http_tls,
        )
        .with_bearer_token(token)
    }

    /// The configuration's problems ([`PosConfig::problems`]) plus what
    /// keeps the HTTPS setup from being built from its files.
    ///
    /// [`PosConfig::problems`]: pos_core::config::PosConfig::problems
    pub fn config_problems(config: &pos_core::config::PosConfig) -> Vec<String> {
        let mut problems = config.problems();
        if config.fdm_url.starts_with("https://")
            && let Some(ca) = config.fdm_ca_file.as_deref()
        {
            let client = config
                .fdm_client_cert_file
                .as_deref()
                .zip(config.fdm_client_key_file.as_deref());
            if let Err(e) = crate::fdm_tls::client_config(ca, client) {
                problems.push(format!("FDM HTTPS: {e}"));
            }
        }
        problems
    }

    /// HTTPS with the FDM's CA (and client certificate for mTLS), when
    /// `fdm_url` is https and `fdm_ca_file` is set; see
    /// [`crate::fdm_tls`]. Files that cannot be used are logged, and the
    /// FDM's certificate is then refused, which the operator sees.
    fn tls_from_config(config: &pos_core::config::PosConfig) -> Option<rustls::ClientConfig> {
        if !config.fdm_url.starts_with("https://") {
            return None;
        }
        let ca = config.fdm_ca_file.as_deref()?;
        let client = config
            .fdm_client_cert_file
            .as_deref()
            .zip(config.fdm_client_key_file.as_deref());
        crate::fdm_tls::client_config(ca, client)
            .inspect_err(|e| tracing::error!(error = %e, "FDM TLS setup failed"))
            .ok()
    }

    /// Send `token` as a bearer token with every request.
    pub fn with_bearer_token(mut self, token: Option<String>) -> Self {
        self.bearer_token = token;
        self
    }

    /// Where the messages of signed events go. POS description v2.2,
    /// Message: a warning received with a mutation's response is shown
    /// at least once per bookingDate, so every sign response's warnings
    /// and informations (minus `showPos: NEVER`) reach `sink`, whichever
    /// command signed.
    pub fn with_sign_messages(mut self, sink: SignMessageSink) -> Self {
        self.sign_messages = Some(sink);
        self
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    // ── Status Query ───────────────────────────────────────────

    /// Full selection for a status field: device block + messages.
    const STATUS_SELECTION: &'static str = "{ device { fdmId fdmSwVersion fdmDateTime bufferCapacityUsed } initialized informations { message extensions { category code showPos } } warnings { message extensions { category code showPos } } errors { message extensions { category code showPos } } }";

    /// Messages-only selection, used for the per-language aliases of
    /// the Q110 multi-language query.
    const MESSAGES_SELECTION: &'static str = "{ informations { message extensions { category code showPos } } warnings { message extensions { category code showPos } } errors { message extensions { category code showPos } } }";

    /// GraphQL enum literal for a message language.
    fn gql_language(lang: crate::types::Language) -> &'static str {
        match lang {
            crate::types::Language::Fr => "FR",
            crate::types::Language::Nl => "NL",
            crate::types::Language::De => "DE",
            crate::types::Language::En => "EN",
        }
    }

    /// Status with FR messages - the default for the POS runtime.
    pub async fn status(&self) -> Result<FdmStatus, FdmError> {
        self.status_with_language(crate::types::Language::Fr).await
    }

    /// Status with FDM messages localized to `lang` (Q100). The FDM
    /// supports FR/NL/DE/EN; callers map unsupported UI languages
    /// (e.g. zh) to one of these before calling.
    pub async fn status_with_language(
        &self,
        lang: crate::types::Language,
    ) -> Result<FdmStatus, FdmError> {
        // The POS detailed description v2.2 SDL declares the field as
        // `Status`, but the official Q100/Q110 UC fixtures and the
        // certified FDMs (module2's manual) query lowercase `status`.
        // We ask `status` first, so a conforming FDM answers in one
        // call, and fall back to `Status` if the FDM rejects it. An
        // alias keeps the response shape identical in both cases.
        #[derive(serde::Deserialize)]
        struct StatusData {
            #[serde(rename = "fdmStatus")]
            status: FdmStatus,
        }

        let gql_lang = Self::gql_language(lang);
        let selection = Self::STATUS_SELECTION;
        let lower = format!("query {{ fdmStatus: status(language: {gql_lang}) {selection} }}");
        let resp: GraphQlResponse<StatusData> =
            self.execute_raw(&lower, serde_json::Value::Null).await?;
        if let Some(d) = resp.data {
            return Ok(d.status);
        }

        let first_err = format_gql_errors(resp.errors);
        let upper = format!("query {{ fdmStatus: Status(language: {gql_lang}) {selection} }}");
        let resp: GraphQlResponse<StatusData> =
            self.execute_raw(&upper, serde_json::Value::Null).await?;
        match resp.data {
            Some(d) => Ok(d.status),
            // Report the first error: it answers the spelling the
            // fixtures and the certified FDMs use.
            None => Err(FdmError::GraphQl(first_err)),
        }
    }

    /// Q110 - status with messages in all official languages, fetched
    /// in a single request via GraphQL aliases (mirrors the published
    /// `Q110_Status_multi_lang_messages` query: primary status in EN,
    /// plus `messagesFR/NL/DE` aliases).
    pub async fn status_multi_lang(&self) -> Result<MultiLangStatus, FdmError> {
        let sel = Self::STATUS_SELECTION;
        let msg = Self::MESSAGES_SELECTION;

        let build = |field: &str| {
            format!(
                "query {{ fdmStatus: {field}(language: EN) {sel} \
                 messagesFR: {field}(language: FR) {msg} \
                 messagesNL: {field}(language: NL) {msg} \
                 messagesDE: {field}(language: DE) {msg} }}"
            )
        };

        let resp: GraphQlResponse<MultiLangStatus> = self
            .execute_raw(&build("status"), serde_json::Value::Null)
            .await?;
        if let Some(d) = resp.data {
            return Ok(d);
        }

        let first_err = format_gql_errors(resp.errors);
        let resp: GraphQlResponse<MultiLangStatus> = self
            .execute_raw(&build("Status"), serde_json::Value::Null)
            .await?;
        match resp.data {
            Some(d) => Ok(d),
            None => Err(FdmError::GraphQl(first_err)),
        }
    }

    // ── Sign Mutations ─────────────────────────────────────────

    pub async fn sign_sale(
        &self,
        data: SaleInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::Sale, &data, is_training).await
    }

    pub async fn sign_order(
        &self,
        data: OrderInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::Order, &data, is_training).await
    }

    pub async fn sign_pre_bill(
        &self,
        data: PreBillInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::PreBill, &data, is_training)
            .await
    }

    pub async fn sign_work_in(
        &self,
        data: WorkInOutInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::WorkIn, &data, is_training)
            .await
    }

    pub async fn sign_work_out(
        &self,
        data: WorkInOutInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::WorkOut, &data, is_training)
            .await
    }

    pub async fn sign_copy(
        &self,
        data: CopyInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::Copy, &data, is_training).await
    }

    pub async fn sign_money_in_out(
        &self,
        data: MoneyInOutInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::MoneyInOut, &data, is_training)
            .await
    }

    pub async fn sign_drawer_open(
        &self,
        data: DrawerOpenInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::DrawerOpen, &data, is_training)
            .await
    }

    pub async fn sign_payment_correction(
        &self,
        data: PaymentCorrectionInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::PaymentCorrection, &data, is_training)
            .await
    }

    pub async fn sign_invoice(
        &self,
        data: InvoiceInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::Invoice, &data, is_training)
            .await
    }

    pub async fn sign_report_turnover_x(
        &self,
        data: ReportTurnoverXInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::ReportTurnoverX, &data, is_training)
            .await
    }

    pub async fn sign_report_turnover_z(
        &self,
        data: ReportTurnoverZInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::ReportTurnoverZ, &data, is_training)
            .await
    }

    pub async fn sign_report_user_x(
        &self,
        data: ReportUserXInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::ReportUserX, &data, is_training)
            .await
    }

    pub async fn sign_report_user_z(
        &self,
        data: ReportUserZInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::ReportUserZ, &data, is_training)
            .await
    }

    /// M184 - end-of-day Z: the financial Z and the user Z are signed
    /// in ONE GraphQL operation, exactly like the published
    /// `M184_signReportTurnoverZUserZ` query (two mutation fields in a
    /// single document). Returns `(turnover_result, user_result)`.
    pub async fn sign_report_turnover_z_user_z(
        &self,
        turnover: ReportTurnoverZInput,
        user: ReportUserZInput,
        is_training: bool,
    ) -> Result<(SignResult, SignResult), FdmError> {
        let request = SignRequest::turnover_z_user_z(&turnover, &user, is_training)?;
        let mut results = self.send(&request).await?.into_iter();
        match (results.next(), results.next()) {
            (Some(turnover), Some(user)) => Ok((turnover, user)),
            _ => Err(FdmError::BadResponse("M184: two results expected".into())),
        }
    }

    pub async fn sign_cost_center_change(
        &self,
        data: CostCenterChangeInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_one(SignMutation::CostCenterChange, &data, is_training)
            .await
    }

    // ── Internal ───────────────────────────────────────────────

    /// Selection set shared by every sign mutation (single or M184
    /// combined) - the full `SignResult` shape.
    const SIGN_RESULT_SELECTION: &'static str = "{ posId posFiscalTicketNo posDateTime terminalId deviceId eventOperation fdmRef { fdmId fdmDateTime eventLabel eventCounter totalCounter } fdmSwVersion digitalSignature shortSignature verificationUrl vatCalc { label rate taxableAmount vatAmount totalAmount outOfScope } bufferCapacityUsed warnings { message extensions { category code showPos } } informations { message extensions { category code showPos } } footer }";

    async fn sign_one<D: Serialize>(
        &self,
        mutation: SignMutation,
        data: &D,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        let request = SignRequest::new(mutation, data, is_training)?;
        self.send(&request)
            .await?
            .pop()
            .ok_or_else(|| FdmError::BadResponse("no result".into()))
    }

    /// Send `request` as is and check each result against its input.
    pub async fn send(&self, request: &SignRequest) -> Result<Vec<SignResult>, FdmError> {
        let resp: GraphQlResponse<serde_json::Value> =
            self.post(request.body().to_string()).await?;
        if let Some(errors) = resp.errors.as_ref().filter(|e| !e.is_empty()) {
            // M184 sends two mutations in one document: when one field
            // carries a result next to `errors`, that event may have been
            // signed. No codes makes the outcome Unknown (resend the
            // same request), never "nothing was signed".
            let partly_signed = resp
                .data
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .is_some_and(|fields| fields.values().any(|v| !v.is_null()));
            return Err(FdmError::Refused {
                codes: if partly_signed {
                    Vec::new()
                } else {
                    errors
                        .iter()
                        .filter_map(|e| e.code().map(str::to_string))
                        .collect()
                },
                message: format_gql_errors(resp.errors.clone()),
            });
        }
        let data = resp
            .data
            .ok_or_else(|| FdmError::BadResponse("No data in response".into()))?;
        let inputs = request.inputs()?;
        request
            .mutation()
            .fields()
            .iter()
            .zip(&inputs)
            .map(|((field, _, _), input)| {
                parse_checked(
                    self.sign_messages.as_ref(),
                    self.placeholders,
                    &data,
                    field,
                    input,
                    request.is_training(),
                )
            })
            .collect()
    }

    async fn execute_raw<V: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: V,
    ) -> Result<GraphQlResponse<R>, FdmError> {
        // Canonical JSON (sorted keys, no whitespace): the same input
        // always gives the same bytes.
        let body = crate::canonical::to_canonical(&GqlRequest {
            query: query.to_string(),
            variables,
        })
        .map_err(|e| FdmError::Serialization(format!("canonical serialization: {e}")))?;
        self.post(body).await
    }

    async fn post<R: serde::de::DeserializeOwned>(
        &self,
        body: String,
    ) -> Result<GraphQlResponse<R>, FdmError> {
        let mut request = self
            .http
            .post(&self.url)
            .header("Content-Type", "application/json");
        if let Some(token) = &self.bearer_token {
            request = request.bearer_auth(token);
        }
        let resp = request.body(body).send().await.map_err(|e| {
            if let Some(tls) = tls_cause(&e) {
                FdmError::CertificateRefused(tls.to_string())
            } else if e.is_connect() {
                FdmError::NotConnected
            } else {
                FdmError::Http(e)
            }
        })?;

        let status = resp.status();
        let bytes = resp.bytes().await?;
        serde_json::from_slice(&bytes).map_err(|e| {
            let head = &bytes[..bytes.len().min(200)];
            FdmError::BadResponse(format!(
                "HTTP {status}, body is not a GraphQL answer ({e}): {}",
                String::from_utf8_lossy(head)
            ))
        })
    }
}

/// The TLS error behind a failed request, if any. An `io::Error`'s
/// `source()` skips the error it wraps, so look inside it too.
fn tls_cause<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<&'a rustls::Error> {
    let mut next: Option<&'a (dyn std::error::Error + 'static)> = Some(error);
    while let Some(e) = next {
        if let Some(tls) = e.downcast_ref::<rustls::Error>() {
            return Some(tls);
        }
        // hyper wraps the TLS error in an io::Error, itself in another.
        let mut io = e.downcast_ref::<std::io::Error>();
        while let Some(inner) = io.and_then(|io| io.get_ref()) {
            if let Some(tls) = inner.downcast_ref::<rustls::Error>() {
                return Some(tls);
            }
            io = inner.downcast_ref::<std::io::Error>();
        }
        next = e.source();
    }
    None
}

fn format_gql_errors(errors: Option<Vec<GraphQlError>>) -> String {
    match errors {
        Some(errs) => errs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; "),
        None => "Unknown GraphQL error".to_string(),
    }
}
