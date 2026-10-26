//! Export settings: the `export` section of `pos.json`, checked and parsed.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use pos_core::config::ExportConfig;

/// The export configuration, ready to use.
#[derive(Clone)]
pub struct ExportSettings {
    /// Endpoint URL without a trailing `/`.
    pub url: String,
    /// File holding the bearer token (read at each upload, never cached).
    pub token_file: PathBuf,
    /// Recipients of the backup: the owner first, then the producer when
    /// configured.
    pub recipients: Vec<age::x25519::Recipient>,
    /// Days the exports stay in the outbox.
    pub keep_days: i64,
}

impl ExportSettings {
    /// Check and parse a configuration section. The error lists every
    /// problem found.
    pub fn from_config(cfg: &ExportConfig) -> Result<ExportSettings, String> {
        let problems = cfg.problems();
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
        let owner = age::x25519::Recipient::from_str(&cfg.owner_recipient)
            .map_err(|e| format!("export.owner_recipient: {e}"))?;
        let mut recipients = vec![owner];
        if let Some(p) = &cfg.producer_recipient {
            recipients.push(
                age::x25519::Recipient::from_str(p)
                    .map_err(|e| format!("export.producer_recipient: {e}"))?,
            );
        }
        Ok(ExportSettings {
            url: cfg.url.trim_end_matches('/').to_string(),
            token_file: PathBuf::from(&cfg.token_file),
            recipients,
            keep_days: i64::from(cfg.keep_days),
        })
    }

    /// `scheme://host[:port]` of the endpoint, without any path.
    pub fn endpoint(&self) -> String {
        match self.url.split_once("://") {
            Some((scheme, rest)) => {
                let authority = rest.split('/').next().unwrap_or(rest);
                format!("{scheme}://{authority}")
            }
            None => self.url.clone(),
        }
    }

    /// The recipients as `age1...` strings, in order, for the manifest.
    pub fn recipient_strings(&self) -> Vec<String> {
        self.recipients
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    }

    /// The bearer token, read from its file at each call and trimmed.
    /// The error never holds the token.
    pub fn read_token(&self) -> Result<String, String> {
        let raw = std::fs::read_to_string(&self.token_file).map_err(|e| {
            format!(
                "export token file {} unreadable: {}",
                self.token_file.display(),
                e.kind()
            )
        })?;
        let token = raw.trim();
        if token.is_empty() {
            return Err(format!(
                "export token file {} is empty",
                self.token_file.display()
            ));
        }
        Ok(token.to_string())
    }
}

// By hand: no secret may reach a log through `{:?}`.
impl fmt::Debug for ExportSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExportSettings")
            .field("url", &self.url)
            .field("token_file", &self.token_file)
            .field("recipients", &self.recipient_strings())
            .field("keep_days", &self.keep_days)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(dir: &std::path::Path) -> (ExportConfig, String, String) {
        let (_, owner) = crate::crypt::generate_identity();
        let (_, producer) = crate::crypt::generate_identity();
        (
            ExportConfig {
                url: "https://export.example.com:8443/v1/inst-42/".into(),
                token_file: dir.join("export.token").to_string_lossy().into_owned(),
                owner_recipient: owner.clone(),
                producer_recipient: Some(producer.clone()),
                keep_days: 30,
            },
            owner,
            producer,
        )
    }

    #[test]
    fn settings_are_parsed_from_the_config() {
        let dir = tempfile::tempdir().unwrap();
        let (c, owner, producer) = cfg(dir.path());
        let s = ExportSettings::from_config(&c).unwrap();
        assert_eq!(s.url, "https://export.example.com:8443/v1/inst-42");
        assert_eq!(s.endpoint(), "https://export.example.com:8443");
        assert_eq!(s.keep_days, 30);
        assert_eq!(s.recipient_strings(), vec![owner, producer]);
        assert_eq!(s.token_file, dir.path().join("export.token"));
    }

    #[test]
    fn the_producer_is_optional() {
        let dir = tempfile::tempdir().unwrap();
        let (mut c, owner, _) = cfg(dir.path());
        c.producer_recipient = None;
        c.url = "http://10.0.2.2:9300".into();
        let s = ExportSettings::from_config(&c).unwrap();
        assert_eq!(s.recipient_strings(), vec![owner]);
        assert_eq!(s.endpoint(), "http://10.0.2.2:9300");
        assert_eq!(s.url, "http://10.0.2.2:9300");
    }

    #[test]
    fn a_bad_section_is_refused_with_every_problem() {
        let dir = tempfile::tempdir().unwrap();
        let (mut c, _, _) = cfg(dir.path());
        c.url = "http://example.com".into();
        c.keep_days = 0;
        let err = ExportSettings::from_config(&c).unwrap_err();
        assert!(err.contains("export.url: "), "{err}");
        assert!(err.contains("export.keep_days: "), "{err}");
    }

    #[test]
    fn the_token_is_read_at_each_call_and_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        let (c, _, _) = cfg(dir.path());
        let s = ExportSettings::from_config(&c).unwrap();
        std::fs::write(&s.token_file, "  tok-one\n").unwrap();
        assert_eq!(s.read_token().unwrap(), "tok-one");
        std::fs::write(&s.token_file, "tok-two").unwrap();
        assert_eq!(s.read_token().unwrap(), "tok-two");
    }

    #[test]
    fn a_missing_or_empty_token_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let (c, _, _) = cfg(dir.path());
        let s = ExportSettings::from_config(&c).unwrap();
        assert!(s.read_token().is_err());
        std::fs::write(&s.token_file, " \n\t").unwrap();
        assert!(s.read_token().unwrap_err().contains("empty"));
    }

    #[test]
    fn the_token_never_reaches_an_error_or_debug_output() {
        let dir = tempfile::tempdir().unwrap();
        let (c, _, _) = cfg(dir.path());
        let s = ExportSettings::from_config(&c).unwrap();
        std::fs::write(&s.token_file, "s3cr3t-token-value").unwrap();
        let shown = format!("{s:?}");
        assert!(!shown.contains("s3cr3t-token-value"));
        assert!(shown.contains("keep_days"));
        std::fs::write(&s.token_file, "").unwrap();
        assert!(!s.read_token().unwrap_err().contains("s3cr3t"));
    }
}
