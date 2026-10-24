//! A HubRise catalog as the neutral catalog of `pos-catalog`.
//!
//! - Each SKU is an item, keyed by its `ref` (the POS product id stays
//!   tied to it). A product with several SKUs names each one
//!   "product sku".
//! - VAT comes from the product's `tax_rate`: `eat_in` for dine-in,
//!   `collection` for takeaway, and `delivery` must equal `collection`.
//!   21, 12, 6 and 0 % are A, B, C and D. Without a rate, the SKU's
//!   `custom_fields.vat_category` decides (food B/C, beverage A/A,
//!   zero_rated D/D, out_of_scope X/X).
//! - An option list attached to a SKU is one group of that item
//!   (`{sku}|{list}`), its options `{sku}|{list}|{option}`.
//! - Catalog charges are items of a hidden category (`charge:{ref}`),
//!   never on the POS grid.
//! - The SKU's `custom_fields.code` is the product's menu code ("A2"),
//!   shown on the till in place of the name when it is set to.
//! - Tags on the product or the SKU: `pos:open_price` (price entered on
//!   the POS), `pos:no_kitchen` (no bar/kitchen slip), `pos:hidden`
//!   (sold online only).
//! - Allergen tags (V12-D18) on the product, the SKU or an option:
//!   `allergen:<code>` (contains), `traces:<code>` (may contain),
//!   `allergen:none` (declared, no allergen); the 14 codes are those of
//!   `pos_core::allergen`. Any allergen tag declares the list; without
//!   one there is no information. An unknown code refuses the catalog.
//! - A disabled restriction (`restrictions.enabled = false`) takes an
//!   element off sale. Deals and discounts are not mapped.

use std::collections::HashMap;

use pos_catalog::{Catalog, Category, Choice, Group, Issue, IssueCode, Item, ItemKind, Names};
use pos_core::allergen::AllergenSet;
use pos_core::vat::VatCode;
use pos_hubrise_types::money::{parse_cents, parse_rate_hundredths};
use pos_hubrise_types::{CatalogData, OptionList, Product, Restrictions, Sku, TaxRate};

use crate::catalog_mapper::{decode_category_translations, decode_sku_translations};

/// The catalog source name in `catalog_sync_state`.
pub const SOURCE: &str = "hubrise";

/// Key of the hidden category holding the charges.
pub const CHARGES_CATEGORY: &str = "pos:charges";
pub const TAG_OPEN_PRICE: &str = "pos:open_price";
pub const TAG_NO_KITCHEN: &str = "pos:no_kitchen";
pub const TAG_HIDDEN: &str = "pos:hidden";
/// `allergen:<code>`: the item contains the allergen.
pub const TAG_ALLERGEN_PREFIX: &str = "allergen:";
/// `traces:<code>`: the item may contain traces of the allergen.
pub const TAG_TRACES_PREFIX: &str = "traces:";
/// `allergen:none`: the list is declared and empty.
pub const TAG_ALLERGEN_NONE: &str = "allergen:none";

/// The allergen list the tags give: `None` without any allergen tag
/// (no information), otherwise a declared list. Codes are matched in
/// lower case; unknown ones are kept for the catalog validation.
pub(crate) fn allergens_of<'a>(tags: impl Iterator<Item = &'a String>) -> Option<AllergenSet> {
    let mut set = AllergenSet::default();
    let mut declared = false;
    for tag in tags {
        let tag = tag.trim().to_lowercase();
        if let Some(code) = tag.strip_prefix(TAG_ALLERGEN_PREFIX) {
            declared = true;
            if code != "none" {
                set.contains.push(code.trim().to_owned());
            }
        } else if let Some(code) = tag.strip_prefix(TAG_TRACES_PREFIX) {
            declared = true;
            if code != "none" {
                set.may_contain.push(code.trim().to_owned());
            }
        }
    }
    declared.then(|| set.normalized())
}

