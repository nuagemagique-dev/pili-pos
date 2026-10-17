//! The 14 allergens of Regulation (EU) No 1169/2011, Annex II, in
//! Annex order (v1.2, decision V12-D17).
//!
//! Non-fiscal data: it is displayed and printed, never signed and
//! never sent to the FDM. A product either has a declared list (which
//! may be empty: "no allergen") or no information at all; the two must
//! never be confused, so "declared" is a separate fact from the list.

use serde::{Deserialize, Serialize};

/// One of the 14 substances of Annex II.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Allergen {
    Gluten,
    Crustaceans,
    Eggs,
    Fish,
    Peanuts,
    Soybeans,
    Milk,
    Nuts,
    Celery,
    Mustard,
    Sesame,
    Sulphites,
    Lupin,
    Molluscs,
}

/// Every allergen, in Annex order (number 1 to 14).
pub const ALL: [Allergen; 14] = [
    Allergen::Gluten,
    Allergen::Crustaceans,
    Allergen::Eggs,
    Allergen::Fish,
    Allergen::Peanuts,
    Allergen::Soybeans,
    Allergen::Milk,
    Allergen::Nuts,
    Allergen::Celery,
    Allergen::Mustard,
    Allergen::Sesame,
    Allergen::Sulphites,
    Allergen::Lupin,
    Allergen::Molluscs,
];

/// Names per language: fr, nl, en, zh, de.
const NAMES: [[&str; 5]; 14] = [
    [
        "Céréales contenant du gluten",
        "Glutenbevattende granen",
        "Cereals containing gluten",
        "含麸质的谷物",
        "Glutenhaltiges Getreide",
    ],
    [
        "Crustacés",
        "Schaaldieren",
        "Crustaceans",
        "甲壳类",
        "Krebstiere",
    ],
    ["Œufs", "Eieren", "Eggs", "蛋", "Eier"],
    ["Poissons", "Vis", "Fish", "鱼", "Fisch"],
    ["Arachides", "Pinda's", "Peanuts", "花生", "Erdnüsse"],
    ["Soja", "Soja", "Soybeans", "大豆", "Soja"],
    ["Lait", "Melk", "Milk", "奶", "Milch"],
    ["Fruits à coque", "Noten", "Nuts", "坚果", "Schalenfrüchte"],
    ["Céleri", "Selderij", "Celery", "芹菜", "Sellerie"],
    ["Moutarde", "Mosterd", "Mustard", "芥末", "Senf"],
    ["Sésame", "Sesamzaad", "Sesame", "芝麻", "Sesam"],
    ["Sulfites", "Sulfiet", "Sulphites", "亚硫酸盐", "Sulfite"],
    ["Lupin", "Lupine", "Lupin", "羽扇豆", "Lupinen"],
    [
        "Mollusques",
        "Weekdieren",
        "Molluscs",
        "软体动物",
        "Weichtiere",
    ],
];

/// The languages a sheet can be printed in.
pub const SHEET_LANGUAGES: [&str; 4] = ["fr", "nl", "en", "de"];

impl Allergen {
    /// The Annex II number, 1 to 14.
    pub fn number(self) -> u8 {
        ALL.iter()
            .position(|a| *a == self)
            .map_or(0, |i| i as u8 + 1)
    }

    /// The stable code used in tags, the database and the IPC.
    pub fn code(self) -> &'static str {
        match self {
            Allergen::Gluten => "gluten",
            Allergen::Crustaceans => "crustaceans",
            Allergen::Eggs => "eggs",
            Allergen::Fish => "fish",
            Allergen::Peanuts => "peanuts",
            Allergen::Soybeans => "soybeans",
            Allergen::Milk => "milk",
            Allergen::Nuts => "nuts",
            Allergen::Celery => "celery",
            Allergen::Mustard => "mustard",
            Allergen::Sesame => "sesame",
            Allergen::Sulphites => "sulphites",
            Allergen::Lupin => "lupin",
            Allergen::Molluscs => "molluscs",
        }
    }

    /// The allergen with this code (exact, lower case).
    pub fn from_code(code: &str) -> Option<Allergen> {
        ALL.iter().copied().find(|a| a.code() == code)
    }

    /// The allergen with this Annex number.
    pub fn from_number(number: u8) -> Option<Allergen> {
        (1..=14)
            .contains(&number)
            .then(|| ALL[usize::from(number) - 1])
    }

    /// The name in `lang` (`fr`, `nl`, `en`, `zh`, `de`); any other
    /// language reads English.
    pub fn name(self, lang: &str) -> &'static str {
        let col = match lang {
            "fr" => 0,
            "nl" => 1,
            "zh" => 3,
            "de" => 4,
            _ => 2,
        };
        NAMES[usize::from(self.number()) - 1][col]
    }
}

