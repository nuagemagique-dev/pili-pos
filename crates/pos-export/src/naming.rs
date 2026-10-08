//! Names of the export files. The export id is `<YYYYMMDD>-z<6 digits>`.

use chrono::NaiveDate;

/// Largest Z report number that fits the six digits of the export id.
const MAX_Z: i64 = 999_999;

/// Export id of a Z report: `20261008-z000042` for the booking date
/// `2026-10-08` and the Z report number 42.
pub fn export_id(booking_date: &str, z_report_no: i64) -> Result<String, String> {
    let well_formed = booking_date.len() == 10
        && booking_date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    let date = NaiveDate::parse_from_str(booking_date, "%Y-%m-%d")
        .ok()
        .filter(|_| well_formed)
        .ok_or_else(|| format!("booking date {booking_date:?}: YYYY-MM-DD expected"))?;
    if !(0..=MAX_Z).contains(&z_report_no) {
        return Err(format!(
            "Z report number {z_report_no}: 0 to {MAX_Z} expected"
        ));
    }
    Ok(format!("{}-z{z_report_no:06}", date.format("%Y%m%d")))
}

/// `pos-<id>.db.age`: the encrypted backup.
pub fn backup_name(id: &str) -> String {
    format!("pos-{id}.db.age")
}

/// `stats-<id>.sqlite`: the statistics file.
pub fn stats_name(id: &str) -> String {
    format!("stats-{id}.sqlite")
}

/// `manifest-<id>.json`: the manifest.
pub fn manifest_name(id: &str) -> String {
    format!("manifest-{id}.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_the_compact_date_and_a_padded_z_number() {
        assert_eq!(export_id("2026-10-08", 42).unwrap(), "20261008-z000042");
        assert_eq!(export_id("2026-01-01", 0).unwrap(), "20260101-z000000");
        assert_eq!(
            export_id("2028-02-29", 999_999).unwrap(),
            "20280229-z999999"
        );
    }

    #[test]
    fn a_malformed_date_is_refused() {
        for bad in [
            "",
            "2026-10-8",
            "2026-1-08",
            "20261008",
            "2026/10/08",
            "2026-13-01",
            "2026-02-30",
            "2027-02-29",
            "2026-10-08 ",
            " 2026-10-08",
            "+026-10-08",
            "abcd-ef-gh",
        ] {
            assert!(export_id(bad, 1).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn a_z_number_out_of_range_is_refused() {
        assert!(export_id("2026-10-08", -1).is_err());
        assert!(export_id("2026-10-08", 1_000_000).is_err());
    }

    #[test]
    fn file_names_follow_the_id() {
        let id = export_id("2026-10-08", 42).unwrap();
        assert_eq!(backup_name(&id), "pos-20261008-z000042.db.age");
        assert_eq!(stats_name(&id), "stats-20261008-z000042.sqlite");
        assert_eq!(manifest_name(&id), "manifest-20261008-z000042.json");
    }
}
