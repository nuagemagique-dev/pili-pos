//! Installation helpers: create the database before the engine opens, and check the
//! provisioning bundle's `pos.json`. File work runs on a dedicated thread (see
//! [`crate::helpers::on_thread`]).

use std::path::{Path, PathBuf};

use pos_engine::engine::{ConfigSource, EnginePaths, LOCK_FILE};

use crate::error::PosError;
use crate::helpers::on_thread;

fn absolute(call: &str, name: &str, value: &str) -> Result<PathBuf, PosError> {
    let path = Path::new(value);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Err(PosError::internal(format!(
            "{call}: {name} must be an absolute path: {value}"
        )))
    }
}

/// Take the exclusive lock the engine takes on `<data_dir>/engine.lock`; it lives as long as the
/// returned file.
fn lock_data_dir(data_dir: &Path) -> Result<std::fs::File, PosError> {
    let fail = |e: std::io::Error| PosError::internal(format!("install_init_db: {e}"));
    std::fs::create_dir_all(data_dir).map_err(fail)?;
    let path = data_dir.join(LOCK_FILE);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(fail)?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(PosError::internal(format!(
            "install_init_db: engine already open: {}",
            path.display()
        ))),
        Err(std::fs::TryLockError::Error(e)) => Err(fail(e)),
    }
}

/// Create or migrate `<data_dir>/pos.db`, and load the SQL seed at `seed_sql_path` when given.
/// Holds the engine's lock on the data directory meanwhile and refuses (`Internal`, "engine
/// already open") when an engine holds it. The engine refuses a seed once the database holds
/// fiscal events.
#[uniffi::export]
pub async fn install_init_db(
    data_dir: String,
    seed_sql_path: Option<String>,
) -> Result<(), PosError> {
    let data_dir = absolute("install_init_db", "data_dir", &data_dir)?;
    let seed = seed_sql_path
        .as_deref()
        .map(|s| absolute("install_init_db", "seed_sql_path", s))
        .transpose()?;
    on_thread("pos-ffi-install", move || {
        let _lock = lock_data_dir(&data_dir)?;
        let paths = EnginePaths::under(data_dir);
        pos_engine::install::init_db(&paths.db, seed.as_deref())
            .map_err(|e| PosError::internal(format!("install_init_db: {e:#}")))
    })
    .await
}

/// What the engine would refuse in the provisioning bundle's `pos.json` (the absolute path
/// `config_bundle`), or why it cannot be loaded; empty when it is usable. Relative paths inside
/// it are resolved against its directory. Each problem is a coded error `config_<setting>|detail=<English
/// text>` (FFI.md section 9): the screen says it in its language and keeps the text as detail.
#[uniffi::export]
pub async fn check_config_bundle(config_bundle: String) -> Result<Vec<String>, PosError> {
    let path = absolute("check_config_bundle", "config_bundle", &config_bundle)?;
    on_thread("pos-ffi-install", move || {
        Ok(pos_engine::install::check_config_source_coded(
            &ConfigSource::Bundle(path),
        ))
    })
    .await
}