/// The neutral catalog, and the issues found while mapping (they
/// refuse the catalog like validation issues do).
pub fn to_neutral(data: &CatalogData) -> (Catalog, Vec<Issue>) {
    let mut issues = Vec::new();
    let mut out = Catalog::default();

    // Retrieved catalogs name categories and option lists by HubRise
    // id, uploaded ones by ref.
    let category_by_id: HashMap<&str, &str> = data
        .categories
        .iter()
        .filter_map(|c| Some((c.id.as_deref()?, c.ref_field.as_str())))
        .collect();
    let lists_by_ref: HashMap<&str, &OptionList> = data
        .option_lists
        .iter()
        .map(|l| (l.ref_field.as_str(), l))
        .collect();
    let list_ref_by_id: HashMap<&str, &str> = data
        .option_lists
        .iter()
        .filter_map(|l| Some((l.id.as_deref()?, l.ref_field.as_str())))
        .collect();

    for cat in &data.categories {
        let decoded = decode_category_translations(&cat.description);
        let mut names = Names::new();
        for (lang, value) in [
            ("fr", decoded.fr),
            ("en", decoded.en),
            ("nl", decoded.nl),
            ("zh", decoded.zh),
        ] {
            if !value.trim().is_empty() {
                names.insert(lang.into(), value);
            }
        }
        names.entry("fr".into()).or_insert_with(|| cat.name.clone());
        out.categories.push(Category {
            key: cat.ref_field.clone(),
            names,
            visible: true,
        });
    }

    for product in &data.products {
        let category_key = if product.category_ref.is_empty() {
            product
                .category_id
                .as_deref()
                .map(|id| category_by_id.get(id).copied().unwrap_or(id))
                .unwrap_or_default()
                .to_owned()
        } else {
            product.category_ref.clone()
        };
        for sku in &product.skus {
            let item = map_sku(
                product,
                sku,
                &category_key,
                &lists_by_ref,
                &list_ref_by_id,
                &mut issues,
            );
            out.items.push(item);
        }
    }

    if !data.charges.is_empty() {
        let mut names = Names::new();
        names.insert("fr".into(), "Frais".into());
        out.categories.push(Category {
            key: CHARGES_CATEGORY.into(),
            names,
            visible: false,
        });
    }
    for charge in &data.charges {
        let key = format!("charge:{}", charge.ref_field.as_deref().unwrap_or_default());
        if charge
            .ref_field
            .as_deref()
            .is_none_or(|r| r.trim().is_empty())
        {
            issues.push(Issue::new(IssueCode::EmptyKey, &charge.name));
        }
        let (price_cents, open_price) = match charge.price.as_deref() {
            Some(p) => (price(p, &key, &mut issues), false),
            // A charge without a catalog price is priced per order.
            None => (0, true),
        };
        out.items.push(Item {
            key,
            category_key: CHARGES_CATEGORY.into(),
            kind: ItemKind::Charge,
            names: fr_name(&charge.name),
            code: None,
            price_cents,
            // Only the label of a charge with no product to follow:
            // on an order, a charge is split over its products' labels
            // (`webshop::resolve`), and a platform's are excluded per
            // channel.
            vat_eat_in: VatCode::C,
            vat_takeaway: VatCode::C,
            open_price,
            kitchen: false,
            on_pos: false,
            available: Restrictions::is_enabled(&charge.restrictions),
            groups: Vec::new(),
            allergens: None,
        });
    }

    (out, issues)
}

/// Map and apply a HubRise catalog. A catalog without data is refused
/// like an empty one. `last_catalog_sync_at` is bumped when the menu
/// mirrors the catalog.
pub fn apply(
    conn: &mut rusqlite::Connection,
    catalog: &pos_hubrise_types::Catalog,
) -> crate::errors::Result<pos_catalog::ApplyOutcome> {
    let empty = CatalogData::default();
    let (neutral, issues) = to_neutral(catalog.data.as_ref().unwrap_or(&empty));
    let outcome = pos_catalog::apply_checked(conn, SOURCE, &neutral, issues)
        .map_err(|e| crate::errors::HubriseError::Other(e.to_string()))?;
    if !matches!(outcome, pos_catalog::ApplyOutcome::Rejected { .. }) {
        conn.execute(
            "UPDATE hubrise_connection SET last_catalog_sync_at = datetime('now') WHERE id = 1",
            [],
        )?;
    }
    Ok(outcome)
}

