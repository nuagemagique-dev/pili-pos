//! Whole-catalog validation. Every issue is collected, not just the
//! first, so the owner can fix the source in one go. A catalog with
//! any issue is never applied: the previous menu stays in place.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{Catalog, ItemKind, Names, fiscal_name};

/// The languages the POS stores names in (`languages` table).
pub const LANGUAGES: &[&str] = &["fr", "nl", "en", "zh", "de"];

/// The FDM limit on `productName` (POS description v2.2, "longueur
/// min. 1 et max. 600 caractères").
pub const NAME_MAX_CHARS: usize = 600;

/// One problem of a catalog, with the key of the element concerned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub code: IssueCode,
    /// The key of the category, item, group or option at fault; empty
    /// for the catalog itself.
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCode {
    EmptyKey,
    DuplicateKey,
    UnknownCategory,
    MissingName,
    NameTooLong,
    UnknownLanguage,
    NegativePrice,
    MinAboveMax,
    NothingToSell,
    // Found while a source maps its own format.
    /// A price the source format cannot express in euro cents.
    BadPrice,
    /// A VAT rate other than 21, 12, 6 or 0 %.
    UnknownVatRate,
    /// Delivered and collected at different VAT rates: the POS has one
    /// takeaway label.
    VatMismatch,
    /// Neither a VAT rate nor a VAT category.
    MissingVat,
    /// An option list the catalog does not define.
    UnknownOptionList,
}

impl Issue {
    pub fn new(code: IssueCode, key: &str) -> Self {
        Self {
            code,
            key: key.to_owned(),
        }
    }
}

/// Every issue of `catalog`; empty when it can be applied.
pub fn validate(catalog: &Catalog) -> Vec<Issue> {
    let mut issues = Vec::new();

    let mut category_keys = HashSet::new();
    for cat in &catalog.categories {
        check_key(&cat.key, &mut category_keys, &mut issues);
        check_names(&cat.key, &cat.names, &mut issues);
    }

    // Items, groups and options live in one key space each; a group
    // or option key may repeat an item key.
    let mut item_keys = HashSet::new();
    let mut group_keys = HashSet::new();
    let mut option_keys = HashSet::new();
    for item in &catalog.items {
        check_key(&item.key, &mut item_keys, &mut issues);
        check_names(&item.key, &item.names, &mut issues);
        if !category_keys.contains(item.category_key.as_str()) {
            issues.push(Issue::new(IssueCode::UnknownCategory, &item.key));
        }
        if item.price_cents < 0 {
            issues.push(Issue::new(IssueCode::NegativePrice, &item.key));
        }
        for group in &item.groups {
            check_key(&group.key, &mut group_keys, &mut issues);
            check_names(&group.key, &group.names, &mut issues);
            if group.max.is_some_and(|max| group.min > max) {
                issues.push(Issue::new(IssueCode::MinAboveMax, &group.key));
            }
            for choice in &group.options {
                check_key(&choice.key, &mut option_keys, &mut issues);
                check_names(&choice.key, &choice.names, &mut issues);
                if choice.price_cents < 0 {
                    issues.push(Issue::new(IssueCode::NegativePrice, &choice.key));
                }
            }
        }
    }

    let sellable = catalog
        .items
        .iter()
        .any(|i| i.kind == ItemKind::Product && i.available);
    if !sellable {
        issues.push(Issue::new(IssueCode::NothingToSell, ""));
    }
    issues
}

fn check_key<'a>(key: &'a str, seen: &mut HashSet<&'a str>, issues: &mut Vec<Issue>) {
    if key.trim().is_empty() {
        issues.push(Issue::new(IssueCode::EmptyKey, key));
    } else if !seen.insert(key) {
        issues.push(Issue::new(IssueCode::DuplicateKey, key));
    }
}

