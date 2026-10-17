//! The balanced journal: one entry per booking period.
//!
//! Debit: what was collected, per payment method (payments, cash rounding, tips), and
//! the sales not collected. Credit: sales per VAT label, the VAT collected per rate,
//! gift vouchers sold (a liability, not a sale), cash rounding and tips to return.
//! Redeemed gift vouchers are a debit on the gift voucher account. A refund is the same
//! lines with the opposite sign, so a day is a net entry.
//!
//! Account numbers come from a mapping with Belgian chart of accounts (PCMN) defaults;
//! an accounts file (JSON) overrides any of them. Everything is computed from the signed
//! tickets: base from the FDM's VAT calculation, VAT as total minus base, so an entry
//! always balances when the day reconciles with its Z report.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::aggregate::{Day, Kind, Method};
use crate::csv::{Decimal, Table, amount};
use crate::error::{AccountingError, Result};
use crate::parse::LABELS;

/// The layout of the journal file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Plain columns, `YYYY-MM-DD`, decimal point: the reference layout.
    Generic,
    /// Winbooks miscellaneous operations (best effort, see the README).
    Winbooks,
    /// Yuki general journal (best effort, see the README).
    Yuki,
}

impl Profile {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "generic" => Some(Self::Generic),
            "winbooks" => Some(Self::Winbooks),
            "yuki" => Some(Self::Yuki),
            _ => None,
        }
    }
}

/// Method codes an accounts file may map.
const METHOD_CODES: [&str; 6] = [
    "especes",
    "carte",
    "titres_repas",
    "application",
    "en_ligne",
    "autre",
];

/// Account numbers of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accounts {
    pub journal: String,
    /// Sales account per VAT label.
    pub sales: BTreeMap<String, String>,
    /// VAT collected account per label (labels without VAT need none).
    pub vat: BTreeMap<String, String>,
    /// Collection account per method code (`especes`, `carte`...).
    pub payments: BTreeMap<String, String>,
    /// Gift vouchers outstanding (credit at sale, debit at redemption).
    pub gift_vouchers: String,
    pub rounding_cost: String,
    pub rounding_income: String,
    /// Tips to return to the staff.
    pub tips: String,
    /// Sales not collected (receivable).
    pub unpaid: String,
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