fn map_sku(
    product: &Product,
    sku: &Sku,
    category_key: &str,
    lists_by_ref: &HashMap<&str, &OptionList>,
    list_ref_by_id: &HashMap<&str, &str>,
    issues: &mut Vec<Issue>,
) -> Item {
    let key = sku.ref_field.clone().unwrap_or_default();
    if key.trim().is_empty() {
        let at = sku.id.as_deref().unwrap_or(product.name.as_str());
        issues.push(Issue::new(IssueCode::EmptyKey, at));
    }

    let decoded = decode_sku_translations(&sku.custom_fields);
    let base_name = match sku.name.as_deref().map(str::trim) {
        Some(sku_name) if product.skus.len() > 1 && !sku_name.is_empty() => {
            format!("{} {}", product.name, sku_name)
        }
        _ => product.name.clone(),
    };
    let t = &decoded.translations;
    let mut names = Names::new();
    for (lang, value) in [
        ("fr", &t.fr.name),
        ("en", &t.en.name),
        ("nl", &t.nl.name),
        ("zh", &t.zh.name),
    ] {
        if !value.trim().is_empty() {
            names.insert(lang.into(), value.clone());
        }
    }
    names.entry("fr".into()).or_insert(base_name);

    let (vat_eat_in, vat_takeaway) = vat(
        product.tax_rate.as_ref(),
        vat_category(&sku.custom_fields),
        &key,
        issues,
    );

    let tagged = |tag: &str| product.tags.iter().chain(&sku.tags).any(|t| t == tag);

    let list_refs = sku.option_list_refs.iter().map(String::as_str).chain(
        sku.option_list_ids
            .iter()
            .map(|id| list_ref_by_id.get(id.as_str()).copied().unwrap_or(id)),
    );
    let mut groups = Vec::new();
    for list_ref in list_refs {
        match lists_by_ref.get(list_ref) {
            Some(list) => groups.push(map_list(&key, list, issues)),
            None => issues.push(Issue::new(IssueCode::UnknownOptionList, &key)),
        }
    }

    Item {
        price_cents: price(&sku.price, &key, issues),
        key,
        category_key: category_key.to_owned(),
        kind: ItemKind::Product,
        names,
        code: menu_code(&sku.custom_fields),
        vat_eat_in,
        vat_takeaway,
        open_price: tagged(TAG_OPEN_PRICE),
        kitchen: !tagged(TAG_NO_KITCHEN),
        on_pos: !tagged(TAG_HIDDEN),
        available: Restrictions::is_enabled(&sku.restrictions),
        groups,
        allergens: allergens_of(product.tags.iter().chain(&sku.tags)),
    }
}

fn map_list(sku_key: &str, list: &OptionList, issues: &mut Vec<Issue>) -> Group {
    let key = format!("{sku_key}|{}", list.ref_field);
    let options = list
        .options
        .iter()
        .map(|opt| {
            let opt_ref = opt.ref_field.as_deref().unwrap_or_default();
            let opt_key = format!("{key}|{opt_ref}");
            if opt_ref.trim().is_empty() {
                issues.push(Issue::new(IssueCode::EmptyKey, &opt_key));
            }
            Choice {
                price_cents: opt
                    .price
                    .as_deref()
                    .map_or(0, |p| price(p, &opt_key, issues)),
                names: fr_name(&opt.name),
                available: Restrictions::is_enabled(&opt.restrictions),
                allergens: allergens_of(opt.tags.iter()),
                key: opt_key,
            }
        })
        .collect();
    Group {
        names: fr_name(&list.name),
        min: list.min_selections.unwrap_or(0),
        max: list.max_selections,
        options,
        key,
    }
}

fn fr_name(name: &str) -> Names {
    let mut names = Names::new();
    names.insert("fr".into(), name.to_owned());
    names
}

fn price(money: &str, key: &str, issues: &mut Vec<Issue>) -> i64 {
    parse_cents(money).unwrap_or_else(|_| {
        issues.push(Issue::new(IssueCode::BadPrice, key));
        0
    })
}

/// `custom_fields.code`, trimmed, when the SKU sets one.
fn menu_code(custom_fields: &Option<serde_json::Value>) -> Option<String> {
    let code = custom_fields.as_ref()?.get("code")?.as_str()?.trim();
    (!code.is_empty()).then(|| code.to_owned())
}

/// `custom_fields.vat_category` when the SKU sets it.
fn vat_category(custom_fields: &Option<serde_json::Value>) -> Option<&str> {
    custom_fields.as_ref()?.get("vat_category")?.as_str()
}

