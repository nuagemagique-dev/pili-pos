//! Shared test support: an engine on a temp directory, seeded like the SPF test machine, with the
//! FDM mock and the printer mock running in the test's own runtime.
//!
//! Seeded users (`src-tauri/seed_spf_test.sql`, PINs in its header):
//! - manager "Gerant Test", INSZ `94060242576`, role admin, PIN `1111`;
//! - "Elisa", PIN `2222`; "John" (cashier), INSZ `75061189702`, PIN `4444`; "Sammy", PIN `5555`;
//! - technician `00000000097`, PIN `0097`; the robot `00000000029` has no PIN.
//!
//! Tests use `#[tokio::test(flavor = "multi_thread")]` and end with `engine.shutdown().await`.

#![expect(dead_code, reason = "helpers for the domain cards")]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use pos_ffi::records::reports::SocialResult;
use pos_ffi::{EngineOptions, PosEngine, PosError};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Manager INSZ and PIN of the seed.
pub const MANAGER_INSZ: &str = "94060242576";
pub const MANAGER_PIN: &str = "1111";
/// The handset id the tests give the engine.
pub const DEVICE_ID: &str = "EMULATOR-TEST-0001";

const POS_JSON: &str = r#"{
    "pos_id": "CPOS0031234567",
    "terminal_id": "POS-2-ANDROID",
    "device_id": "bundle-device",
    "restaurant_name": "Restaurant Test",
    "restaurant_address": "Rue de l'Exemple 1, 4000 Liege",
    "restaurant_vat": "BE0499999960",
    "restaurant_est_no": "2123456791",
    "fdm_url": "@FDM@",
    "printer_address": "@PRINTER@",
    "table_count": 25
}"#;

pub struct TestEngine {
    pub engine: Arc<PosEngine>,
    pub dir: tempfile::TempDir,
    /// `http://127.0.0.1:<port>/graphql`, the FDM mock.
    pub fdm_url: String,
    /// `127.0.0.1:<port>`, the printer mock.
    pub printer: String,
}

impl TestEngine {
    pub fn data_dir(&self) -> PathBuf {
        self.dir.path().join("data")
    }

    pub fn bundle_json(&self) -> PathBuf {
        self.dir.path().join("bundle").join("pos.json")
    }

    /// The options this engine was opened with (for a second open on the same directory).
    pub fn options(&self) -> EngineOptions {
        options_for(self.dir.path())
    }
}

pub fn options_for(dir: &Path) -> EngineOptions {
    EngineOptions {
        data_dir: dir.join("data").to_string_lossy().into_owned(),
        config_bundle: dir
            .join("bundle")
            .join("pos.json")
            .to_string_lossy()
            .into_owned(),
        device_id: Some(DEVICE_ID.to_string()),
        timers: None,
    }
}

/// Open an engine on a fresh seeded database, pointed at its own mocks.
pub async fn open_seeded() -> TestEngine {
    open_seeded_on(pos_fdm_mock::app()).await
}

/// Holds the first clock-out the FDM mock receives until the test releases it, so a call stays in
/// flight on the engine for as long as the test wants.
#[derive(Clone)]
pub struct Hold {
    armed: Arc<AtomicBool>,
    arrived: Arc<Notify>,
    release: Arc<Notify>,
}

impl Hold {
    /// Let the held clock-out go on.
    pub fn release(&self) {
        self.release.notify_one();
    }
}

async fn hold_clock_out(State(hold): State<Hold>, req: Request, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();
    if bytes.windows(11).any(|w| w == b"signWorkOut") && hold.armed.swap(false, Ordering::SeqCst) {
        hold.arrived.notify_one();
        hold.release.notified().await;
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

/// Like [`open_seeded`], with the FDM mock behind a [`Hold`].
pub async fn open_seeded_with_hold() -> (TestEngine, Hold) {
    let hold = Hold {
        armed: Arc::new(AtomicBool::new(true)),
        arrived: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
    };
    let app = pos_fdm_mock::app().layer(axum::middleware::from_fn_with_state(
        hold.clone(),
        hold_clock_out,
    ));
    (open_seeded_on(app).await, hold)
}

/// Start `end_shift` for the manager and wait until its clock-out is held at the FDM: from then
/// on one call is in flight on the engine and `shutdown` has to wait for it.
pub async fn start_held_call(
    t: &TestEngine,
    hold: &Hold,
) -> JoinHandle<Result<SocialResult, PosError>> {
    let engine = t.engine.clone();
    let task = tokio::spawn(async move {
        engine
            .end_shift(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
            .await
    });
    tokio::time::timeout(Duration::from_secs(20), hold.arrived.notified())
        .await
        .expect("the clock-out reached the FDM");
    task
}

async fn open_seeded_on(app: axum::Router) -> TestEngine {
    let fdm = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind the FDM mock");
    let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(fdm, app).await.expect("serve the FDM mock");
    });
    let printer_mock = pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
    let printer = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer_mock)
        .await
        .expect("bind the printer mock")
        .to_string();

    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("pos.json"),
        POS_JSON
            .replace("@FDM@", &fdm_url)
            .replace("@PRINTER@", &printer),
    )
    .unwrap();

    let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed))
        .expect("seed the database");

    let engine = PosEngine::open(options_for(dir.path())).unwrap_or_else(|e| panic!("open: {e}"));
    TestEngine {
        engine,
        dir,
        fdm_url,
        printer,
    }
}