impl Default for Accounts {
    fn default() -> Self {
        Self {
            journal: "VTE".to_string(),
            sales: map(&[
                ("A", "700000"),
                ("B", "700100"),
                ("C", "700200"),
                ("D", "700300"),
                ("X", "700400"),
            ]),
            vat: map(&[("A", "451021"), ("B", "451012"), ("C", "451006")]),
            payments: map(&[
                ("especes", "570000"),
                ("carte", "580000"),
                ("titres_repas", "580100"),
                ("application", "580200"),
                ("en_ligne", "580300"),
                ("autre", "580900"),
            ]),
            gift_vouchers: "489000".to_string(),
            rounding_cost: "657000".to_string(),
            rounding_income: "757000".to_string(),
            tips: "489100".to_string(),
            unpaid: "400000".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountsFile {
    journal: Option<String>,
    sales: Option<BTreeMap<String, String>>,
    vat: Option<BTreeMap<String, String>>,
    payments: Option<BTreeMap<String, String>>,
    gift_vouchers: Option<String>,
    rounding_cost: Option<String>,
    rounding_income: Option<String>,
    tips: Option<String>,
    unpaid: Option<String>,
}

fn valid_account(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 16
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn check(what: &str, value: &str) -> Result<()> {
    if valid_account(value) {
        Ok(())
    } else {
        Err(AccountingError::Accounts(format!(
            "{what}: {value:?} is not an account number (1 to 16 letters, digits, '.' or '-')"
        )))
    }
}

fn overlay(
    target: &mut BTreeMap<String, String>,
    given: Option<BTreeMap<String, String>>,
    allowed: &[&str],
    what: &str,
) -> Result<()> {
    for (key, value) in given.into_iter().flatten() {
        if !allowed.contains(&key.as_str()) {
            return Err(AccountingError::Accounts(format!(
                "{what}: unknown key {key:?} (expected one of {})",
                allowed.join(", ")
            )));
        }
        check(&format!("{what}.{key}"), &value)?;
        target.insert(key, value);
    }
    Ok(())
}

impl Accounts {
    /// The defaults with the entries of a JSON file applied on top.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| AccountingError::Accounts(format!("{}: {e}", path.display())))?;
        Self::from_json(&text)
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let file: AccountsFile = serde_json::from_str(text)
            .map_err(|e| AccountingError::Accounts(format!("invalid JSON: {e}")))?;
        let mut accounts = Self::default();
        overlay(&mut accounts.sales, file.sales, &LABELS, "sales")?;
        overlay(&mut accounts.vat, file.vat, &LABELS, "vat")?;
        overlay(
            &mut accounts.payments,
            file.payments,
            &METHOD_CODES,
            "payments",
        )?;
        for (what, value, target) in [
            ("journal", file.journal, &mut accounts.journal),
            (
                "gift_vouchers",
                file.gift_vouchers,
                &mut accounts.gift_vouchers,
            ),
            (
                "rounding_cost",
                file.rounding_cost,
                &mut accounts.rounding_cost,
            ),
            (
                "rounding_income",
                file.rounding_income,
                &mut accounts.rounding_income,
            ),
            ("tips", file.tips, &mut accounts.tips),
            ("unpaid", file.unpaid, &mut accounts.unpaid),
        ] {
            if let Some(value) = value {
                check(what, &value)?;
                *target = value;
            }
        }
        Ok(accounts)
    }

    fn payment_account(&self, method: Method) -> &str {
        match method {
            Method::GiftVoucher => &self.gift_vouchers,
            m => self
                .payments
                .get(m.code())
                .map_or(self.payments["autre"].as_str(), String::as_str),
        }
    }
}

/// One line of an entry. Exactly one of `debit` and `credit` is non-zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JLine {
    pub account: String,
    pub description: String,
    pub debit: i64,
    pub credit: i64,
    /// VAT label for sales and VAT lines, else empty.
    pub vat_code: String,
}

/// The entry of one booking period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Booking date, `YYYY-MM-DD`.
    pub date: String,
    pub journal: String,
    /// The Z report number.
    pub document: String,
    pub lines: Vec<JLine>,
}

impl Entry {
    pub fn debit(&self) -> i64 {
        self.lines.iter().map(|l| l.debit).sum()
    }

    pub fn credit(&self) -> i64 {
        self.lines.iter().map(|l| l.credit).sum()
    }
}

struct Builder {
    lines: Vec<JLine>,
}

impl Builder {
    /// Add a line; `debit_cents` positive is a debit, negative a credit.
    fn push(&mut self, account: &str, description: &str, debit_cents: i64, vat_code: &str) {
        if debit_cents == 0 {
            return;
        }
        self.lines.push(JLine {
            account: account.to_string(),
            description: description.to_string(),
            debit: debit_cents.max(0),
            credit: (-debit_cents).max(0),
            vat_code: vat_code.to_string(),
        });
    }
}

fn method_name(method: Method) -> &'static str {
    match method {
        Method::Cash => "especes",
        Method::Card => "carte",
        Method::MealVoucher => "titres-repas",
        Method::GiftVoucher => "bons cadeaux utilises",
        Method::App => "applications de commande",
        Method::Online => "paiements en ligne",
        Method::Other => "autres moyens",
    }
}

