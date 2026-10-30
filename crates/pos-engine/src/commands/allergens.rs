//! The 14 EU allergens (decisions V12-D17 to D21): the legend, the
//! allergen sheet of a product or of the whole menu (shown and printed)
//! and the local edit of a product's or an option's list.
//!
//! Non-fiscal: allergens are display and print data. They never enter
//! a signed request, a product name or a department.

use pos_core::allergen::{ALL, Allergen, AllergenSet, SHEET_LANGUAGES};
use pos_core::menu::MenuItem;
use pos_db::queries::allergens::{self, Entity};
use pos_fiscal::event::now_in_belgium;
use pos_print::allergen_sheet::{
    AllergenSheet as PrintSheet, SheetOption, SheetProduct, format_allergen_sheet,
};
use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::SlipOutcome;
use crate::commands::errors;
use crate::state::AppState;

/// One of the 14 allergens, with its name in each screen language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AllergenEntry {
    pub code: String,
    /// The Annex II number, 1 to 14.
    pub number: i32,
    pub names: AllergenNames,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AllergenNames {
    pub fr: String,
    pub nl: String,
    pub en: String,
    pub zh: String,
}

/// An allergen on a sheet, named in the sheet's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AllergenRef {
    pub code: String,
    pub number: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SheetOptionView {
    pub group: String,
    pub name: String,
    pub declared: bool,
    pub contains: Vec<AllergenRef>,
    pub traces: Vec<AllergenRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SheetProductView {
    pub id: i64,
    pub name: String,
    pub name_fr: String,
    /// False: no information, the sheet says so instead of "none".
    pub declared: bool,
    pub contains: Vec<AllergenRef>,
    pub traces: Vec<AllergenRef>,
    pub options: Vec<SheetOptionView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AllergenSheetView {
    pub lang: String,
    pub products: Vec<SheetProductView>,
}

/// The edit of one list. `declared = false` puts the entity back to "no
/// information" (the two lists are then ignored).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAllergensInput {
    /// `menu_item` or `modifier`.
    pub entity_type: String,
    pub entity_id: i64,
    pub declared: bool,
    #[serde(default)]
    pub contains: Vec<String>,
    #[serde(default)]
    pub may_contain: Vec<String>,
}

fn entry(a: Allergen) -> AllergenEntry {
    AllergenEntry {
        code: a.code().to_owned(),
        number: i32::from(a.number()),
        names: AllergenNames {
            fr: a.name("fr").to_owned(),
            nl: a.name("nl").to_owned(),
            en: a.name("en").to_owned(),
            zh: a.name("zh").to_owned(),
        },
    }
}

/// The 14 allergens in Annex order, for the legend. Any logged-in
/// operator.
pub fn do_list_allergens(state: &AppState) -> Result<Vec<AllergenEntry>, String> {
    access::operator(state, Access::Staff)?;
    Ok(ALL.iter().copied().map(entry).collect())
}

fn refs(codes: &[String], lang: &str) -> Vec<AllergenRef> {
    codes
        .iter()
        .filter_map(|c| Allergen::from_code(c))
        .map(|a| AllergenRef {
            code: a.code().to_owned(),
            number: i32::from(a.number()),
            name: a.name(lang).to_owned(),
        })
        .collect()
}

/// The language of a sheet: the one asked for, else the establishment's.
/// A sheet can be shown in fr, nl, en, zh and de; printing leaves out
/// zh (the printer's code page has no Chinese).
fn sheet_lang(state: &AppState, requested: Option<&str>, printing: bool) -> Result<String, String> {
    let allowed = |l: &str| {
        if printing {
            SHEET_LANGUAGES.contains(&l)
        } else {
            SHEET_LANGUAGES.contains(&l) || l == "zh"
        }
    };
    if let Some(lang) = requested.filter(|l| !l.is_empty()) {
        if allowed(lang) {
            return Ok(lang.to_owned());
        }
        return Err(errors::coded(errors::ALLERGEN_UNKNOWN, &[("code", &lang)]));
    }
    let language = state
        .config
        .lock()
        .map_err(errors::internal)?
        .language
        .clone();
    Ok(if allowed(&language) {
        language
    } else {
        "fr".to_owned()
    })
}

fn product_name(item: &MenuItem) -> String {
    if item.name == "unnamed" {
        item.name_fr.clone().unwrap_or_else(|| item.name.clone())
    } else {
        item.name.clone()
    }
}

/// The sheet of one product, or of the whole menu on the grid
/// (hidden and system items left out).
fn build_sheet(
    db: &rusqlite::Connection,
    menu_item_id: Option<i64>,
    lang: &str,
) -> Result<AllergenSheetView, String> {
    let mut items: Vec<MenuItem> = Vec::new();
    for category in pos_db::queries::menu::get_categories(db, lang).map_err(errors::internal)? {
        items.extend(
            pos_db::queries::menu::get_menu_items(db, Some(category.id), lang)
                .map_err(errors::internal)?,
        );
    }
    if let Some(id) = menu_item_id {
        items.retain(|i| i.id == id);
        if items.is_empty() {
            return Err(errors::coded(
                errors::ALLERGEN_TARGET_NOT_FOUND,
                &[("id", &id)],
            ));
        }
    }
    let mut products = Vec::with_capacity(items.len());
    for item in &items {
        products.push(SheetProductView {
            id: item.id,
            name: product_name(item),
            name_fr: item.name_fr.clone().unwrap_or_else(|| product_name(item)),
            declared: item.allergens_declared,
            contains: refs(&item.allergens, lang),
            traces: refs(&item.traces, lang),
            options: options_of(db, item.id, lang)?,
        });
    }
    Ok(AllergenSheetView {
        lang: lang.to_owned(),
        products,
    })
}

#[cfg(feature = "hubrise")]
fn options_of(
    db: &rusqlite::Connection,
    menu_item_id: i64,
    lang: &str,
) -> Result<Vec<SheetOptionView>, String> {
    let groups = pos_db::queries::menu::get_modifier_groups(db, menu_item_id, lang)
        .map_err(errors::internal)?;
    let mut out = Vec::new();
    for group in groups {
        for option in group.options {
            out.push(SheetOptionView {
                group: group.name.clone(),
                name: option.name,
                declared: option.allergens_declared,
                contains: refs(&option.allergens, lang),
                traces: refs(&option.traces, lang),
            });
        }
    }
    Ok(out)
}

#[cfg(not(feature = "hubrise"))]
fn options_of(
    _db: &rusqlite::Connection,
    _menu_item_id: i64,
    _lang: &str,
) -> Result<Vec<SheetOptionView>, String> {
    Ok(Vec::new())
}

/// The allergen sheet of `menu_item_id`, or of the whole active menu
/// (`None`), in `lang` (default: the establishment's). Any logged-in
/// operator.
pub fn do_get_allergen_sheet(
    state: &AppState,
    menu_item_id: Option<i64>,
    lang: Option<String>,
) -> Result<AllergenSheetView, String> {
    access::operator(state, Access::Staff)?;
    let lang = sheet_lang(state, lang.as_deref(), false)?;
    let db = state.db.lock().map_err(errors::internal)?;
    build_sheet(&db, menu_item_id, &lang)
}

fn names(refs: &[AllergenRef]) -> Vec<String> {
    refs.iter().map(|r| r.name.clone()).collect()
}

fn print_sheet_of(view: AllergenSheetView, establishment: String) -> PrintSheet {
    PrintSheet {
        establishment,
        date: now_in_belgium().format("%d/%m/%Y").to_string(),
        lang: view.lang,
        products: view
            .products
            .into_iter()
            .map(|p| SheetProduct {
                name: p.name,
                declared: p.declared,
                contains: names(&p.contains),
                traces: names(&p.traces),
                options: p
                    .options
                    .into_iter()
                    .map(|o| SheetOption {
                        name: o.name,
                        declared: o.declared,
                        contains: names(&o.contains),
                        traces: names(&o.traces),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Print the allergen sheet on the ticket printer (the kitchen printer
/// when the terminal has no other). A printer failure is reported as
/// the outcome, not as an error. Any logged-in operator.
pub async fn do_print_allergen_sheet(
    state: &AppState,
    menu_item_id: Option<i64>,
    lang: Option<String>,
) -> Result<SlipOutcome, String> {
    access::operator(state, Access::Staff)?;
    let lang = sheet_lang(state, lang.as_deref(), true)?;
    let view = {
        let db = state.db.lock().map_err(errors::internal)?;
        build_sheet(&db, menu_item_id, &lang)?
    };
    if view.products.is_empty() {
        return Ok(SlipOutcome::NothingToPrint);
    }
    let (address, establishment) = {
        let config = state.config.lock().map_err(errors::internal)?;
        let address = config
            .printer_address
            .clone()
            .or_else(|| config.kitchen_printer_address.clone())
            .filter(|a| !a.is_empty())
            .map(|a| {
                let paper = config.paper_for(&a);
                (a, paper)
            });
        (address, config.restaurant_name.clone())
    };
    let Some((address, paper)) = address else {
        return Ok(SlipOutcome::NoPrinter);
    };
    let bytes = format_allergen_sheet(&print_sheet_of(view, establishment), paper);
    match pos_print::transport::send_to_printer_with(
        &address,
        &bytes,
        state.timers.printer_status_read,
    )
    .await
    {
        Ok(status) if status.ready() => Ok(SlipOutcome::Printed),
        Ok(status) => {
            tracing::warn!(?status.warnings, address, "allergen sheet printer not ready");
            Ok(SlipOutcome::Failed)
        }
        Err(err) => {
            tracing::warn!(%err, address, "allergen sheet not printed");
            Ok(SlipOutcome::Failed)
        }
    }
}

/// While a catalog source manages the menu, allergens are edited there.
#[cfg(feature = "hubrise")]
fn refuse_when_managed(db: &rusqlite::Connection) -> Result<(), String> {
    match pos_catalog::managed_by(db) {
        Ok(Some(_)) => Err(errors::hubrise::MENU_MANAGED.to_owned()),
        Ok(None) => Ok(()),
        Err(e) => Err(errors::internal(e)),
    }
}

#[cfg(not(feature = "hubrise"))]
fn refuse_when_managed(_db: &rusqlite::Connection) -> Result<(), String> {
    Ok(())
}

/// Set the allergens of a product or an option by hand. Managers only;
/// refused (`hubrise_menu_managed`) while a catalog source manages the
/// menu. Every real change is logged with the manager's INSZ in
/// `allergen_change_log`. Returns whether anything changed.
pub fn do_set_allergens(state: &AppState, input: SetAllergensInput) -> Result<bool, String> {
    let admin = access::operator(state, Access::Admin)?;
    let entity = match input.entity_type.as_str() {
        "menu_item" => Entity::MenuItem,
        "modifier" => Entity::Modifier,
        _ => {
            return Err(errors::coded(
                errors::ALLERGEN_TARGET_NOT_FOUND,
                &[("id", &input.entity_id)],
            ));
        }
    };
    let set = AllergenSet {
        contains: input.contains.clone(),
        may_contain: input.may_contain.clone(),
    };
    if input.declared
        && let Some(code) = set.first_unknown()
    {
        return Err(errors::coded(errors::ALLERGEN_UNKNOWN, &[("code", &code)]));
    }
    let db = state.db.lock().map_err(errors::internal)?;
    refuse_when_managed(&db)?;
    // The POS's own rows (`pili:%`, the gift voucher product) are no food: they never get an
    // allergen list and answer as not found, like in the menu editor.
    let exists: bool = db
        .query_row(
            match entity {
                Entity::MenuItem => {
                    "SELECT EXISTS(SELECT 1 FROM menu_items WHERE id = ?1
                     AND (external_id IS NULL OR external_id NOT LIKE 'pili:%'))"
                }
                Entity::Modifier => "SELECT EXISTS(SELECT 1 FROM modifiers WHERE id = ?1)",
            },
            [input.entity_id],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if !exists {
        return Err(errors::coded(
            errors::ALLERGEN_TARGET_NOT_FOUND,
            &[("id", &input.entity_id)],
        ));
    }
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let changed = allergens::write(
        &tx,
        entity,
        input.entity_id,
        input.declared.then_some(&set),
        "local",
        Some(&admin.insz),
    )
    .map_err(errors::internal)?;
    tx.commit().map_err(errors::internal)?;
    Ok(changed)
}
