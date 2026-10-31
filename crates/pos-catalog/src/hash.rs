//! A stable fingerprint of a catalog: the drift check compares it with
//! the last applied one instead of rewriting the menu.

use sha2::{Digest, Sha256};

use std::fmt::Write as _;

use crate::model::Catalog;

/// Hex SHA-256 of the catalog's JSON. Names are sorted maps, so the
/// hash depends only on content and order of elements.
pub fn hash(catalog: &Catalog) -> String {
    let json = serde_json::to_vec(catalog).expect("a catalog always serializes");
    Sha256::digest(&json)
        .iter()
        .fold(String::with_capacity(64), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::catalog;

    #[test]
    fn the_same_catalog_has_the_same_hash() {
        assert_eq!(hash(&catalog()), hash(&catalog()));
        assert_eq!(hash(&catalog()).len(), 64);
    }

    #[test]
    fn a_price_change_changes_the_hash() {
        let mut c = catalog();
        let before = hash(&c);
        c.items[0].price_cents += 1;
        assert_ne!(before, hash(&c));
    }

    #[test]
    fn the_order_of_translations_does_not_matter() {
        let mut a = catalog();
        let mut b = catalog();
        a.items[0].names.insert("zh".into(), "卷".into());
        a.items[0].names.insert("en".into(), "Roll".into());
        b.items[0].names.insert("en".into(), "Roll".into());
        b.items[0].names.insert("zh".into(), "卷".into());
        assert_eq!(hash(&a), hash(&b));
    }
}