/// Build the entry of one day.
pub fn entry_of(day: &Day, accounts: &Accounts) -> Result<Entry> {
    let mut b = Builder { lines: Vec::new() };
    let payments = day.payments_by_method();

    // Collected: payments, rounding and tips, per method.
    let mut rounding = 0;
    let mut tips = 0;
    for method in [
        Method::Cash,
        Method::Card,
        Method::MealVoucher,
        Method::GiftVoucher,
        Method::App,
        Method::Online,
        Method::Other,
    ] {
        let mut total = 0;
        for kind in [Kind::Payment, Kind::Rounding, Kind::Tip] {
            let amount = payments.get(&(method, kind)).copied().unwrap_or(0);
            total += amount;
            match kind {
                Kind::Rounding => rounding += amount,
                Kind::Tip => tips += amount,
                Kind::Payment => {}
            }
        }
        b.push(
            accounts.payment_account(method),
            &format!("Encaissements {}", method_name(method)),
            total,
            "",
        );
    }
    // Sales not collected.
    b.push(&accounts.unpaid, "Ventes non encaissees", day.unpaid(), "");

    // Sales and VAT per label, gift vouchers apart.
    let gift = day.gift_voucher_sold();
    b.push(&accounts.gift_vouchers, "Bons cadeaux vendus", -gift, "X");
    for (label, a) in day.vat_by_label() {
        let sales = if label == "X" { a.base - gift } else { a.base };
        let vat = a.total - a.base;
        let sales_account = accounts.sales.get(&label).ok_or_else(|| {
            AccountingError::Accounts(format!("no sales account for VAT label {label}"))
        })?;
        b.push(
            sales_account,
            &format!("Ventes TVA {label}"),
            -sales,
            &label,
        );
        if vat != 0 {
            let vat_account = accounts.vat.get(&label).ok_or_else(|| {
                AccountingError::Accounts(format!("no VAT account for label {label}"))
            })?;
            b.push(vat_account, &format!("TVA collectee {label}"), -vat, &label);
        }
    }
    // Rounding: the cash received differs from the amount due.
    if rounding > 0 {
        b.push(
            &accounts.rounding_income,
            "Arrondis de caisse",
            -rounding,
            "",
        );
    } else if rounding < 0 {
        b.push(&accounts.rounding_cost, "Arrondis de caisse", -rounding, "");
    }
    b.push(&accounts.tips, "Pourboires a reverser", -tips, "");

    let entry = Entry {
        date: day.booking_date.clone(),
        journal: accounts.journal.clone(),
        document: day.z_report_no.to_string(),
        lines: b.lines,
    };
    if entry.debit() != entry.credit() {
        return Err(AccountingError::JournalUnbalanced {
            day: day.booking_date.clone(),
            debit: amount(entry.debit(), Decimal::Point),
            credit: amount(entry.credit(), Decimal::Point),
        });
    }
    Ok(entry)
}

/// The entries of all days, in order.
pub fn entries(days: &[Day], accounts: &Accounts) -> Result<Vec<Entry>> {
    days.iter().map(|d| entry_of(d, accounts)).collect()
}

fn date_parts(date: &str) -> (&str, &str, &str) {
    let mut it = date.splitn(3, '-');
    (
        it.next().unwrap_or_default(),
        it.next().unwrap_or_default(),
        it.next().unwrap_or_default(),
    )
}

/// Render the journal in a profile's layout.
pub fn render(entries: &[Entry], profile: Profile) -> Vec<u8> {
    match profile {
        Profile::Generic => generic(entries),
        Profile::Winbooks => winbooks(entries),
        Profile::Yuki => yuki(entries),
    }
}

fn generic(entries: &[Entry]) -> Vec<u8> {
    let mut t = Table::new(&[
        "date", "journal", "document", "compte", "libelle", "debit", "credit", "code_tva",
    ]);
    for e in entries {
        for l in &e.lines {
            t.push(vec![
                e.date.clone(),
                e.journal.clone(),
                e.document.clone(),
                l.account.clone(),
                l.description.clone(),
                amount(l.debit, Decimal::Point),
                amount(l.credit, Decimal::Point),
                l.vat_code.clone(),
            ]);
        }
    }
    t.render()
}

fn winbooks(entries: &[Entry]) -> Vec<u8> {
    let mut t = Table::new(&[
        "DBKCODE",
        "DBKTYPE",
        "DOCNUMBER",
        "DOCORDER",
        "ACCOUNTGL",
        "BOOKYEAR",
        "PERIOD",
        "DATE",
        "COMMENT",
        "AMOUNTEUR",
        "VATCODE",
    ]);
    for e in entries {
        let (year, month, day) = date_parts(&e.date);
        for (index, l) in e.lines.iter().enumerate() {
            t.push(vec![
                e.journal.clone(),
                "PRI".to_string(),
                e.document.clone(),
                (index + 1).to_string(),
                l.account.clone(),
                year.to_string(),
                month.to_string(),
                format!("{year}/{month}/{day}"),
                l.description.clone(),
                amount(l.debit - l.credit, Decimal::Point),
                l.vat_code.clone(),
            ]);
        }
    }
    t.render()
}

