//! The text of the owner alerts, in French, Chinese and English.
//!
//! A message carries the installation label, the kind of trouble, the
//! local time it started (Europe/Brussels) and, depending on the kind,
//! a count, the channel names, a booking date or the printers in
//! trouble. It never carries an INSZ, a name, an amount, a ticket or
//! the content of an order (V12-D14): [`Detail`] has no field for them.

use chrono::{DateTime, Utc};
use chrono_tz::Europe::Brussels;

use super::rules::{Decision, Detail, Kind, Printer, PrinterProblem, Transition};

/// The languages of the messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr,
    Zh,
    En,
}

impl Lang {
    /// `fr`, `zh` or `en`; anything else reads French (the default).
    pub fn parse(code: &str) -> Lang {
        match code {
            "zh" => Lang::Zh,
            "en" => Lang::En,
            _ => Lang::Fr,
        }
    }
}

/// `HH:MM` when `at` is on the same Brussels day as `now`, else
/// `dd.mm HH:MM`.
fn local_time(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let at = at.with_timezone(&Brussels);
    let now = now.with_timezone(&Brussels);
    if at.date_naive() == now.date_naive() {
        at.format("%H:%M").to_string()
    } else {
        at.format("%d.%m %H:%M").to_string()
    }
}

/// `2026-10-16` as `16.10.2026`; anything else as it is.
fn local_date(iso: &str, lang: Lang) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    match (lang, parts.as_slice()) {
        (Lang::Zh, [y, m, d]) => format!("{y}年{m}月{d}日"),
        (_, [y, m, d]) => format!("{d}.{m}.{y}"),
        _ => iso.to_string(),
    }
}

fn printer_name(printer: Printer, lang: Lang) -> &'static str {
    match (printer, lang) {
        (Printer::Ticket, Lang::Fr | Lang::En) => "tickets",
        (Printer::Kitchen, Lang::Fr) => "cuisine",
        (Printer::Kitchen, Lang::En) => "kitchen",
        (Printer::Ticket, Lang::Zh) => "小票",
        (Printer::Kitchen, Lang::Zh) => "厨房",
    }
}

fn problem_name(problem: PrinterProblem, lang: Lang) -> &'static str {
    match (problem, lang) {
        (PrinterProblem::Offline, Lang::Fr) => "hors ligne",
        (PrinterProblem::PaperOut, Lang::Fr) => "plus de papier",
        (PrinterProblem::CoverOpen, Lang::Fr) => "capot ouvert",
        (PrinterProblem::Error, Lang::Fr) => "erreur",
        (PrinterProblem::Offline, Lang::En) => "offline",
        (PrinterProblem::PaperOut, Lang::En) => "out of paper",
        (PrinterProblem::CoverOpen, Lang::En) => "cover open",
        (PrinterProblem::Error, Lang::En) => "error",
        (PrinterProblem::Offline, Lang::Zh) => "离线",
        (PrinterProblem::PaperOut, Lang::Zh) => "缺纸",
        (PrinterProblem::CoverOpen, Lang::Zh) => "盖子打开",
        (PrinterProblem::Error, Lang::Zh) => "故障",
    }
}

