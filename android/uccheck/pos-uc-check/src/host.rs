//! The reference side: the engine compiled for the host, opened the way
//! the Linux shell opens it (`src-tauri/src/main.rs`), against the FDM and
//! printer mocks served in this process.
//!
//! `call` runs one command of the scenario vocabulary (P7.md section 4.3).
//! Every engine command calls the same `pos_engine::commands::*::do_*`
//! function its Tauri wrapper in `src-tauri/src/ipc/` calls, with the
//! arguments of the `invoke` object (camelCase names, the engine input
//! types for the structured ones). The result is the serde JSON of the
//! engine result, as the Vue side sees it. This file computes no amount
//! and holds no fiscal rule.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use pos_engine::commands::{
    auth, checkout, copy, financial, modifiers, order, refund, report, session, settings, social,
    tables, unpaid,
};
use pos_engine::engine::{ConfigSource, Engine, EngineConfig, EnginePaths, EngineTimers};
use pos_engine::{HttpTls, NullSink, SoftwareIdentity, install};
use pos_fiscal::payment::PaymentPart;
use pos_printer_mock::Printer;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::task::JoinHandle;

/// The commands that never reach the engine.
pub const MOCK_COMMANDS: [&str; 4] = [
    "fdm_control",
    "fdm_control_reset",
    "printer_control",
    "sleep_ms",
];

/// The longest `sleep_ms` a scenario may ask for.
pub const MAX_SLEEP_MS: u64 = 10_000;

/// How long `login` waits for the clock-in it queued to settle.
const CLOCK_IN_WAIT: Duration = Duration::from_millis(1500);

/// What the Android bundle writes into `pos.json` (`scripts/android/make_dev_bundle.sh`),
/// but for the two addresses.
fn pos_json(fdm_url: &str, printer_address: &str) -> Value {
    json!({
        "pos_id": "CPOS0031234567",
        "terminal_id": "POS-2-ANDROID",
        "device_id": "pos-json-device",
        "restaurant_name": "Restaurant Test",
        "restaurant_address": "1 Rue du Test, 4000 Liege",
        "restaurant_vat": "BE0000000097",
        "restaurant_est_no": "2000000042",
        "fdm_url": fdm_url,
        "printer_address": printer_address,
        "table_count": 25,
    })
}

/// One engine on a fresh data directory, with its own mocks.
pub struct Host {
    engine: Engine,
    http: reqwest::Client,
    fdm_base: String,
    printer_http: String,
    /// The `posSwVersion` the engine was opened with.
    pub sw_version: String,
    tasks: Vec<JoinHandle<()>>,
    _dir: tempfile::TempDir,
}