fn yuki(entries: &[Entry]) -> Vec<u8> {
    let mut t = Table::new(&[
        "Date",
        "Journal",
        "Reference",
        "Account",
        "Description",
        "Debit",
        "Credit",
        "VATCode",
    ]);
    for e in entries {
        let (year, month, day) = date_parts(&e.date);
        for l in &e.lines {
            t.push(vec![
                format!("{day}-{month}-{year}"),
                e.journal.clone(),
                e.document.clone(),
                l.account.clone(),
                l.description.clone(),
                amount(l.debit, Decimal::Comma),
                amount(l.credit, Decimal::Comma),
                l.vat_code.clone(),
            ]);
        }
    }
    t.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_chart_of_accounts() {
        let a = Accounts::default();
        assert_eq!(a.sales["A"], "700000");
        assert_eq!(a.vat["B"], "451012");
        assert_eq!(a.payments["especes"], "570000");
        assert_eq!(a.gift_vouchers, "489000");
    }

    #[test]
    fn an_accounts_file_overrides_single_entries() {
        let a = Accounts::from_json(
            r#"{"journal":"CAI","sales":{"A":"701000"},"payments":{"carte":"550000"},"unpaid":"411000"}"#,
        )
        .unwrap();
        assert_eq!(a.journal, "CAI");
        assert_eq!(a.sales["A"], "701000");
        assert_eq!(a.sales["B"], "700100");
        assert_eq!(a.payments["carte"], "550000");
        assert_eq!(a.unpaid, "411000");
    }

    #[test]
    fn an_accounts_file_with_a_typo_is_refused() {
        for bad in [
            r#"{"sale":{"A":"1"}}"#,
            r#"{"sales":{"Z":"1"}}"#,
            r#"{"payments":{"cheque":"1"}}"#,
            r#"{"unpaid":"4 00"}"#,
            r#"{"tips":""}"#,
            "not json",
        ] {
            let err = Accounts::from_json(bad).unwrap_err();
            assert_eq!(err.code(), "accounts_invalid", "{bad}");
        }
    }

    fn entry() -> Entry {
        Entry {
            date: "2026-10-04".into(),
            journal: "VTE".into(),
            document: "7".into(),
            lines: vec![
                JLine {
                    account: "570000".into(),
                    description: "Encaissements especes".into(),
                    debit: 12100,
                    credit: 0,
                    vat_code: String::new(),
                },
                JLine {
                    account: "700000".into(),
                    description: "Ventes TVA A".into(),
                    debit: 0,
                    credit: 10000,
                    vat_code: "A".into(),
                },
                JLine {
                    account: "451021".into(),
                    description: "TVA collectee A".into(),
                    debit: 0,
                    credit: 2100,
                    vat_code: "A".into(),
                },
            ],
        }
    }

    #[test]
    fn the_generic_profile_uses_iso_dates_and_decimal_points() {
        let text = String::from_utf8(render(&[entry()], Profile::Generic)).unwrap();
        assert!(text.starts_with('\u{feff}'));
        assert!(text.contains("date;journal;document;compte;libelle;debit;credit;code_tva\r\n"));
        assert!(text.contains("2026-10-04;VTE;7;570000;Encaissements especes;121.00;0.00;\r\n"));
        assert!(text.contains("2026-10-04;VTE;7;700000;Ventes TVA A;0.00;100.00;A\r\n"));
    }

    #[test]
    fn the_winbooks_profile_signs_amounts_and_uses_slash_dates() {
        let text = String::from_utf8(render(&[entry()], Profile::Winbooks)).unwrap();
        assert!(
            text.contains(
                "VTE;PRI;7;1;570000;2026;10;2026/10/04;Encaissements especes;121.00;\r\n"
            )
        );
        assert!(
            text.contains("VTE;PRI;7;3;451021;2026;10;2026/10/04;TVA collectee A;-21.00;A\r\n")
        );
    }

    #[test]
    fn the_yuki_profile_uses_day_first_dates() {
        let text = String::from_utf8(render(&[entry()], Profile::Yuki)).unwrap();
        assert!(text.contains("04-10-2026;VTE;7;570000;Encaissements especes;121,00;0,00;\r\n"));
    }

    #[test]
    fn profiles_have_stable_names() {
        assert_eq!(Profile::parse("generic"), Some(Profile::Generic));
        assert_eq!(Profile::parse("winbooks"), Some(Profile::Winbooks));
        assert_eq!(Profile::parse("yuki"), Some(Profile::Yuki));
        assert_eq!(Profile::parse("exact"), None);
    }
}