fn printers_text(detail: &Detail, lang: Lang) -> String {
    detail
        .printers
        .iter()
        .map(|(p, problem)| {
            format!(
                "{} : {}",
                printer_name(*p, lang),
                problem_name(*problem, lang)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The channel names, reduced to plain characters so a name never
/// carries more than a name.
pub fn clean_channel(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_'))
        .take(30)
        .collect::<String>()
        .trim()
        .to_string()
}

/// The sentence about the trouble, `since` being the local time text.
fn body(kind: Kind, detail: &Detail, since: &str, lang: Lang) -> String {
    match (kind, lang) {
        (Kind::Fdm, Lang::Fr) => format!(
            "Le module fiscal (FDM) ne répond plus depuis {since}. Les ventes sont bloquées."
        ),
        (Kind::Fdm, Lang::En) => {
            format!("The fiscal module (FDM) has not answered since {since}. Sales are blocked.")
        }
        (Kind::Fdm, Lang::Zh) => format!("财务模块（FDM）自 {since} 起无响应，无法收银。"),

        (Kind::Printer, Lang::Fr) => {
            format!(
                "Imprimante en défaut depuis {since} ({}).",
                printers_text(detail, lang)
            )
        }
        (Kind::Printer, Lang::En) => {
            format!(
                "Printer problem since {since} ({}).",
                printers_text(detail, lang)
            )
        }
        (Kind::Printer, Lang::Zh) => {
            format!(
                "打印机自 {since} 起故障（{}）。",
                printers_text(detail, lang)
            )
        }

        (Kind::Terminal, Lang::Fr) => {
            format!("Le terminal de carte ne répond plus depuis {since}.")
        }
        (Kind::Terminal, Lang::En) => {
            format!("The card terminal has not answered since {since}.")
        }
        (Kind::Terminal, Lang::Zh) => format!("刷卡终端自 {since} 起无响应。"),

        (Kind::ZMissing, _) => {
            let date = detail
                .booking_date
                .as_deref()
                .map(|d| local_date(d, lang))
                .unwrap_or_default();
            let hour = detail.deadline_hour.unwrap_or(0);
            match lang {
                Lang::Fr => format!("Le Z du {date} n'est pas fait (attendu avant {hour} h)."),
                Lang::En => {
                    format!("The Z report of {date} is not done (expected before {hour}:00).")
                }
                Lang::Zh => format!("{date} 的日结（Z）尚未完成（应在 {hour} 点前完成）。"),
            }
        }

        (Kind::OnlineWaiting, _) => {
            let n = detail.count.unwrap_or(0);
            let minutes = detail.wait_minutes.unwrap_or(0);
            let channels = detail.channels.join(", ");
            match lang {
                Lang::Fr => format!(
                    "{n} commande(s) en ligne attendent depuis plus de {minutes} min ({channels})."
                ),
                Lang::En => format!(
                    "{n} online order(s) have been waiting for more than {minutes} min ({channels})."
                ),
                Lang::Zh => format!("{n} 个在线订单已等待超过 {minutes} 分钟（{channels}）。"),
            }
        }
    }
}

/// The message of a decision. `now` is the time of sending (to decide
/// whether the "since" needs its date).
pub fn message(decision: &Decision, label: &str, lang: Lang, now: DateTime<Utc>) -> String {
    let head = match (decision.transition, lang) {
        (Transition::Raised, Lang::Fr) => "ALERTE",
        (Transition::Reminder, Lang::Fr) => "RAPPEL",
        (Transition::Resolved, Lang::Fr) => "RÉSOLU",
        (Transition::Raised, Lang::En) => "ALERT",
        (Transition::Reminder, Lang::En) => "REMINDER",
        (Transition::Resolved, Lang::En) => "RESOLVED",
        (Transition::Raised, Lang::Zh) => "警报",
        (Transition::Reminder, Lang::Zh) => "提醒",
        (Transition::Resolved, Lang::Zh) => "已恢复",
    };
    let since = local_time(decision.since, now);
    let text = body(decision.kind, &decision.detail, &since, lang);
    if decision.transition == Transition::Resolved {
        let back = match (decision.kind, lang) {
            (Kind::ZMissing, Lang::Fr) => "Le Z est fait.",
            (Kind::ZMissing, Lang::En) => "The Z report is done.",
            (Kind::ZMissing, Lang::Zh) => "日结已完成。",
            (Kind::OnlineWaiting, Lang::Fr) => "Plus de commande en ligne en attente.",
            (Kind::OnlineWaiting, Lang::En) => "No online order is waiting any more.",
            (Kind::OnlineWaiting, Lang::Zh) => "已无等待中的在线订单。",
            (_, Lang::Fr) => "Tout est rentré dans l'ordre.",
            (_, Lang::En) => "Everything is back to normal.",
            (_, Lang::Zh) => "已恢复正常。",
        };
        let was = match lang {
            Lang::Fr => format!("Problème signalé depuis {since}."),
            Lang::En => format!("Problem reported since {since}."),
            Lang::Zh => format!("问题自 {since} 起。"),
        };
        return format!("{head} [{label}]\n{back} {was}");
    }
    format!("{head} [{label}]\n{text}")
}

/// The message of the test button.
pub fn test_message(label: &str, lang: Lang) -> String {
    match lang {
        Lang::Fr => {
            format!("TEST [{label}]\nLes alertes du propriétaire arrivent bien sur ce téléphone.")
        }
        Lang::En => format!("TEST [{label}]\nOwner alerts reach this phone."),
        Lang::Zh => format!("测试 [{label}]\n店主警报已成功发送到此手机。"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> DateTime<Utc> {
        // 2026-10-17 is CEST (UTC+2).
        Utc.with_ymd_and_hms(2026, 10, 17, h, m, 0).unwrap()
    }

    fn decision(kind: Kind, transition: Transition, detail: Detail) -> Decision {
        Decision {
            kind,
            transition,
            since: at(10, 30),
            detail,
        }
    }

    #[test]
    fn the_time_is_brussels_time() {
        assert_eq!(local_time(at(10, 30), at(11, 0)), "12:30");
        assert_eq!(
            local_time(at(10, 30), at(10, 30) + chrono::Duration::days(1)),
            "17.10 12:30"
        );
    }

    #[test]
    fn every_kind_reads_in_every_language() {
        let detail = Detail {
            count: Some(2),
            channels: vec!["webshop".into()],
            wait_minutes: Some(5),
            booking_date: Some("2026-10-16".into()),
            deadline_hour: Some(6),
            printers: vec![(Printer::Kitchen, PrinterProblem::PaperOut)],
        };
        for lang in [Lang::Fr, Lang::En, Lang::Zh] {
            for kind in Kind::ALL {
                for tr in [
                    Transition::Raised,
                    Transition::Reminder,
                    Transition::Resolved,
                ] {
                    let text = message(
                        &decision(kind, tr, detail.clone()),
                        "Salle",
                        lang,
                        at(11, 0),
                    );
                    assert!(text.contains("[Salle]"), "{text}");
                    if matches!(kind, Kind::Fdm | Kind::Printer | Kind::Terminal)
                        || tr == Transition::Resolved
                    {
                        assert!(text.contains("12:30"), "{text}");
                    }
                    assert!(!text.contains('{'), "{text}");
                }
            }
        }
    }

    #[test]
    fn the_online_message_names_the_count_and_the_channels() {
        let detail = Detail {
            count: Some(3),
            channels: vec!["webshop".into(), "deliveroo".into()],
            wait_minutes: Some(5),
            ..Detail::default()
        };
        let text = message(
            &decision(Kind::OnlineWaiting, Transition::Raised, detail),
            "T1",
            Lang::Fr,
            at(11, 0),
        );
        assert_eq!(
            text,
            "ALERTE [T1]\n3 commande(s) en ligne attendent depuis plus de 5 min (webshop, deliveroo)."
        );
    }

    #[test]
    fn the_z_message_names_the_booking_date_and_the_hour() {
        let detail = Detail {
            booking_date: Some("2026-10-16".into()),
            deadline_hour: Some(6),
            ..Detail::default()
        };
        let text = message(
            &decision(Kind::ZMissing, Transition::Reminder, detail),
            "T1",
            Lang::En,
            at(11, 0),
        );
        assert_eq!(
            text,
            "REMINDER [T1]\nThe Z report of 16.10.2026 is not done (expected before 6:00)."
        );
    }

    #[test]
    fn the_printer_message_names_the_printer_and_the_problem() {
        let detail = Detail {
            printers: vec![
                (Printer::Ticket, PrinterProblem::Offline),
                (Printer::Kitchen, PrinterProblem::PaperOut),
            ],
            ..Detail::default()
        };
        let text = message(
            &decision(Kind::Printer, Transition::Raised, detail),
            "T1",
            Lang::Fr,
            at(11, 0),
        );
        assert!(
            text.contains("tickets : hors ligne, cuisine : plus de papier"),
            "{text}"
        );
    }

    #[test]
    fn a_resolve_says_it_is_over_and_since_when() {
        let text = message(
            &decision(Kind::Fdm, Transition::Resolved, Detail::default()),
            "T1",
            Lang::Fr,
            at(11, 0),
        );
        assert_eq!(
            text,
            "RÉSOLU [T1]\nTout est rentré dans l'ordre. Problème signalé depuis 12:30."
        );
    }

    #[test]
    fn a_channel_name_is_reduced_to_a_plain_name() {
        assert_eq!(clean_channel("Uber Eats"), "Uber Eats");
        assert_eq!(clean_channel("a<b>@c/d?e=1"), "abcde1");
        assert_eq!(clean_channel(&"x".repeat(80)).len(), 30);
    }

    #[test]
    fn the_language_defaults_to_french() {
        assert_eq!(Lang::parse("zh"), Lang::Zh);
        assert_eq!(Lang::parse("en"), Lang::En);
        assert_eq!(Lang::parse("fr"), Lang::Fr);
        assert_eq!(Lang::parse("nl"), Lang::Fr);
    }

    #[test]
    fn the_test_message_exists_in_every_language() {
        for lang in [Lang::Fr, Lang::En, Lang::Zh] {
            assert!(test_message("T1", lang).contains("[T1]"));
        }
    }
}
