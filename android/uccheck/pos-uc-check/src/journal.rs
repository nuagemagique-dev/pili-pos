//! The journal of one scenario run (format 1, P7.md section 4.4), written
//! by `record` on the host and by the Kotlin runner on the device, read
//! by `compare`.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Journal {
    pub format: u32,
    /// `linux` (the engine on the host) or `android`.
    pub side: String,
    pub group: String,
    pub scenario: String,
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(rename = "posSwVersion")]
    pub pos_sw_version: String,
    pub steps: Vec<JournalStep>,
    /// The non-Status FDM calls, in order.
    pub fdm: Vec<FdmCall>,
    #[serde(default, rename = "statusCalls")]
    pub status_calls: Vec<FdmCall>,
    /// The text of each printer job, in print order.
    pub tickets: Vec<String>,
    /// `PASS` when every step met its `expect`.
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JournalStep {
    pub cmd: String,
    /// The step met its `expect` (UC.md section 5): an error the scenario expects is an ok step.
    pub ok: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub result: Value,
    /// Only on the host side, and only when the step did not meet its
    /// `expect`: why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmet: Option<String>,
}

/// One call as `GET /mock/requests` logged it (the mock's own `at` stamp
/// is left out: it is wall-clock time).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FdmCall {
    #[serde(default)]
    pub operation: String,
    pub request: Value,
    pub response: Value,
}

impl Journal {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("journal {}", path.display()))
    }

    pub fn write(&self, out_root: &Path) -> anyhow::Result<PathBuf> {
        let dir = out_root.join(&self.side).join(&self.group);
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(format!("{}.json", self.scenario));
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }
}

/// `(group, scenario)` of every journal under `root/<group>/<scenario>.json`,
/// sorted.
pub fn list(root: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let mut found = Vec::new();
    let groups = std::fs::read_dir(root).with_context(|| format!("reading {}", root.display()))?;
    for group in groups.filter_map(Result::ok) {
        if !group.path().is_dir() {
            continue;
        }
        let group_name = group.file_name().to_string_lossy().into_owned();
        for file in std::fs::read_dir(group.path())?.filter_map(Result::ok) {
            let path = file.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Some(stem) = path.file_stem()
            {
                found.push((group_name.clone(), stem.to_string_lossy().into_owned()));
            }
        }
    }
    found.sort();
    Ok(found)
}
