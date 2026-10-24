//! Filesystem locations of the POS runtime data (database, logs,
//! backups).
//!
//! The data directory is resolved, in priority order, from:
//!
//! 1. `PILI_POS_DATA_DIR` - explicit override (image build, tests).
//! 2. `STATE_DIRECTORY` - exported by systemd for `StateDirectory=pili-pos`
//!    in `deploy/pili-pos.service`, i.e. `/var/lib/pili-pos`: the path
//!    declared to SPF Finances for data acquisition
//!    (`FEUILLE_ACQUISITION_DONNEES_FR.md`).
//! 3. `dirs::data_dir()/pili-pos` - development machines.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Directory holding `pos.db`, `logs/` and `backups/`.
/// The configuration of an installed terminal (installation manual).
pub const PRODUCTION_CONFIG: &str = "/etc/pili-pos/pos.json";

/// Where the image installs RustDesk (`deploy/provision/provision.sh`).
pub const REMOTE_SUPPORT_PROGRAM: &str = "/opt/pili-pos/rustdesk/AppRun";

/// The remote support program the engine is given: RustDesk as installed
/// on the kiosk, when it is there.
pub fn support_program() -> Option<PathBuf> {
    support_program_at(Path::new(REMOTE_SUPPORT_PROGRAM))
}

fn support_program_at(program: &Path) -> Option<PathBuf> {
    program.exists().then(|| program.to_path_buf())
}

pub fn data_dir() -> PathBuf {
    resolve_data_dir(
        std::env::var_os("PILI_POS_DATA_DIR"),
        std::env::var_os("STATE_DIRECTORY"),
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pili-pos"),
    )
}

pub fn db_path() -> PathBuf {
    data_dir().join("pos.db")
}

pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn backups_dir() -> PathBuf {
    data_dir().join("backups")
}

fn resolve_data_dir(
    data_dir_override: Option<OsString>,
    state_directory: Option<OsString>,
    fallback: PathBuf,
) -> PathBuf {
    if let Some(dir) = data_dir_override.filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    // systemd joins several StateDirectory= entries with ':'; the unit
    // declares exactly one, so the first entry is ours.
    if let Some(first) = state_directory
        .as_ref()
        .and_then(|d| d.to_str())
        .and_then(|d| d.split(':').next())
        .filter(|d| !d.is_empty())
    {
        return PathBuf::from(first);
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fallback() -> PathBuf {
        PathBuf::from("/home/dev/.local/share/pili-pos")
    }

    #[test]
    fn the_support_program_is_given_only_when_it_exists() {
        let dir = std::env::temp_dir().join(format!("pili-paths-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let program = dir.join("AppRun");
        assert_eq!(support_program_at(&program), None);
        std::fs::write(&program, "").unwrap();
        assert_eq!(support_program_at(&program), Some(program.clone()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn override_wins_over_systemd() {
        let dir = resolve_data_dir(
            Some("/tmp/image-build".into()),
            Some("/var/lib/pili-pos".into()),
            fallback(),
        );
        assert_eq!(dir, PathBuf::from("/tmp/image-build"));
    }

    #[test]
    fn systemd_state_directory_is_used_in_production() {
        let dir = resolve_data_dir(None, Some("/var/lib/pili-pos".into()), fallback());
        assert_eq!(dir, PathBuf::from("/var/lib/pili-pos"));
    }

    #[test]
    fn first_of_several_state_directories() {
        let dir = resolve_data_dir(
            None,
            Some("/var/lib/pili-pos:/var/lib/other".into()),
            fallback(),
        );
        assert_eq!(dir, PathBuf::from("/var/lib/pili-pos"));
    }

    #[test]
    fn empty_values_fall_back_to_user_data_dir() {
        let dir = resolve_data_dir(Some("".into()), Some("".into()), fallback());
        assert_eq!(dir, fallback());
    }

    #[test]
    fn the_engine_layout_is_the_production_layout() {
        let paths = pos_engine::engine::EnginePaths::under(data_dir());
        assert_eq!(paths.db, db_path());
        assert_eq!(paths.backups, backups_dir());
        assert_eq!(paths.data_dir, data_dir());
    }
}
