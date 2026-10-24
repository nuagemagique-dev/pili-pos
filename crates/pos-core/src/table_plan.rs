//! The table plan: which table numbers the restaurant uses, written as
//! ranges ("1-12, 20-25"). `pos.json` gives the starting plan (`tables`,
//! or 1 to `table_count`); a manager can change it in the settings.

/// Highest table number.
pub const MAX_TABLE_NUMBER: u32 = 999;
/// Most tables on one plan.
pub const MAX_TABLES: usize = 200;

/// The table numbers of `text`, ascending and without duplicates.
/// `text` is a comma-separated list of numbers ("7") and ranges
/// ("1-12"), each from 1 to [`MAX_TABLE_NUMBER`], with at most
/// [`MAX_TABLES`] tables in all. The error says what is wrong, in
/// English, for the log and the configuration check.
pub fn parse(text: &str) -> Result<Vec<u32>, String> {
    let mut numbers = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(format!("empty entry in {text:?}"));
        }
        let (from, to) = match part.split_once('-') {
            Some((a, b)) => (number(a)?, number(b)?),
            None => {
                let n = number(part)?;
                (n, n)
            }
        };
        if from > to {
            return Err(format!("range {part:?} goes down"));
        }
        if (to - from) as usize >= MAX_TABLES {
            return Err(format!("more than {MAX_TABLES} tables"));
        }
        numbers.extend(from..=to);
    }
    numbers.sort_unstable();
    numbers.dedup();
    if numbers.len() > MAX_TABLES {
        return Err(format!("more than {MAX_TABLES} tables"));
    }
    Ok(numbers)
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

/// `text` in its canonical form ("20-25,1-12" is "1-12, 20-25").
pub fn normalize(text: &str) -> Result<String, String> {
    parse(text).map(|n| format(&n))
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
}
