//! Small catalogs for tests, here and in the crates that map a source
//! into the neutral model.

use pos_core::vat::VatCode;

use crate::model::{Catalog, Category, Choice, Group, Item, ItemKind, Names};

pub fn names(fr: &str) -> Names {
    let mut n = Names::new();
    n.insert("fr".into(), fr.into());
    n
}

pub fn category(key: &str) -> Category {
    Category {
        key: key.into(),
        names: names(key),
        visible: true,
    }
}

/// A food item: B eaten in, C taken away.
pub fn item(key: &str, category_key: &str, kind: ItemKind, price_cents: i64) -> Item {
    Item {
        key: key.into(),
        category_key: category_key.into(),
        kind,
        names: names(key),
        price_cents,
        vat_eat_in: VatCode::B,
        vat_takeaway: VatCode::C,
        open_price: false,
        kitchen: true,
        on_pos: kind == ItemKind::Product,
        available: true,
        groups: Vec::new(),
    }
}

pub fn group(key: &str, min: u32, max: Option<u32>, options: Vec<Choice>) -> Group {
    Group {
        key: key.into(),
        names: names(key),
        min,
        max,
        options,
    }
}

pub fn choice(key: &str, price_cents: i64) -> Choice {
    Choice {
        key: key.into(),
        names: names(key),
        price_cents,
        available: true,
    }
}

/// Two categories, a maki with a sauce group, a drink (A/A) and a
/// delivery charge in a hidden category.
pub fn catalog() -> Catalog {
    let mut maki = item("MAKI-6", "SUSHI", ItemKind::Product, 650);
    maki.groups = vec![group(
        "MAKI-6|SAUCE",
        0,
        Some(1),
        vec![
            choice("MAKI-6|SAUCE|SOJA", 0),
            choice("MAKI-6|SAUCE|SPICY", 50),
        ],
    )];
    let mut coca = item("COCA", "DRINKS", ItemKind::Product, 300);
    coca.vat_eat_in = VatCode::A;
    coca.vat_takeaway = VatCode::A;
    coca.kitchen = false;
    let fee = item("charge:DELIVERY", "charges", ItemKind::Charge, 250);
    let mut charges = category("charges");
    charges.visible = false;
    Catalog {
        categories: vec![category("SUSHI"), category("DRINKS"), charges],
        items: vec![maki, coca, fee],
    }
}
