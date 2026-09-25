use std::time::Duration;

use serde::Serialize;

use crate::inputs::*;
use crate::response::*;

#[derive(Debug, thiserror::Error)]
pub enum FdmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("GraphQL error: {0}")]
    GraphQl(String),
    #[error("FDM not connected")]
    NotConnected,
    #[error("FDM not operational: {0}")]
    NotOperational(String),
}

/// Client for communicating with the FDM over GraphQL/HTTP.
pub struct FdmClient {
    http: reqwest::Client,
    url: String,
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
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(connect_timeout)
                .timeout(timeout)
                .build()
                .expect("reqwest client builder"),
            url: fdm_url.to_string(),
        }
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
        // `Status`, but the official Q100/Q110 UC fixtures query
        // lowercase `status`. The two SPF documents contradict each
        // other, so we try `Status` first and transparently fall back
        // to `status` if the FDM rejects the capitalised field. An
        // alias keeps the response shape identical in both cases.
        #[derive(serde::Deserialize)]
        struct StatusData {
            #[serde(rename = "fdmStatus")]
            status: FdmStatus,
        }

        let gql_lang = Self::gql_language(lang);
        let selection = Self::STATUS_SELECTION;
        let upper = format!("query {{ fdmStatus: Status(language: {gql_lang}) {selection} }}");
        let resp: GraphQlResponse<StatusData> =
            self.execute_raw(&upper, serde_json::Value::Null).await?;
        if let Some(d) = resp.data {
            return Ok(d.status);
        }

        let first_err = format_gql_errors(resp.errors);
        let lower = format!("query {{ fdmStatus: status(language: {gql_lang}) {selection} }}");
        let resp: GraphQlResponse<StatusData> =
            self.execute_raw(&lower, serde_json::Value::Null).await?;
        match resp.data {
            Some(d) => Ok(d.status),
            // Report the original error - it is the one that matches
            // the spec-mandated spelling.
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
            .execute_raw(&build("Status"), serde_json::Value::Null)
            .await?;
        if let Some(d) = resp.data {
            return Ok(d);
        }

        let first_err = format_gql_errors(resp.errors);
        let resp: GraphQlResponse<MultiLangStatus> = self
            .execute_raw(&build("status"), serde_json::Value::Null)
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
        self.sign_mutation("signSale", "SaleInput", &data, is_training)
            .await
    }

    pub async fn sign_order(
        &self,
        data: OrderInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signOrder", "OrderInput", &data, is_training)
            .await
    }

    pub async fn sign_pre_bill(
        &self,
        data: PreBillInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signPreBill", "PreBillInput", &data, is_training)
            .await
    }

    pub async fn sign_work_in(
        &self,
        data: WorkInOutInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signWorkIn", "WorkInOutInput", &data, is_training)
            .await
    }

    pub async fn sign_work_out(
        &self,
        data: WorkInOutInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signWorkOut", "WorkInOutInput", &data, is_training)
            .await
    }

    pub async fn sign_copy(
        &self,
        data: CopyInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signCopy", "CopyInput", &data, is_training)
            .await
    }

    pub async fn sign_money_in_out(
        &self,
        data: MoneyInOutInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signMoneyInOut", "MoneyInOutInput", &data, is_training)
            .await
    }

    pub async fn sign_drawer_open(
        &self,
        data: DrawerOpenInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signDrawerOpen", "DrawerOpenInput", &data, is_training)
            .await
    }

    pub async fn sign_payment_correction(
        &self,
        data: PaymentCorrectionInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation(
            "signPaymentCorrection",
            "PaymentCorrectionInput",
            &data,
            is_training,
        )
        .await
    }

    pub async fn sign_invoice(
        &self,
        data: InvoiceInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signInvoice", "InvoiceInput", &data, is_training)
            .await
    }

    pub async fn sign_report_turnover_x(
        &self,
        data: ReportTurnoverXInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation(
            "signReportTurnoverX",
            "ReportTurnoverXInput",
            &data,
            is_training,
        )
        .await
    }

    pub async fn sign_report_turnover_z(
        &self,
        data: ReportTurnoverZInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation(
            "signReportTurnoverZ",
            "ReportTurnoverZInput",
            &data,
            is_training,
        )
        .await
    }

    pub async fn sign_report_user_x(
        &self,
        data: ReportUserXInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signReportUserX", "ReportUserXInput", &data, is_training)
            .await
    }

    pub async fn sign_report_user_z(
        &self,
        data: ReportUserZInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation("signReportUserZ", "ReportUserZInput", &data, is_training)
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
        let sel = Self::SIGN_RESULT_SELECTION;
        let query = format!(
            "mutation($turnover: ReportTurnoverZInput!, $user: ReportUserZInput!, $isTraining: Boolean!) {{ signReportTurnoverZ(data: $turnover, isTraining: $isTraining) {sel} signReportUserZ(data: $user, isTraining: $isTraining) {sel} }}"
        );

        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Vars<'a> {
            turnover: &'a ReportTurnoverZInput,
            user: &'a ReportUserZInput,
            is_training: bool,
        }

        let vars = Vars {
            turnover: &turnover,
            user: &user,
            is_training,
        };

        let resp: GraphQlResponse<serde_json::Value> = self.execute_raw(&query, &vars).await?;
        if resp.errors.as_ref().is_some_and(|e| !e.is_empty()) {
            return Err(FdmError::GraphQl(format_gql_errors(resp.errors)));
        }
        let data = resp
            .data
            .ok_or_else(|| FdmError::GraphQl("No data in response".into()))?;

        let parse = |key: &str| -> Result<SignResult, FdmError> {
            let value = data
                .get(key)
                .ok_or_else(|| FdmError::GraphQl(format!("Missing {key} in response")))?;
            serde_json::from_value(value.clone())
                .map_err(|e| FdmError::GraphQl(format!("Failed to parse {key}: {e}")))
        };
        Ok((parse("signReportTurnoverZ")?, parse("signReportUserZ")?))
    }

    pub async fn sign_cost_center_change(
        &self,
        data: CostCenterChangeInput,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        self.sign_mutation(
            "signCostCenterChange",
            "CostCenterChangeInput",
            &data,
            is_training,
        )
        .await
    }

    // ── Internal ───────────────────────────────────────────────

    /// Selection set shared by every sign mutation (single or M184
    /// combined) - the full `SignResult` shape.
    const SIGN_RESULT_SELECTION: &'static str = "{ posId posFiscalTicketNo posDateTime terminalId deviceId eventOperation fdmRef { fdmId fdmDateTime eventLabel eventCounter totalCounter } fdmSwVersion digitalSignature shortSignature verificationUrl vatCalc { label rate taxableAmount vatAmount totalAmount outOfScope } bufferCapacityUsed warnings { message extensions { category code showPos } } informations { message extensions { category code showPos } } footer }";

    async fn sign_mutation<D: Serialize>(
        &self,
        mutation_name: &str,
        input_type: &str,
        data: &D,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        let sel = Self::SIGN_RESULT_SELECTION;
        let query = format!(
            "mutation($data: {input_type}!, $isTraining: Boolean!) {{ {mutation_name}(data: $data, isTraining: $isTraining) {sel} }}"
        );

        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Vars<'a, D: Serialize> {
            data: &'a D,
            is_training: bool,
        }

        let vars = Vars { data, is_training };

        let resp: GraphQlResponse<serde_json::Value> = self.execute_raw(&query, &vars).await?;

        if resp.errors.as_ref().is_some_and(|e| !e.is_empty()) {
            let msg = resp
                .errors
                .as_ref()
                .unwrap()
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(FdmError::GraphQl(msg));
        }

        let data = resp
            .data
            .ok_or_else(|| FdmError::GraphQl("No data in response".into()))?;

        // The response is at data.<mutationName>
        let result_value = data
            .get(mutation_name)
            .ok_or_else(|| FdmError::GraphQl(format!("Missing {mutation_name} in response")))?;

        serde_json::from_value(result_value.clone())
            .map_err(|e| FdmError::GraphQl(format!("Failed to parse SignResult: {e}")))
    }

    async fn execute_raw<V: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: V,
    ) -> Result<GraphQlResponse<R>, FdmError> {
        let body = GqlRequest {
            query: query.to_string(),
            variables,
        };

        // Send the request as canonical JSON (sorted keys, no whitespace).
        // Same wire-correct payload, but with deterministic bytes - which
        // matters for: (1) request log diffs in the cert rehearsal capture,
        // (2) reproducing exact requests when chasing FDM-reported errors,
        // (3) future request-hash dedup in the local retry table. The FDM
        // does its own canonicalization for signing, so this does not
        // affect the digital signature it returns.
        let body_canonical = crate::canonical::to_canonical(&body)
            .map_err(|e| FdmError::GraphQl(format!("canonical serialization: {e}")))?;

        let resp = self
            .http
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(body_canonical)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    FdmError::NotConnected
                } else {
                    FdmError::Http(e)
                }
            })?;

        let gql_resp: GraphQlResponse<R> = resp.json().await?;
        Ok(gql_resp)
    }
}

fn format_gql_errors(errors: Option<Vec<GraphQlError>>) -> String {
    match errors {
        Some(errs) => errs
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; "),
        None => "Unknown GraphQL error".to_string(),
    }
}