pub(crate) fn code_of_rate(rate: &str) -> Option<VatCode> {
    match parse_rate_hundredths(rate)? {
        2100 => Some(VatCode::A),
        1200 => Some(VatCode::B),
        600 => Some(VatCode::C),
        0 => Some(VatCode::D),
        _ => None,
    }
}

fn codes_of_category(category: &str) -> Option<(VatCode, VatCode)> {
    match category {
        "food" => Some((VatCode::B, VatCode::C)),
        "beverage" => Some((VatCode::A, VatCode::A)),
        "zero_rated" => Some((VatCode::D, VatCode::D)),
        "out_of_scope" => Some((VatCode::X, VatCode::X)),
        _ => None,
    }
}

/// Dine-in and takeaway labels of a SKU. A missing side falls back to
/// the VAT category; with neither, the catalog is refused.
fn vat(
    tax: Option<&TaxRate>,
    category: Option<&str>,
    key: &str,
    issues: &mut Vec<Issue>,
) -> (VatCode, VatCode) {
    let from_rate = |rate: Option<&String>, issues: &mut Vec<Issue>| -> Option<VatCode> {
        let rate = rate?;
        let code = code_of_rate(rate);
        if code.is_none() {
            issues.push(Issue::new(IssueCode::UnknownVatRate, key));
        }
        code
    };
    let eat_in = from_rate(tax.and_then(|t| t.eat_in.as_ref()), issues);
    let collection = from_rate(tax.and_then(|t| t.collection.as_ref()), issues);
    let delivery = from_rate(tax.and_then(|t| t.delivery.as_ref()), issues);
    if let (Some(c), Some(d)) = (collection, delivery)
        && c != d
    {
        issues.push(Issue::new(IssueCode::VatMismatch, key));
    }
    let takeaway = collection.or(delivery);

    let fallback = category.and_then(codes_of_category);
    match (eat_in, takeaway, fallback) {
        (Some(e), Some(t), _) => (e, t),
        (e, t, Some((fe, ft))) => (e.unwrap_or(fe), t.unwrap_or(ft)),
        (e, t, None) => {
            // A rate already refused needs no second issue.
            let refused = issues
                .iter()
                .any(|i| i.code == IssueCode::UnknownVatRate && i.key == key);
            if !refused {
                issues.push(Issue::new(IssueCode::MissingVat, key));
            }
            (e.unwrap_or(VatCode::B), t.unwrap_or(VatCode::C))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data(v: serde_json::Value) -> CatalogData {
        serde_json::from_value(v).unwrap()
    }

    fn sample() -> serde_json::Value {
        json!({
            "categories": [{"ref": "SUSHI", "name": "Sushi"}, {"ref": "DRINKS", "name": "Boissons"}],
            "option_lists": [{
                "ref": "SAUCE", "name": "Sauce", "min_selections": 0, "max_selections": 1,
                "options": [{"ref": "SOJA", "name": "Soja"}, {"ref": "SPICY", "name": "Spicy", "price": "0.50 EUR"}]
            }],
            "products": [{
                "ref": "MAKI", "category_ref": "SUSHI", "name": "Maki saumon",
                "tax_rate": {"eat_in": "12.0", "collection": "6.0", "delivery": "6.0"},
                "skus": [
                    {"ref": "MAKI-6", "name": "6 pcs", "price": "6.50 EUR", "option_list_refs": ["SAUCE"]},
                    {"ref": "MAKI-12", "name": "12 pcs", "price": "12.00 EUR", "option_list_refs": ["SAUCE"]}
                ]
            }, {
                "ref": "COCA", "category_ref": "DRINKS", "name": "Coca-Cola", "tags": ["pos:no_kitchen"],
                "tax_rate": {"eat_in": "21.0", "collection": "21.0", "delivery": "21.0"},
                "skus": [{"ref": "COCA", "price": "3.00 EUR"}]
            }],
            "charges": [{"ref": "DELIVERY", "name": "Livraison", "type": "delivery", "price": "2.50 EUR"}]
        })
    }

    fn only_issues(v: serde_json::Value) -> Vec<(IssueCode, String)> {
        to_neutral(&data(v))
            .1
            .into_iter()
            .map(|i| (i.code, i.key))
            .collect()
    }

    #[test]
    fn a_catalog_without_allergen_tags_has_no_information() {
        let (c, _) = to_neutral(&data(sample()));
        assert!(c.items.iter().all(|i| i.allergens.is_none()));
        assert!(
            c.items
                .iter()
                .flat_map(|i| &i.groups)
                .flat_map(|g| &g.options)
                .all(|o| o.allergens.is_none())
        );
    }

    #[test]
    fn allergen_tags_on_product_sku_and_option_are_mapped() {
        let mut v = sample();
        v["products"][0]["tags"] = json!(["allergen:fish", "traces:molluscs"]);
        v["products"][0]["skus"][0]["tags"] = json!(["allergen:Sesame", "allergen:fish"]);
        v["option_lists"][0]["options"][0]["tags"] =
            json!(["allergen:soybeans", "allergen:gluten"]);
        v["option_lists"][0]["options"][1]["tags"] = json!(["allergen:none"]);
        let (c, issues) = to_neutral(&data(v));
        assert!(issues.is_empty(), "{issues:?}");
        assert!(pos_catalog::validate(&c).is_empty());
        let a6 = c.items[0].allergens.as_ref().unwrap();
        assert_eq!(a6.contains, vec!["fish", "sesame"]);
        assert_eq!(a6.may_contain, vec!["molluscs"]);
        // The product's tags reach the second SKU too.
        let a12 = c.items[1].allergens.as_ref().unwrap();
        assert_eq!(a12.contains, vec!["fish"]);
        assert!(c.items[2].allergens.is_none());
        let soja = &c.items[0].groups[0].options[0];
        assert_eq!(
            soja.allergens.as_ref().unwrap().contains,
            vec!["gluten", "soybeans"]
        );
        let spicy = &c.items[0].groups[0].options[1];
        assert_eq!(
            spicy.allergens,
            Some(AllergenSet::default()),
            "declared, none"
        );
    }

    #[test]
    fn allergen_none_declares_an_empty_list_on_a_sku() {
        let mut v = sample();
        v["products"][1]["skus"][0]["tags"] = json!(["allergen:none"]);
        let (c, _) = to_neutral(&data(v));
        assert_eq!(c.items[2].allergens, Some(AllergenSet::default()));
    }

    #[test]
    fn a_traces_only_tag_declares_the_list() {
        let mut v = sample();
        v["products"][1]["tags"] = json!(["traces:nuts"]);
        let (c, _) = to_neutral(&data(v));
        let a = c.items[2].allergens.as_ref().unwrap();
        assert!(a.contains.is_empty());
        assert_eq!(a.may_contain, vec!["nuts"]);
    }

    #[test]
    fn an_unknown_allergen_code_refuses_the_catalog() {
        let mut v = sample();
        v["products"][1]["tags"] = json!(["allergen:glutten"]);
        v["option_lists"][0]["options"][0]["tags"] = json!(["traces:peanut"]);
        let (c, _) = to_neutral(&data(v));
        let found: Vec<(IssueCode, String)> = pos_catalog::validate(&c)
            .into_iter()
            .map(|i| (i.code, i.key))
            .collect();
        assert!(found.contains(&(IssueCode::UnknownAllergen, "COCA".into())));
        assert!(found.contains(&(IssueCode::UnknownAllergen, "MAKI-6|SAUCE|SOJA".into())));
    }

    #[test]
    fn a_sample_catalog_maps_without_issues() {
        let (c, issues) = to_neutral(&data(sample()));
        assert!(issues.is_empty(), "{issues:?}");
        assert!(pos_catalog::validate(&c).is_empty());
        let keys: Vec<&str> = c.items.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(keys, vec!["MAKI-6", "MAKI-12", "COCA", "charge:DELIVERY"]);
    }

    #[test]
    fn each_sku_is_an_item_named_after_product_and_sku() {
        let (c, _) = to_neutral(&data(sample()));
        assert_eq!(c.items[0].names["fr"], "Maki saumon 6 pcs");
        assert_eq!(c.items[0].price_cents, 650);
        assert_eq!(
            c.items[2].names["fr"], "Coca-Cola",
            "single SKU: product name"
        );
    }

    #[test]
    fn tax_rates_give_the_dine_in_and_takeaway_labels() {
        let (c, _) = to_neutral(&data(sample()));
        assert_eq!(
            (c.items[0].vat_eat_in, c.items[0].vat_takeaway),
            (VatCode::B, VatCode::C)
        );
        assert_eq!(
            (c.items[2].vat_eat_in, c.items[2].vat_takeaway),
            (VatCode::A, VatCode::A)
        );
    }

    #[test]
    fn every_belgian_rate_maps_and_others_are_refused() {
        for (rate, code) in [
            ("21.0", VatCode::A),
            ("12", VatCode::B),
            ("6.0", VatCode::C),
            ("0.0", VatCode::D),
        ] {
            assert_eq!(code_of_rate(rate), Some(code), "{rate}");
        }
        let mut v = sample();
        v["products"][1]["tax_rate"]["eat_in"] = json!("20.0");
        assert_eq!(
            only_issues(v),
            vec![(IssueCode::UnknownVatRate, "COCA".into())]
        );
    }

    #[test]
    fn delivery_and_collection_must_agree() {
        let mut v = sample();
        v["products"][0]["tax_rate"]["delivery"] = json!("12.0");
        let found = only_issues(v);
        assert_eq!(found.len(), 2, "one per SKU: {found:?}");
        assert!(found.iter().all(|(c, _)| *c == IssueCode::VatMismatch));
    }

    #[test]
    fn delivery_alone_sets_the_takeaway_label() {
        let mut v = sample();
        v["products"][0]["tax_rate"] = json!({"eat_in": "12.0", "delivery": "6.0"});
        let (c, issues) = to_neutral(&data(v));
        assert!(issues.is_empty());
        assert_eq!(c.items[0].vat_takeaway, VatCode::C);
    }

    #[test]
    fn without_a_rate_the_vat_category_decides() {
        let mut v = sample();
        v["products"][0].as_object_mut().unwrap().remove("tax_rate");
        v["products"][0]["skus"][0]["custom_fields"] = json!({"vat_category": "food"});
        v["products"][0]["skus"][1]["custom_fields"] = json!({"vat_category": "zero_rated"});
        let (c, issues) = to_neutral(&data(v));
        assert!(issues.is_empty());
        assert_eq!(
            (c.items[0].vat_eat_in, c.items[0].vat_takeaway),
            (VatCode::B, VatCode::C)
        );
        assert_eq!(
            (c.items[1].vat_eat_in, c.items[1].vat_takeaway),
            (VatCode::D, VatCode::D)
        );
    }

    #[test]
    fn a_beverage_and_an_out_of_scope_category_map_too() {
        assert_eq!(
            codes_of_category("beverage"),
            Some((VatCode::A, VatCode::A))
        );
        assert_eq!(
            codes_of_category("out_of_scope"),
            Some((VatCode::X, VatCode::X))
        );
        assert_eq!(codes_of_category("sushi"), None);
    }

    #[test]
    fn without_rate_or_category_the_catalog_is_refused() {
        let mut v = sample();
        v["products"][1].as_object_mut().unwrap().remove("tax_rate");
        assert_eq!(only_issues(v), vec![(IssueCode::MissingVat, "COCA".into())]);
    }

    #[test]
    fn a_shared_option_list_is_one_group_per_item() {
        let (c, _) = to_neutral(&data(sample()));
        let g6 = &c.items[0].groups[0];
        let g12 = &c.items[1].groups[0];
        assert_eq!(
            (g6.key.as_str(), g12.key.as_str()),
            ("MAKI-6|SAUCE", "MAKI-12|SAUCE")
        );
        assert_eq!((g6.min, g6.max), (0, Some(1)));
        assert_eq!(g6.options[1].key, "MAKI-6|SAUCE|SPICY");
        assert_eq!(g6.options[1].price_cents, 50);
        assert_eq!(g6.options[0].price_cents, 0);
    }

    #[test]
    fn option_lists_named_by_id_resolve() {
        let mut v = sample();
        v["option_lists"][0]["id"] = json!("ol1");
        v["products"][0]["skus"][0]["option_list_refs"] = json!([]);
        v["products"][0]["skus"][0]["option_list_ids"] = json!(["ol1"]);
        let (c, issues) = to_neutral(&data(v));
        assert!(issues.is_empty());
        assert_eq!(c.items[0].groups[0].key, "MAKI-6|SAUCE");
    }

    #[test]
    fn an_unknown_option_list_is_refused() {
        let mut v = sample();
        v["products"][1]["skus"][0]["option_list_refs"] = json!(["NOPE"]);
        assert_eq!(
            only_issues(v),
            vec![(IssueCode::UnknownOptionList, "COCA".into())]
        );
    }

    #[test]
    fn categories_named_by_id_resolve() {
        let mut v = sample();
        v["categories"][1]["id"] = json!("c2");
        v["products"][1]["category_ref"] = json!(null);
        v["products"][1]["category_id"] = json!("c2");
        let (c, _) = to_neutral(&data(v));
        assert_eq!(c.items[2].category_key, "DRINKS");
    }

    #[test]
    fn a_sku_without_ref_or_with_a_bad_price_is_refused() {
        let mut v = sample();
        v["products"][1]["skus"][0]["ref"] = json!(null);
        v["products"][0]["skus"][0]["price"] = json!("6.505 EUR");
        let found = only_issues(v);
        assert!(found.contains(&(IssueCode::EmptyKey, "Coca-Cola".into())));
        assert!(found.contains(&(IssueCode::BadPrice, "MAKI-6".into())));
    }

    #[test]
    fn charges_are_hidden_items() {
        let (c, _) = to_neutral(&data(sample()));
        let fee = c.items.last().unwrap();
        assert_eq!(fee.kind, ItemKind::Charge);
        assert_eq!(
            (fee.price_cents, fee.on_pos, fee.kitchen),
            (250, false, false)
        );
        let cat = c
            .categories
            .iter()
            .find(|c| c.key == CHARGES_CATEGORY)
            .unwrap();
        assert!(!cat.visible);
    }

    #[test]
    fn a_charge_without_price_is_priced_per_order() {
        let mut v = sample();
        v["charges"][0].as_object_mut().unwrap().remove("price");
        let (c, _) = to_neutral(&data(v));
        assert!(c.items.last().unwrap().open_price);
    }

    #[test]
    fn pos_tags_set_the_flags() {
        let mut v = sample();
        v["products"][0]["skus"][0]["tags"] = json!(["pos:open_price", "pos:hidden"]);
        let (c, _) = to_neutral(&data(v));
        assert!(c.items[0].open_price && !c.items[0].on_pos && c.items[0].kitchen);
        assert!(!c.items[1].open_price && c.items[1].on_pos);
        assert!(!c.items[2].kitchen, "pos:no_kitchen on the product");
    }

    #[test]
    fn a_disabled_restriction_takes_the_sku_off_sale() {
        let mut v = sample();
        v["products"][0]["skus"][1]["restrictions"] = json!({"enabled": false});
        v["option_lists"][0]["options"][1]["restrictions"] = json!({"enabled": false});
        let (c, _) = to_neutral(&data(v));
        assert!(c.items[0].available && !c.items[1].available);
        assert!(!c.items[0].groups[0].options[1].available);
    }

    #[test]
    fn translations_come_from_custom_fields() {
        let mut v = sample();
        v["products"][1]["skus"][0]["custom_fields"] = json!({
            "translations": {"fr": {"name": "Coca 33cl"}, "zh": {"name": "可口可乐"}}
        });
        v["categories"][0]["description"] =
            json!(r#"{"translations":{"fr":"Sushis","en":"Sushi","nl":"","zh":"寿司"}}"#);
        let (c, _) = to_neutral(&data(v));
        assert_eq!(c.items[2].names["fr"], "Coca 33cl");
        assert_eq!(c.items[2].names["zh"], "可口可乐");
        assert_eq!(c.categories[0].names["fr"], "Sushis");
        assert!(
            !c.categories[0].names.contains_key("nl"),
            "empty names are left out"
        );
    }

    #[test]
    fn the_menu_code_comes_from_custom_fields() {
        let mut v = sample();
        v["products"][0]["skus"][0]["custom_fields"] =
            json!({"code": " A2 ", "vat_category": "food"});
        v["products"][0]["skus"][1]["custom_fields"] = json!({"code": ""});
        let (c, issues) = to_neutral(&data(v));
        assert!(issues.is_empty());
        assert_eq!(c.items[0].code.as_deref(), Some("A2"));
        assert_eq!(c.items[1].code, None, "an empty code is no code");
        assert_eq!(c.items[2].code, None);
    }
}