/// The `version` of `[workspace.package]` in the root `Cargo.toml` of `repo`.
pub fn workspace_version(repo: &Path) -> anyhow::Result<String> {
    let path = repo.join("Cargo.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    version_of_workspace_package(&text)
        .with_context(|| format!("no [workspace.package] version in {}", path.display()))
}

pub fn version_of_workspace_package(manifest: &str) -> Option<String> {
    let mut in_section = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == "[workspace.package]";
            continue;
        }
        if in_section && let Some(rest) = line.strip_prefix("version") {
            let value = rest.trim_start().strip_prefix('=')?.trim();
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

impl Host {
    /// Start the mocks and open a fresh engine (fresh database, seeded
    /// with `src-tauri/seed_spf_test.sql`, background loops running).
    pub async fn start(repo: &Path) -> anyhow::Result<Host> {
        let version = workspace_version(repo)?;
        let sw_version = format!("pili-pos/{version}");
        let seed = repo.join("src-tauri").join("seed_spf_test.sql");
        if !seed.is_file() {
            bail!(
                "{} not found (is --repo the repository root?)",
                seed.display()
            );
        }

        let mut tasks = Vec::new();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let fdm_port = listener.local_addr()?.port();
        let fdm_app = pos_fdm_mock::app_with(pos_fdm_mock::Control::default());
        tasks.push(tokio::spawn(async move {
            let _ = axum::serve(listener, fdm_app).await;
        }));

        let printer = Printer::new(pos_printer_mock::Control::default(), None);
        let printer_addr = pos_printer_mock::bind("127.0.0.1:0".parse()?, printer.clone()).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let printer_http_port = listener.local_addr()?.port();
        let printer_app = pos_printer_mock::http_app(printer);
        tasks.push(tokio::spawn(async move {
            let _ = axum::serve(listener, printer_app).await;
        }));

        let dir = tempfile::tempdir().context("temp data directory")?;
        let data_dir: PathBuf = dir.path().join("data");
        let config_path = dir.path().join("pos.json");
        let fdm_url = format!("http://127.0.0.1:{fdm_port}/graphql");
        std::fs::write(
            &config_path,
            serde_json::to_vec_pretty(&pos_json(&fdm_url, &printer_addr.to_string()))?,
        )?;

        let paths = EnginePaths::under(data_dir);
        install::init_db(&paths.db, Some(&seed)).context("init_db with the seed")?;

        let engine = Engine::open(EngineConfig {
            paths,
            config: ConfigSource::File(config_path),
            events: Arc::new(NullSink),
            timers: EngineTimers::default(),
            identity: SoftwareIdentity::new(sw_version.clone())
                .map_err(|e| anyhow::anyhow!("software identity: {e}"))?,
            http_tls: HttpTls::Platform,
            device_id_override: None,
            support_program: None,
        })
        .context("Engine::open")?;
        engine.start_background(&tokio::runtime::Handle::current());

        Ok(Host {
            engine,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            fdm_base: format!("http://127.0.0.1:{fdm_port}"),
            printer_http: format!("http://127.0.0.1:{printer_http_port}"),
            sw_version,
            tasks,
            _dir: dir,
        })
    }

    /// Stop the engine's loops and the mocks.
    pub async fn stop(self) {
        self.engine.shutdown();
        let background = self.engine.take_background_tasks();
        for task in background {
            let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
        }
        for task in self.tasks {
            task.abort();
        }
    }

    // ── The mocks, over HTTP like the Android host script ─────────

    /// Clear the FDM request log and the printer jobs, and put both mocks
    /// back to their default states.
    pub async fn reset_mocks(&self) -> anyhow::Result<()> {
        self.http
            .delete(format!("{}/mock/requests", self.fdm_base))
            .send()
            .await?
            .error_for_status()?;
        self.http
            .delete(format!("{}/jobs", self.printer_http))
            .send()
            .await?
            .error_for_status()?;
        self.post_json(&format!("{}/mock/control", self.fdm_base), &json!({}))
            .await?;
        self.post_json(&format!("{}/control", self.printer_http), &json!({}))
            .await?;
        Ok(())
    }

    async fn post_json(&self, url: &str, body: &Value) -> anyhow::Result<Value> {
        let response = self
            .http
            .post(url)
            .json(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await.unwrap_or(Value::Null))
    }

    /// `GET /mock/requests`: every call the FDM mock logged.
    pub async fn fdm_requests(&self) -> anyhow::Result<Vec<Value>> {
        let calls: Value = self
            .http
            .get(format!("{}/mock/requests", self.fdm_base))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(calls.as_array().cloned().unwrap_or_default())
    }

    /// `GET /jobs`: the printer jobs, in print order.
    pub async fn printer_jobs(&self) -> anyhow::Result<Vec<Value>> {
        let jobs: Value = self
            .http
            .get(format!("{}/jobs", self.printer_http))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(jobs.as_array().cloned().unwrap_or_default())
    }

    /// Wait until the FDM log and the printer jobs stop growing: a
    /// background pass (the clock-in a login queued, a status check) or a
    /// print that finished after its step must not land in the next
    /// scenario.
    pub async fn settle(&self) -> anyhow::Result<()> {
        let mut last = (usize::MAX, usize::MAX);
        let mut stable = 0;
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let now = (
                self.fdm_requests().await?.len(),
                self.printer_jobs().await?.len(),
            );
            if now == last {
                stable += 1;
                if stable >= 2 {
                    return Ok(());
                }
            } else {
                stable = 0;
                last = now;
            }
        }
        Ok(())
    }

    async fn count_operation(&self, operation: &str) -> usize {
        self.fdm_requests()
            .await
            .map(|calls| calls.iter().filter(|c| c["operation"] == operation).count())
            .unwrap_or(0)
    }

    // ── Commands ──────────────────────────────────────────────────

    /// Run one command of the vocabulary. `Err` is the engine's error
    /// string (`code|k=v`), or `uc_bad_args|...` when the scenario's
    /// arguments are not what the command takes.
    pub async fn call(&self, cmd: &str, args: &Value) -> Result<Value, String> {
        if cmd == "login" {
            return self.login(args).await;
        }
        match cmd {
            "fdm_control" => self.mock_post("fdm", "/mock/control", args).await,
            "fdm_control_reset" => self.mock_post("fdm", "/mock/control", &json!({})).await,
            "printer_control" => self.mock_post("printer", "/control", args).await,
            "sleep_ms" => {
                let ms = req_i64(args, "ms")?;
                if !(0..=MAX_SLEEP_MS as i64).contains(&ms) {
                    return Err(bad_args(format!("ms must be 0 to {MAX_SLEEP_MS}")));
                }
                tokio::time::sleep(Duration::from_millis(ms as u64)).await;
                Ok(Value::Null)
            }
            _ => self.engine_call(cmd, args).await,
        }
    }

    async fn mock_post(&self, which: &str, path: &str, body: &Value) -> Result<Value, String> {
        let base = if which == "fdm" {
            &self.fdm_base
        } else {
            &self.printer_http
        };
        self.post_json(&format!("{base}{path}"), body)
            .await
            .map(|_| Value::Null)
            .map_err(|e| format!("uc_mock_failed|detail={e}"))
    }

    /// `login`, then wait for the clock-in it queued: the engine signs it
    /// in the background (the presence worker), and a later step must not
    /// race it. The wait ends when the employee is on shift or the FDM
    /// logged a signWorkIn (answered or refused), at most 1.5 s.
    async fn login(&self, args: &Value) -> Result<Value, String> {
        let insz = req_str(args, "insz")?;
        let pin = opt_str(args, "pin")?;
        let before = self.count_operation("signWorkIn").await;
        let user = ser(auth::do_login(&self.engine, insz.clone(), pin))?;
        let deadline = tokio::time::Instant::now() + CLOCK_IN_WAIT;
        while tokio::time::Instant::now() < deadline {
            let on_shift = social::do_list_open_shifts(&self.engine)
                .map(|shifts| shifts.iter().any(|s| s.user_insz == insz))
                .unwrap_or(false);
            if on_shift || self.count_operation("signWorkIn").await > before {
                tokio::time::sleep(Duration::from_millis(50)).await;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok(user)
    }

    async fn engine_call(&self, cmd: &str, a: &Value) -> Result<Value, String> {
        let e = &self.engine;
        match cmd {
            "logout" => ser(auth::do_logout(e)),
            "set_training_mode" => ser(settings::do_set_training_mode(e, req_bool(a, "enabled")?)),
            "work_in" => ser(social::do_work_in(e, req_str(a, "employeeInsz")?).await),
            "work_out" => ser(social::do_work_out(e, req_str(a, "employeeInsz")?).await),
            "end_shift" => {
                ser(social::do_end_shift(e, req_str(a, "insz")?, opt_str(a, "pin")?).await)
            }
            "create_order" => ser(order::do_create_order(e, typed(a, "input")?)),
            "add_order_line" => ser(order::do_add_order_line(e, typed(a, "input")?)),
            "add_order_line_with_modifiers" => {
                let input = typed(a, "input")?;
                ser(modifiers::do_add_order_line_with_modifiers(e, &input))
            }
            "update_order_line_quantity" => {
                let quantity = i32::try_from(req_i64(a, "quantity")?)
                    .map_err(|_| bad_args("quantity out of range".to_string()))?;
                ser(order::do_update_order_line_quantity(
                    e,
                    req_i64(a, "lineId")?,
                    quantity,
                ))
            }
            "remove_order_line" => ser(order::do_remove_order_line(e, req_i64(a, "lineId")?)),
            "set_active_order" => ser(session::do_set_active_order(e, opt_i64(a, "orderId")?)),
            "leave_order" => {
                ser(session::do_leave_order(e, req_i64(a, "orderId")?, typed(a, "reason")?).await)
            }
            "sign_table_order" => ser(tables::do_sign_table_order(e, req_i64(a, "orderId")?).await),
            "sign_pre_bill" => ser(tables::do_sign_pre_bill(e, req_i64(a, "orderId")?).await),
            "checkout" => {
                let payments: Vec<PaymentPart> = typed(a, "payments")?;
                let delivery = opt_typed(a, "delivery")?;
                ser(checkout::do_checkout_split(
                    e,
                    req_i64(a, "orderId")?,
                    &payments,
                    delivery.as_ref(),
                )
                .await)
            }
            "close_unpaid" => ser(unpaid::do_close_unpaid(
                e,
                req_i64(a, "orderId")?,
                &req_str(a, "insz")?,
                &req_str(a, "pin")?,
            )
            .await),
            "refund_checkout" => {
                let items = typed(a, "items")?;
                let delivery = opt_typed(a, "delivery")?;
                ser(refund::do_refund_checkout_with(
                    e,
                    req_i64(a, "originalFiscalEventId")?,
                    items,
                    req_str(a, "employeeInsz")?,
                    delivery.as_ref(),
                )
                .await)
            }
            "copy_ticket" => {
                let delivery = opt_typed(a, "delivery")?;
                ser(copy::do_copy_ticket_with(
                    e,
                    req_i64(a, "fiscalEventId")?,
                    req_str(a, "employeeInsz")?,
                    delivery.as_ref(),
                )
                .await)
            }
            "money_in_out" => ser(financial::do_money_in_out(
                e,
                req_str(a, "employeeInsz")?,
                req_i64(a, "amountCents")?,
                typed(a, "direction")?,
                req_str(a, "reason")?,
            )
            .await),
            "open_drawer_fiscal" => {
                ser(financial::do_open_drawer_fiscal(e, req_str(a, "employeeInsz")?).await)
            }
            "generate_x_report" => {
                ser(report::do_generate_x_report(e, req_str(a, "employeeInsz")?).await)
            }
            "generate_z_report" => {
                ser(report::do_generate_z_report(e, req_str(a, "employeeInsz")?).await)
            }
            "generate_user_x_report" => {
                ser(report::do_generate_user_x_report(e, req_str(a, "employeeInsz")?).await)
            }
            "check_fdm_status" => ser(checkout::do_check_fdm_status(e).await),
            "list_fiscal_events" => ser(copy::do_list_fiscal_events(e, opt_i64(a, "limit")?)),
            "get_order" => ser(order::do_get_order(e, req_i64(a, "orderId")?)),
            other => Err(bad_args(format!("unknown command {other}"))),
        }
    }
}

// ── Argument helpers ───────────────────────────────────────────────

fn bad_args(detail: String) -> String {
    format!("uc_bad_args|detail={}", detail.replace('|', "/"))
}

/// The JSON of an engine result, as `invoke` returns it.
fn ser<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
    result.and_then(|value| {
        serde_json::to_value(value).map_err(|e| format!("uc_serialize|detail={e}"))
    })
}

fn field<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.get(key).filter(|v| !v.is_null())
}

fn req_i64(args: &Value, key: &str) -> Result<i64, String> {
    field(args, key)
        .and_then(Value::as_i64)
        .ok_or_else(|| bad_args(format!("{key} must be an integer")))
}

fn opt_i64(args: &Value, key: &str) -> Result<Option<i64>, String> {
    match field(args, key) {
        None => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| bad_args(format!("{key} must be an integer or null"))),
    }
}

