//! The table plan: which table numbers the restaurant uses, written as
//! ranges ("1-12, 20-25"), optionally grouped in zones ("A 1-20, B 21-40").
//! `pos.json` gives the starting plan (`tables`, or 1 to `table_count`); a
//! manager can change it in the settings.
//!
//! A zone is one or two letters in front of ranges. It names where a table
//! is, never which table it is: the table is always its number, which stays
//! unique across zones. The zone only changes how the table is written on
//! screen and on the kitchen slips ([`label`]: "A-19"); the table
//! identifier of every signed request and of the fiscal ticket is the
//! number alone.
//!
//! A letter applies to its own range and to the ranges after it that have
//! no letter, up to the next letter: "A 1-10, 15-20, B 21-30" puts 1-10 and
//! 15-20 in A. Ranges before the first letter have no zone. The canonical
//! form ([`format_zoned`]) writes the zone-less ranges first, then each zone
//! in alphabetical order.

use std::collections::BTreeMap;

/// Highest table number.
pub const MAX_TABLE_NUMBER: u32 = 999;
/// Most tables on one plan.
pub const MAX_TABLES: usize = 200;
/// Most letters of a zone.
pub const MAX_ZONE_LETTERS: usize = 2;
/// Most characters of a zone name.
pub const MAX_ZONE_NAME_CHARS: usize = 20;

/// One table of the plan.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlanTable {
    pub number: u32,
    /// The zone letters ("A"), `None` on a plan without letters.
    pub zone: Option<String>,
}

/// The table numbers of `text`, ascending and without duplicates (the
/// zones, if any, are left out: see [`parse_zoned`]).
pub fn parse(text: &str) -> Result<Vec<u32>, String> {
    parse_zoned(text).map(|tables| tables.into_iter().map(|t| t.number).collect())
}

/// The tables of `text`, ascending by number and without duplicates.
/// `text` is a comma-separated list of numbers ("7") and ranges ("1-12"),
/// each from 1 to [`MAX_TABLE_NUMBER`], each range optionally preceded by
/// the letters of its zone ("A 1-20", "A1-20"), with at most
/// [`MAX_TABLES`] tables in all. A number is in one zone only. The error
/// says what is wrong, in English, for the log and the configuration
/// check.
pub fn parse_zoned(text: &str) -> Result<Vec<PlanTable>, String> {
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    let mut zone: Option<String> = None;
    for part in text.split(',') {
        let mut part = part.trim();
        if part.is_empty() {
            return Err(format!("empty entry in {text:?}"));
        }
        let letters = part.chars().take_while(char::is_ascii_alphabetic).count();
        if letters > 0 {
            if letters > MAX_ZONE_LETTERS {
                return Err(format!(
                    "zone {:?} has more than {MAX_ZONE_LETTERS} letters",
                    &part[..letters]
                ));
            }
            zone = Some(part[..letters].to_ascii_uppercase());
            part = part[letters..].trim();
            if part.is_empty() {
                return Err(format!("zone {:?} has no table", zone.unwrap_or_default()));
            }
        }
        let (from, to) = if let Some((a, b)) = part.split_once('-') {
            (number(a)?, number(b)?)
        } else {
            let n = number(part)?;
            (n, n)
        };
        if from > to {
            return Err(format!("range {part:?} goes down"));
        }
        if usize::try_from(to - from).map_or(true, |span| span >= MAX_TABLES) {
            return Err(format!("more than {MAX_TABLES} tables"));
        }
        for n in from..=to {
            match found.get(&n) {
                Some(seen) if *seen != zone => {
                    return Err(format!(
                        "table {n} is in two zones ({} and {})",
                        zone_text(seen.as_deref()),
                        zone_text(zone.as_deref())
                    ));
                }
                _ => {
                    found.insert(n, zone.clone());
                }
            }
        }
        if found.len() > MAX_TABLES {
            return Err(format!("more than {MAX_TABLES} tables"));
        }
    }
    Ok(found
        .into_iter()
        .map(|(number, zone)| PlanTable { number, zone })
        .collect())
}

fn zone_text(zone: Option<&str>) -> String {
    zone.map_or_else(|| "no zone".to_string(), |z| format!("zone {z}"))
}

fn number(s: &str) -> Result<u32, String> {
    let s = s.trim();
    match s.parse::<u32>() {
        Ok(n) if (1..=MAX_TABLE_NUMBER).contains(&n) => Ok(n),
        _ => Err(format!(
            "{s:?} is not a table number (1 to {MAX_TABLE_NUMBER})"
        )),
    }
}

/// `numbers` (ascending, no duplicates) written back as ranges, the
/// canonical form that is stored and shown: `[1, 2, 3, 7]` is "1-3, 7".
pub fn format(numbers: &[u32]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < numbers.len() {
        let start = numbers[i];
        let mut end = start;
        while i + 1 < numbers.len() && numbers[i + 1] == end + 1 {
            i += 1;
            end = numbers[i];
        }
        out.push(if start == end {
            start.to_string()
        } else {
            format!("{start}-{end}")
        });
        i += 1;
    }
    out.join(", ")
}

