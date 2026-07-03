# Matrice de couverture des fixtures - SCE 2.0 Reports v1.3

**Date :** 2026-05-14
**Test pilote :** `crates/pos-fiscal/tests/reports_fixtures.rs`
**Fixtures source :** `docs/20250916_UC_Reports_1.3/`

Reports v1.3 ajoute la suite de rapports X/Z au corpus de
certification. Cette phase valide que chaque réponse publiée se
désérialise dans notre `SignResult` et que la dérive de schéma v1.3
(suppression du `reportBookingDate` des rapports X) est reflétée
dans notre `inputs.rs`.

---

## Matrice de couverture

| UC | Événement | Test | Résultat |
|---|---|---|---|
| M180_signReportTurnoverX | R (TurnoverX) | `m180_turnover_x_response_parses` | ✓ |
| M181_signReportTurnoverZ | R (TurnoverZ) | `m181_turnover_z_response_parses` | ✓ |
| M182_signReportUserX | R (UserX) | `m182_user_x_response_parses` | ✓ |
| M183_signReportUserZ | R (UserZ) | `m183_user_z_response_parses` | ✓ |
| M184_signReportTurnoverZUserZ | R (TurnoverZ + UserZ) | `m184_combined_z_response_parses_and_counters_increment` | ✓ |
| InputReport / T1-T8 (événements de staging) | N + P | `input_report_staging_fixtures_parse` | ✓ (8 trouvés) |
| v1.3 - X reports omettent `reportBookingDate` | - | `x_reports_omit_report_booking_date` | ✓ |
| v1.3 - Z reports incluent `reportBookingDate` | - | `z_reports_include_report_booking_date` | ✓ |

**Total :** 8 tests, tous passants.

Combiné avec `uc_fixtures.rs` (Basic Set v1.7), le label d'événement
**R** est désormais exercé, complétant le set des 8 lettres :
N, P, F, S, I, C, T, R.

---

## M184 : émission Z combinée

Le Z de fin de journée mandaté par la spec est une seule opération
GraphQL aliasant à la fois `signReportTurnoverZ` et `signReportUserZ` :

```graphql
mutation M184_signReportTurnoverZUserZ(
  $turnover: ReportTurnoverZInput!
  $user: ReportUserZInput!
  $training: Boolean!
) {
  signReportTurnoverZ(data: $turnover, isTraining: $training) { ...resultFields }
  signReportUserZ(data: $user, isTraining: $training)         { ...resultFields }
}
```

La réponse fixture porte deux payloads `SignResult` dans une seule
enveloppe. Le test assert :

- les deux se désérialisent en `SignResult`,
- les deux sont du label d'événement `R`,
- le `signReportUserZ.fdmRef.totalCounter` est égal à
  `signReportTurnoverZ.fdmRef.totalCounter + 1` - prouve que le FDM
  a signé les deux événements séquentiellement dans la même opération
  atomique,
- les deux réponses référencent le même `fdmId` (un seul FDM dans
  le setup de test).

---

## Dérive de schéma v1.3 - résultat d'audit

| Champ | Changement de spec | Notre `inputs.rs` | Verdict du test |
|---|---|---|---|
| `ReportTurnoverXInput.reportBookingDate` | Supprimé en v1.3 | Champ **absent** dans la struct | ✓ correspond |
| `ReportUserXInput.reportBookingDate` | Supprimé en v1.3 | Champ **absent** dans la struct | ✓ correspond |
| `ReportTurnoverZInput.reportBookingDate` | Requis | `report_booking_date: String` | ✓ correspond |
| `ReportUserZInput.reportBookingDate` | Requis | `report_booking_date: String` | ✓ correspond |

Aucune modification de code n'a été nécessaire pour l'audit v1.3 -
les structs étaient déjà alignées. Les tests fixture attraperont
maintenant toute régression qui ré-introduirait le champ sur les
rapports X.

---

## Écarts d'implémentation suivis

Les deux écarts identifiés lors de l'audit fixture sont **résolus**
dans la build de soumission :

1. **Co-émission M184 — implémentée.**
   `src-tauri/src/commands/report.rs::do_generate_z_report` émet
   désormais le Z canonique : `signReportTurnoverZ` + `signReportUserZ`
   dans **une seule opération GraphQL**
   (`FdmClient::sign_report_turnover_z_user_z`), avec deux numéros de
   ticket séquentiels et un archivage **atomique** des deux événements
   R (transaction SQLite : les deux archivés ou aucun). Les agrégats
   par utilisateur viennent de
   `pos-db::queries::reports::{get_user_activity, get_user_payment_totals,
   get_user_social_events}`. Le rapport utilisateur X (M182) dispose de
   sa propre commande `do_generate_user_x_report`, déclenchée depuis le
   menu Rapports. E2E : `spf_e2e_reports.rs::{m184_*, m182_*}`.
2. **Info device FDM dans les payloads X/Z — réelle.** Le bloc
   `fdmDevices` est peuplé depuis les références FDM archivées de la
   journée (`get_fdm_ref_range` : fdmId + premier/dernier
   `totalCounter`), plus de placeholder. Vérifié par l'assertion
   `fdmDevices[0].fdmId == <identité archivée>` dans le test M180.

---

## Reproductibilité

```bash
cd moyun-pos
cargo test --package pos-fiscal --test reports_fixtures -- --nocapture
```

S'exécute également dans le cadre de la suite de tests workspace
complète. Totaux actuels dans le workspace : **220 passants,
0 échec.**
