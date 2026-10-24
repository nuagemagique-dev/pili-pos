//! `LISEZMOI.txt`: the French note that travels with an export.

use crate::aggregate::Day;
use crate::csv::{Decimal, amount};

/// What the note needs to know about the export.
pub struct Note<'a> {
    pub generated_at: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub source_name: &'a str,
    pub source_sha256: &'a str,
    pub manifest_checked: bool,
    pub days: &'a [Day],
    pub left_out: &'a [String],
    pub decimal: Decimal,
    pub profile: &'a str,
}

/// Render the note (UTF-8, LF line ends).
pub fn render(n: &Note<'_>) -> String {
    let mut s = String::new();
    let mut line = |text: &str| {
        s.push_str(text);
        s.push('\n');
    };
    line(&format!("EXPORT COMPTABLE du {} au {}", n.from, n.to));
    line("");
    line(&format!("Genere le : {}", n.generated_at));
    line(&format!("Fichier source : {}", n.source_name));
    line(&format!(
        "Empreinte SHA-256 du fichier source : {}",
        n.source_sha256
    ));
    line(&format!(
        "Dossier d'export de la caisse verifie par son MANIFEST.sha256 : {}",
        if n.manifest_checked {
            "oui"
        } else {
            "non applicable"
        }
    ));
    line("");
    line("1. D'ou viennent les chiffres");
    line("");
    line("Toutes les donnees viennent de l'archive fiscale de la caisse: les tickets de vente,");
    line("les factures et les mouvements de caisse signes par le module de controle (FDM), et les");
    line("rapports Z. Rien n'est recalcule a partir de la carte ni des commandes en cours.");
    line("Les ventes de formation, les copies, les tickets provisoires (pro forma) et les");
    line("presences du personnel ne sont pas lus.");
    line("");
    line("2. Rapprochement avec les rapports Z");
    line("");
    line(
        "Pour chaque periode comptable (un rapport Z), le nombre de tickets, le chiffre d'affaires,",
    );
    line(
        "la TVA de chaque code (base, TVA, total) et les paiements de chaque type sont compares au",
    );
    line("rapport Z signe. Un seul ecart arrete l'export (code de sortie 2, aucun fichier ecrit).");
    line("Periodes incluses, toutes rapprochees au centime:");
    line("");
    for d in n.days {
        line(&format!(
            "  - {} : Z n. {}, {} tickets dont {} remboursements, total {} EUR",
            d.booking_date,
            d.z_report_no,
            d.sales.len(),
            d.refunds(),
            amount(d.turnover(), n.decimal)
        ));
    }
    line("");
    line("3. Periodes non incluses");
    line("");
    if n.left_out.is_empty() {
        line("Aucune: toutes les periodes de l'intervalle sont cloturees par un rapport Z.");
    } else {
        line(
            "Ces periodes ne sont pas cloturees par un rapport Z. Faites le Z puis relancez l'export:",
        );
        for date in n.left_out {
            line(&format!("  - {date}"));
        }
    }
    line("");
    line("4. Fichiers");
    line("");
    line(
        "  jours.csv              une ligne par periode: tickets, remboursements, chiffre d'affaires",
    );
    line("  tva.csv                TVA par periode, canal (sur place, emporter, en ligne) et code");
    line("  paiements.csv          paiements par periode et moyen, arrondis et pourboires a part");
    line("  remboursements.csv     chaque ticket de remboursement");
    line(
        "  factures.csv           chaque facture (informatif: ses tickets sont deja dans les ventes)",
    );
    line("  bons_cadeaux.csv       bons cadeaux vendus (ligne TVA X, rayon GV) et utilises");
    line("  mouvements_caisse.csv  especes mises en caisse ou retirees");
    line("  rapports_z.csv         les rapports Z tels que signes");
    line(&format!(
        "  journal.csv            une ecriture equilibree par periode (profil {})",
        n.profile
    ));
    line("  MANIFEST.sha256       empreinte SHA-256 de chaque autre fichier (verifiable avec");
    line("                         shasum -a 256 -c MANIFEST.sha256)");
    line("");
    line("5. Conventions");
    line("");
    line("  - Fichiers CSV en UTF-8 avec BOM, separateur point-virgule, fins de ligne CRLF.");
    line(&format!(
        "  - Montants en euros avec {} comme separateur decimal; chaque montant est suivi d'une colonne",
        if n.decimal == Decimal::Comma {
            "la virgule"
        } else {
            "le point"
        }
    ));
    line("    _centimes en nombre entier de centimes (a utiliser pour les controles).");
    line("  - Un remboursement est un montant negatif; une sortie de caisse aussi.");
    line(
        "  - Le canal vient du centre de cout signe: table = sur place, aucun = emporter, autre = en ligne.",
    );
    line(
        "  - Le journal utilise des numeros de compte du plan comptable minimum normalise par defaut,",
    );
    line(
        "    modifiables avec un fichier de comptes. Les ventes non encaissees vont au compte clients,",
    );
    line(
        "    les bons cadeaux vendus au compte de bons en circulation (pas en chiffre d'affaires),",
    );
    line("    les arrondis de caisse aux comptes d'arrondi.");
    line(
        "  - Le traitement comptable des bons cadeaux non utilises a l'echeance appartient au comptable.",
    );
    line("");
    line("6. Ce qui n'est jamais dans ces fichiers");
    line("");
    line(
        "Aucun numero national, aucun nom d'employe, aucune adresse e-mail. Le nom et le numero de",
    );
    line("TVA d'un client n'apparaissent que dans factures.csv.");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_names_the_source_and_the_left_out_periods() {
        let text = render(&Note {
            generated_at: "2026-10-17 10:00",
            from: "2026-10-01",
            to: "2026-10-31",
            source_name: "pos.db",
            source_sha256: "ab12",
            manifest_checked: true,
            days: &[],
            left_out: &["2026-10-31".to_string()],
            decimal: Decimal::Comma,
            profile: "generic",
        });
        assert!(text.contains("pos.db"));
        assert!(text.contains("ab12"));
        assert!(text.contains("  - 2026-10-31"));
        assert!(text.contains("verifie par son MANIFEST.sha256 : oui"));
        assert!(!text.contains('\u{2014}'));
    }
}
