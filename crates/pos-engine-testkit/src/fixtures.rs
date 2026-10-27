//! Fixture discovery for SPF Finances UC sets.
//!
//! Each UC folder under `docs/20260821_UC_BasicSet_POS_FDM_1.8/` (or the
//! Reports / Advanced equivalents) carries the same triple:
//!
//! - `<UC>_GQL-Variables.json` - the GraphQL `variables` payload, i.e.
//!   the SCE-2.0 input struct serialised to JSON.
//! - `<UC>_GQL-Query.graphql`   - the mutation/query string (not used
//!   by the e2e harness; the `FdmClient` generates its own).
//! - `<UC>_Response.json` (or `signSale_Response.json`, etc.) - the
//!   canonical FDM response we replay through wiremock.
//!
//! Multi-step UCs (M112A, M112B, advanced UCs) live in sub-folders
//! named `<UC>_1_OriginalSale`, `<UC>_2_PartialRefund`, … . We surface
//! each step as its own `UcFixture` with a composite `uc_id` like
//! `"M112A_signSale_RefundPartial/M112A_2_PartialRefund"`.

use std::fs;
use std::path::{Path, PathBuf};

/// One discovered fixture set within a single UC folder (or sub-folder
/// for multi-step UCs).
#[derive(Debug, Clone)]
pub struct UcFixture {
    /// Stable test identifier, e.g. `"M110_signSale"` or
    /// `"M112A_signSale_RefundPartial/M112A_2_PartialRefund"`.
    pub uc_id: String,
    /// Absolute path to the `*_GQL-Variables.json` file, if any.
    /// Status queries (Q*) have no Variables file.
    pub variables_path: Option<PathBuf>,
    /// Absolute path to the response JSON. Always present (we skip
    /// folders that don't have one).
    pub response_path: PathBuf,
    /// What flavour of FDM payload the response carries.
    pub response_kind: ResponseKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ResponseKind {
    /// `signSale` / `signOrder` / `signReport*` / … - wrapped in
    /// `data.<mutationName>` and shaped like `SignResult`.
    SignResult,
    /// `Status` query - shaped like `FdmStatus`.
    Status,
}

/// Workspace root, derived from the crate manifest dir at compile
/// time. `env!("CARGO_MANIFEST_DIR")` in this crate points at
/// `<workspace>/crates/pos-engine-testkit`, so two `..` get to the
/// workspace root.
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

pub fn basic_set_root() -> PathBuf {
    workspace_root()
        .join("docs")
        .join("20260821_UC_BasicSet_POS_FDM_1.8")
}

pub fn reports_root() -> PathBuf {
    workspace_root()
        .join("docs")
        .join("20260821_UC_Reports_1.3")
}

pub fn advanced_set_root() -> PathBuf {
    workspace_root()
        .join("docs")
        .join("20260821_UC_AdvancedSet_POS_FDM_1.6")
}

/// Discover all fixtures under a UC-set root directory.
///
/// Walks the top-level folder for entries matching `M*` / `Q*` / `UC*`, then
/// for each:
/// - If the folder itself contains a `*_Response.json`, register one
///   fixture.
/// - Otherwise recurse one level: each sub-folder with a Response file
///   becomes its own fixture (multi-step UCs).
pub fn discover_fixtures(root: &Path) -> Vec<UcFixture> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(it) => it,
        Err(e) => panic!("cannot read fixture root {}: {e}", root.display()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !(name.starts_with('M') || name.starts_with('Q') || name.starts_with("UC")) {
            continue;
        }

        if let Some((response_path, response_kind)) = find_response_file(&path) {
            let variables_path = find_variables_file(&path);
            out.push(UcFixture {
                uc_id: name.clone(),
                variables_path,
                response_path,
                response_kind,
            });
            continue;
        }

        // Multi-step UC: register each child directory as its own fixture.
        let Ok(sub_entries) = fs::read_dir(&path) else {
            continue;
        };
        for sub in sub_entries.flatten() {
            let sub_path = sub.path();
            if !sub_path.is_dir() {
                continue;
            }
            let Some((response_path, response_kind)) = find_response_file(&sub_path) else {
                continue;
            };
            let variables_path = find_variables_file(&sub_path);
            let sub_name = sub.file_name().to_string_lossy().into_owned();
            out.push(UcFixture {
                uc_id: format!("{name}/{sub_name}"),
                variables_path,
                response_path,
                response_kind,
            });
        }
    }

    out.sort_by(|a, b| a.uc_id.cmp(&b.uc_id));
    out
}

/// Find the canonical UC by exact ID or prefix-match (the latter is
/// useful for "M110" matching "`M110_signSale`"). Panics if no match.
pub fn load_uc(root: &Path, uc_id_or_prefix: &str) -> UcFixture {
    let all = discover_fixtures(root);
    if let Some(exact) = all.iter().find(|f| f.uc_id == uc_id_or_prefix) {
        return exact.clone();
    }
    if let Some(prefixed) = all.iter().find(|f| f.uc_id.starts_with(uc_id_or_prefix)) {
        return prefixed.clone();
    }
    let known: Vec<&str> = all.iter().map(|f| f.uc_id.as_str()).collect();
    panic!(
        "no fixture matches {uc_id_or_prefix:?} under {}. Known: {known:?}",
        root.display()
    );
}

fn find_response_file(dir: &Path) -> Option<(PathBuf, ResponseKind)> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if Path::new(&name).extension().is_none_or(|e| e != "json") {
            continue;
        }
        if name.contains("response") {
            let kind = if name.starts_with("status") {
                ResponseKind::Status
            } else {
                ResponseKind::SignResult
            };
            return Some((entry.path(), kind));
        }
    }
    None
}

fn find_variables_file(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        if name.ends_with("_GQL-Variables.json") {
            return Some(entry.path());
        }
    }
    None
}

/// Read the raw response JSON file. The harness extracts the inner
/// `data.<key>` payload before handing it to wiremock - the AP needs
/// the FULL envelope as published, since `FdmClient` expects that
/// shape.
pub fn read_response_envelope(fixture: &UcFixture) -> serde_json::Value {
    let raw = fs::read_to_string(&fixture.response_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", fixture.response_path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", fixture.response_path.display()))
}

/// Read the variables file. Panics if the fixture has none.
pub fn read_variables(fixture: &UcFixture) -> serde_json::Value {
    let path = fixture
        .variables_path
        .as_ref()
        .unwrap_or_else(|| panic!("{} has no Variables.json", fixture.uc_id));
    let raw =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()))
}