/// `tables` written back, the canonical form: the ranges without a zone
/// first, then each zone in alphabetical order with its letters in front
/// of its first range ("1-5, A 6-10, 15, B 21-30").
pub fn format_zoned(tables: &[PlanTable]) -> String {
    let mut groups: BTreeMap<Option<&str>, Vec<u32>> = BTreeMap::new();
    for t in tables {
        groups.entry(t.zone.as_deref()).or_default().push(t.number);
    }
    let mut out: Vec<String> = Vec::new();
    for (zone, mut numbers) in groups {
        numbers.sort_unstable();
        numbers.dedup();
        let text = format(&numbers);
        match zone {
            None => out.push(text),
            Some(z) => out.push(format!("{z} {text}")),
        }
    }
    out.join(", ")
}

/// `text` in its canonical form ("20-25,1-12" is "1-12, 20-25",
/// "b 21-30, a1-20" is "A 1-20, B 21-30").
pub fn normalize(text: &str) -> Result<String, String> {
    parse_zoned(text).map(|t| format_zoned(&t))
}

/// How a table is written on screen and on the kitchen slips: "A-19" in a
/// zone, "19" without one. Never used in a signed request.
pub fn label(zone: Option<&str>, number: i64) -> String {
    match zone {
        Some(z) => format!("{z}-{number}"),
        None => number.to_string(),
    }
}

/// The zone names of `text`: comma-separated "A · Salle" entries (the
/// letters, then " · ", ":", "-" or just a space, then the name of at most
/// [`MAX_ZONE_NAME_CHARS`] characters), as `(letters, name)` sorted by
/// letters. A letter named twice is an error; an empty text names nothing.
pub fn parse_zone_names(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    for part in text.split(',') {
        let part = part.trim();
        let letters = part.chars().take_while(char::is_ascii_alphabetic).count();
        if letters == 0 || letters > MAX_ZONE_LETTERS {
            return Err(format!("{part:?} does not start with a zone letter"));
        }
        let zone = part[..letters].to_ascii_uppercase();
        let name = part[letters..]
            .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '\u{b7}' | ':' | '-'))
            .trim();
        if name.is_empty() {
            // A letter alone names nothing.
            continue;
        }
        if name.chars().count() > MAX_ZONE_NAME_CHARS || name.chars().any(char::is_control) {
            return Err(format!(
                "the name of zone {zone} is longer than {MAX_ZONE_NAME_CHARS} characters"
            ));
        }
        if names.insert(zone.clone(), name.to_string()).is_some() {
            return Err(format!("zone {zone} is named twice"));
        }
    }
    Ok(names.into_iter().collect())
}