fn req_bool(args: &Value, key: &str) -> Result<bool, String> {
    field(args, key)
        .and_then(Value::as_bool)
        .ok_or_else(|| bad_args(format!("{key} must be a boolean")))
}

fn req_str(args: &Value, key: &str) -> Result<String, String> {
    field(args, key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| bad_args(format!("{key} must be a string")))
}

fn opt_str(args: &Value, key: &str) -> Result<Option<String>, String> {
    match field(args, key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| bad_args(format!("{key} must be a string or null"))),
    }
}

fn typed<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    let value = field(args, key).ok_or_else(|| bad_args(format!("{key} is required")))?;
    serde_json::from_value(value.clone()).map_err(|e| bad_args(format!("{key}: {e}")))
}

fn opt_typed<T: DeserializeOwned>(args: &Value, key: &str) -> Result<Option<T>, String> {
    match field(args, key) {
        None => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|e| bad_args(format!("{key}: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_workspace_version_is_read_from_its_section() {
        let manifest = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.1.0\"\nedition = \"2024\"\n\n[package]\nversion = \"9.9.9\"\n";
        assert_eq!(
            version_of_workspace_package(manifest).as_deref(),
            Some("1.1.0")
        );
        assert_eq!(
            version_of_workspace_package("[package]\nversion = \"2\"\n"),
            None
        );
    }

    #[test]
    fn argument_helpers_treat_null_as_absent_and_reject_wrong_types() {
        let args = json!({ "a": 3, "b": null, "c": "x", "d": true });
        assert_eq!(req_i64(&args, "a"), Ok(3));
        assert_eq!(opt_i64(&args, "b"), Ok(None));
        assert_eq!(opt_i64(&args, "missing"), Ok(None));
        assert!(req_i64(&args, "c").unwrap_err().starts_with("uc_bad_args"));
        assert_eq!(req_str(&args, "c"), Ok("x".to_string()));
        assert_eq!(opt_str(&args, "b"), Ok(None));
        assert_eq!(req_bool(&args, "d"), Ok(true));
        assert!(opt_str(&args, "a").is_err());
        assert!(typed::<Vec<i32>>(&args, "a").is_err());
    }
}