fn check_names(key: &str, names: &Names, issues: &mut Vec<Issue>) {
    if fiscal_name(names).is_none() {
        issues.push(Issue::new(IssueCode::MissingName, key));
    }
    if names.values().any(|n| n.chars().count() > NAME_MAX_CHARS) {
        issues.push(Issue::new(IssueCode::NameTooLong, key));
    }
    if names.keys().any(|lang| !LANGUAGES.contains(&lang.as_str())) {
        issues.push(Issue::new(IssueCode::UnknownLanguage, key));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{catalog, choice, group, item};

    fn codes(c: &Catalog) -> Vec<(IssueCode, String)> {
        validate(c).into_iter().map(|i| (i.code, i.key)).collect()
    }

    #[test]
    fn the_sample_catalog_is_valid() {
        assert!(validate(&catalog()).is_empty());
    }

    #[test]
    fn a_duplicate_or_empty_key_is_reported() {
        let mut c = catalog();
        let dup = c.items[0].clone();
        c.items.push(dup);
        c.items[1].key = " ".into();
        let found = codes(&c);
        assert!(found.contains(&(IssueCode::DuplicateKey, c.items[0].key.clone())));
        assert!(found.contains(&(IssueCode::EmptyKey, " ".into())));
    }

    #[test]
    fn an_item_in_an_unknown_category_is_reported() {
        let mut c = catalog();
        c.items[0].category_key = "nowhere".into();
        assert_eq!(
            codes(&c),
            vec![(IssueCode::UnknownCategory, c.items[0].key.clone())]
        );
    }

    #[test]
    fn names_must_exist_fit_the_fdm_and_use_known_languages() {
        let mut c = catalog();
        c.items[0].names.clear();
        c.items[1]
            .names
            .insert("fr".into(), "x".repeat(NAME_MAX_CHARS + 1));
        c.categories[0].names.insert("xx".into(), "?".into());
        let found = codes(&c);
        assert!(found.contains(&(IssueCode::MissingName, c.items[0].key.clone())));
        assert!(found.contains(&(IssueCode::NameTooLong, c.items[1].key.clone())));
        assert!(found.contains(&(IssueCode::UnknownLanguage, c.categories[0].key.clone())));
    }

    #[test]
    fn a_name_of_600_characters_is_accepted() {
        let mut c = catalog();
        c.items[0]
            .names
            .insert("fr".into(), "é".repeat(NAME_MAX_CHARS));
        assert!(validate(&c).is_empty());
    }

    #[test]
    fn negative_prices_are_reported() {
        let mut c = catalog();
        c.items[0].price_cents = -1;
        c.items[0].groups = vec![group("g", 0, Some(1), vec![choice("o", -5)])];
        let found = codes(&c);
        assert!(found.contains(&(IssueCode::NegativePrice, c.items[0].key.clone())));
        assert!(found.contains(&(IssueCode::NegativePrice, "o".into())));
    }

    #[test]
    fn a_minimum_above_the_maximum_is_reported() {
        let mut c = catalog();
        c.items[0].groups = vec![group("g", 3, Some(2), vec![choice("o", 0)])];
        assert_eq!(codes(&c), vec![(IssueCode::MinAboveMax, "g".into())]);
        c.items[0].groups = vec![group("g", 3, None, vec![choice("o", 0)])];
        assert!(validate(&c).is_empty(), "no maximum");
    }

    #[test]
    fn a_catalog_with_nothing_to_sell_is_refused() {
        let mut c = catalog();
        for i in &mut c.items {
            i.available = false;
        }
        assert_eq!(codes(&c), vec![(IssueCode::NothingToSell, String::new())]);
        let only_charge = Catalog {
            categories: c.categories.clone(),
            items: vec![item("fee", &c.categories[0].key, ItemKind::Charge, 250)],
        };
        assert!(codes(&only_charge).contains(&(IssueCode::NothingToSell, String::new())));
    }

    #[test]
    fn every_issue_is_collected() {
        let mut c = catalog();
        c.items[0].price_cents = -1;
        c.items[1].category_key = "nowhere".into();
        assert_eq!(validate(&c).len(), 2);
    }
}