/// An allergen list as a catalog or an editor gives it: what the item
/// contains and what it may contain as traces. Codes stay text so a
/// typo is reported (`UnknownAllergen`) instead of being lost.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllergenSet {
    #[serde(default)]
    pub contains: Vec<String>,
    #[serde(default)]
    pub may_contain: Vec<String>,
}

impl AllergenSet {
    /// Sorted, deduplicated codes; a code listed in both levels stays
    /// in `contains` only (the stricter statement wins).
    pub fn normalized(&self) -> AllergenSet {
        let key = |c: &String| {
            Allergen::from_code(c)
                .map(|a| (a.number(), String::new()))
                .unwrap_or((u8::MAX, c.clone()))
        };
        let mut contains = self.contains.clone();
        contains.sort_by_key(key);
        contains.dedup();
        let mut may_contain: Vec<String> = self
            .may_contain
            .iter()
            .filter(|c| !contains.contains(c))
            .cloned()
            .collect();
        may_contain.sort_by_key(key);
        may_contain.dedup();
        AllergenSet {
            contains,
            may_contain,
        }
    }

    /// The first code that is not one of the 14, if any.
    pub fn first_unknown(&self) -> Option<&str> {
        self.contains
            .iter()
            .chain(&self.may_contain)
            .map(String::as_str)
            .find(|c| Allergen::from_code(c).is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_14_in_annex_order_with_numbers_1_to_14() {
        assert_eq!(ALL.len(), 14);
        for (i, a) in ALL.iter().enumerate() {
            assert_eq!(usize::from(a.number()), i + 1);
            assert_eq!(Allergen::from_number(a.number()), Some(*a));
            assert_eq!(Allergen::from_code(a.code()), Some(*a));
        }
        assert_eq!(Allergen::Gluten.number(), 1);
        assert_eq!(Allergen::Molluscs.number(), 14);
        assert_eq!(Allergen::from_number(0), None);
        assert_eq!(Allergen::from_number(15), None);
    }

    #[test]
    fn codes_are_unique_and_match_the_serde_names() {
        let mut codes: Vec<&str> = ALL.iter().map(|a| a.code()).collect();
        for a in ALL {
            let json = serde_json::to_string(&a).unwrap();
            assert_eq!(json, format!("\"{}\"", a.code()));
        }
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 14);
    }

    #[test]
    fn every_allergen_has_a_name_in_five_languages() {
        for a in ALL {
            for lang in ["fr", "nl", "en", "zh", "de"] {
                assert!(!a.name(lang).is_empty(), "{} {lang}", a.code());
            }
        }
        assert_eq!(Allergen::Milk.name("fr"), "Lait");
        assert_eq!(Allergen::Milk.name("nl"), "Melk");
        assert_eq!(Allergen::Milk.name("en"), "Milk");
        assert_eq!(Allergen::Milk.name("zh"), "奶");
        assert_eq!(Allergen::Milk.name("de"), "Milch");
        assert_eq!(Allergen::Milk.name("xx"), "Milk");
        assert_eq!(Allergen::Gluten.name("fr"), "Céréales contenant du gluten");
    }

    #[test]
    fn an_unknown_code_is_not_an_allergen() {
        assert_eq!(Allergen::from_code("Gluten"), None);
        assert_eq!(Allergen::from_code("glutten"), None);
        let set = AllergenSet {
            contains: vec!["milk".into()],
            may_contain: vec!["nutz".into()],
        };
        assert_eq!(set.first_unknown(), Some("nutz"));
        assert_eq!(AllergenSet::default().first_unknown(), None);
    }

    #[test]
    fn normalizing_sorts_by_annex_order_and_keeps_the_stricter_level() {
        let set = AllergenSet {
            contains: vec!["milk".into(), "gluten".into(), "milk".into()],
            may_contain: vec!["sesame".into(), "milk".into(), "eggs".into()],
        };
        let n = set.normalized();
        assert_eq!(n.contains, vec!["gluten", "milk"]);
        assert_eq!(n.may_contain, vec!["eggs", "sesame"]);
    }

    #[test]
    fn a_set_rejects_unknown_fields_and_defaults_missing_ones() {
        let s: AllergenSet = serde_json::from_str(r#"{"contains":["fish"]}"#).unwrap();
        assert!(s.may_contain.is_empty());
        assert!(serde_json::from_str::<AllergenSet>(r#"{"x":1}"#).is_err());
    }
}
