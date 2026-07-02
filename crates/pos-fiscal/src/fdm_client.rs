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
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client builder"),
            url: fdm_url.to_string(),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    // ── Status Query ───────────────────────────────────────────

    pub async fn status(&self) -> Result<FdmStatus, FdmError> {
        // The POS detailed description v2.2 SDL declares the field as
        // `Status`, but the official Q100/Q110 UC fixtures query
        // lowercase `status`. The two SPF documents contradict each
        // other, so we try `Status` first and transparently fall back
        // to `status` if the FDM rejects the capitalised field. An
        // alias keeps the response shape identical in both cases.
        const SELECTION: &str = "{ device { fdmId fdmSwVersion fdmDateTime bufferCapacityUsed } initialized informations { message extensions { category code showPos } } warnings { message extensions { category code showPos } } errors { message extensions { category code showPos } } }";

        #[derive(serde::Deserialize)]
        struct StatusData {
            #[serde(rename = "fdmStatus")]
            status: FdmStatus,
        }

        let upper = format!("query {{ fdmStatus: Status(language: FR) {SELECTION} }}");
        let resp: GraphQlResponse<StatusData> =
            self.execute_raw(&upper, serde_json::Value::Null).await?;
        if let Some(d) = resp.data {
            return Ok(d.status);
        }

        let first_err = format_gql_errors(resp.errors);
        let lower = format!("query {{ fdmStatus: status(language: FR) {SELECTION} }}");
        let resp: GraphQlResponse<StatusData> =
            self.execute_raw(&lower, serde_json::Value::Null).await?;
        match resp.data {
            Some(d) => Ok(d.status),
            // Report the original error — it is the one that matches
            // the spec-mandated spelling.
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

    async fn sign_mutation<D: Serialize>(
        &self,
        mutation_name: &str,
        input_type: &str,
        data: &D,
        is_training: bool,
    ) -> Result<SignResult, FdmError> {
        let query = format!(
            "mutation($data: {input_type}!, $isTraining: Boolean!) {{ {mutation_name}(data: $data, isTraining: $isTraining) {{ posId posFiscalTicketNo posDateTime terminalId deviceId eventOperation fdmRef {{ fdmId fdmDateTime eventLabel eventCounter totalCounter }} fdmSwVersion digitalSignature shortSignature verificationUrl vatCalc {{ label rate taxableAmount vatAmount totalAmount outOfScope }} bufferCapacityUsed warnings {{ message extensions {{ category code showPos }} }} informations {{ message extensions {{ category code showPos }} }} footer }} }}"
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
        // Same wire-correct payload, but with deterministic bytes — which
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