/// `names` written back, the canonical form: "A · Salle, B · Terrasse".
pub fn format_zone_names(names: &[(String, String)]) -> String {
    names
        .iter()
        .map(|(zone, name)| format!("{zone} \u{b7} {name}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_single_numbers_are_read_in_order_without_duplicates() {
        assert_eq!(parse("1-3, 7").unwrap(), vec![1, 2, 3, 7]);
        assert_eq!(parse(" 20-22 ,1-2,21 ").unwrap(), vec![1, 2, 20, 21, 22]);
        assert_eq!(parse("5").unwrap(), vec![5]);
        assert_eq!(parse("4-4").unwrap(), vec![4]);
    }

    #[test]
    fn a_wrong_plan_says_why() {
        for bad in [
            "", " ", "1,,2", "0", "1000", "a", "1-", "-3", "5-2", "1-300", "1.5",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} must be refused");
        }
        assert!(parse("1-150, 200-260").unwrap_err().contains("200"));
        assert_eq!(parse("1-200").unwrap().len(), 200);
    }

    #[test]
    fn the_canonical_form_groups_consecutive_numbers() {
        assert_eq!(format(&[1, 2, 3, 7, 9, 10]), "1-3, 7, 9-10");
        assert_eq!(format(&[]), "");
        assert_eq!(normalize("20-25,1-12").unwrap(), "1-12, 20-25");
        assert_eq!(normalize("3,1,2").unwrap(), "1-3");
    }

    fn zone_of(tables: &[PlanTable], number: u32) -> Option<&str> {
        tables
            .iter()
            .find(|t| t.number == number)
            .and_then(|t| t.zone.as_deref())
    }

    #[test]
    fn a_plan_without_letters_has_no_zone_and_reads_as_before() {
        let tables = parse_zoned("1-3, 7").unwrap();
        assert!(tables.iter().all(|t| t.zone.is_none()));
        assert_eq!(parse("1-3, 7").unwrap(), vec![1, 2, 3, 7]);
        assert_eq!(normalize("7, 1-3").unwrap(), "1-3, 7");
        assert_eq!(label(None, 19), "19");
    }

    #[test]
    fn a_letter_names_the_zone_of_the_ranges_after_it() {
        let tables = parse_zoned("A 1-20, B 21-40").unwrap();
        assert_eq!(tables.len(), 40);
        assert_eq!(zone_of(&tables, 1), Some("A"));
        assert_eq!(zone_of(&tables, 20), Some("A"));
        assert_eq!(zone_of(&tables, 21), Some("B"));
        assert_eq!(zone_of(&tables, 40), Some("B"));
        // The numbers come back ascending, whatever the zones.
        assert_eq!(
            parse("A 1-20, B 21-40").unwrap(),
            (1..=40).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_letter_may_touch_the_number_be_lower_case_or_have_two_letters() {
        for text in ["A1-20", "a 1-20", "A  1-20", " A 1-20 "] {
            let tables = parse_zoned(text).unwrap();
            assert_eq!(tables.len(), 20, "{text:?}");
            assert_eq!(zone_of(&tables, 5), Some("A"), "{text:?}");
        }
        let two = parse_zoned("TE 1-4, ba 5-6").unwrap();
        assert_eq!(zone_of(&two, 3), Some("TE"));
        assert_eq!(zone_of(&two, 5), Some("BA"));
    }

    #[test]
    fn ranges_without_a_letter_follow_the_last_zone_and_come_first_without_one() {
        let tables = parse_zoned("1-2, A 3-4, 9, B 10").unwrap();
        assert_eq!(zone_of(&tables, 1), None);
        assert_eq!(zone_of(&tables, 2), None);
        assert_eq!(zone_of(&tables, 3), Some("A"));
        assert_eq!(zone_of(&tables, 9), Some("A"));
        assert_eq!(zone_of(&tables, 10), Some("B"));
    }

    #[test]
    fn a_number_is_unique_across_zones() {
        // The same table twice in one zone is fine, in two zones it is not.
        assert_eq!(parse_zoned("A 1-5, 3-7").unwrap().len(), 7);
        let err = parse_zoned("A 1-20, B 20-30").unwrap_err();
        assert!(err.contains("20") && err.contains("zone A") && err.contains("zone B"));
        assert!(
            parse_zoned("1-5, A 5-9").is_err(),
            "a table with and without a zone"
        );
    }

    #[test]
    fn a_wrong_zone_is_refused() {
        for bad in [
            "A",
            "A ,1",
            "ABC 1-4",
            "A 0",
            "A 1-",
            "1 A",
            "A- 3",
            "A 5-2",
            "A 1-150, B 200-260",
            "3 A 4",
        ] {
            assert!(parse_zoned(bad).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn the_canonical_form_puts_the_plain_ranges_first_then_the_zones_in_order() {
        assert_eq!(normalize("b 21-30, a1-20").unwrap(), "A 1-20, B 21-30");
        assert_eq!(
            normalize("1-5, A 6-10, 15, B 21-30").unwrap(),
            "1-5, A 6-10, 15, B 21-30"
        );
        // A zone split in two ranges keeps its letters once.
        assert_eq!(
            normalize("A 1-3, B 4-5, A 7-8").unwrap(),
            "A 1-3, 7-8, B 4-5"
        );
        // Reading the canonical form gives the same plan back.
        let plan = parse_zoned("TE 30-31, A 1-3, 5").unwrap();
        assert_eq!(parse_zoned(&format_zoned(&plan)).unwrap(), plan);
    }

    #[test]
    fn a_table_is_written_with_its_zone_and_its_number() {
        assert_eq!(label(Some("A"), 19), "A-19");
        assert_eq!(label(Some("TE"), 3), "TE-3");
        // A and B can both have a table 19 on paper in XudoX; here the number is unique.
        assert_ne!(label(Some("A"), 19), label(Some("B"), 30));
    }

    #[test]
    fn zone_names_are_read_in_order_and_written_back() {
        let names = parse_zone_names("B : Terrasse, a \u{b7} Salle").unwrap();
        assert_eq!(
            names,
            vec![
                ("A".to_string(), "Salle".to_string()),
                ("B".to_string(), "Terrasse".to_string())
            ]
        );
        assert_eq!(
            format_zone_names(&names),
            "A \u{b7} Salle, B \u{b7} Terrasse"
        );
        assert_eq!(parse_zone_names("A-Salle, B Terrasse").unwrap().len(), 2);
        assert_eq!(parse_zone_names("").unwrap().len(), 0);
        assert!(
            parse_zone_names("A").unwrap().is_empty(),
            "a letter alone names nothing"
        );
    }

    #[test]
    fn a_wrong_zone_name_is_refused() {
        for bad in [
            "Salle",
            "1 Salle",
            "A Salle, A Bar",
            "ABC Salle",
            "A aaaaaaaaaaaaaaaaaaaaa",
        ] {
            assert!(parse_zone_names(bad).is_err(), "{bad:?} must be refused");
        }
    }
}
