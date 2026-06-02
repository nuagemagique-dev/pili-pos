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

Ce sont des reports volontaires - la validation fixture ne les
requiert pas, mais ils seront repris dans la build de soumission
cert :

1. **Co-émission de `trigger_end_of_day_z_report`.** L'actuel
   `src-tauri/src/commands/report.rs::generate_z_report` n'émet **que**
   `signReportTurnoverZ`. M184 démontre la double émission canonique
   selon la spec (TurnoverZ + UserZ dans une seule opération). Les
   briques existent déjà (`FdmClient::sign_report_user_z`,
   `ReportUserZInput`) ; ce qui manque c'est (a) une requête DB pour
   les agrégats par utilisateur sur une date de réservation donnée,
   et (b) l'orchestration dans la commande Tauri. **Reporté à la
   Phase 10 (polissage UI + dossier)** pour pouvoir implémenter la
   requête DB, la commande et le bouton "Fin de journée" comme un
   seul changement cohérent plutôt que de l'étaler sur plusieurs
   phases.
2. **Info device FDM dans les payloads Z** est actuellement un
   placeholder dans `build_turnover_data` (fdmId = vide, compteurs = 0).
   Pour passer contre un vrai FDM, il faut le populer depuis la
   dernière réponse `FdmClient::status()`. **Sera traité en Phase 4**
   en parallèle du travail de gating opérationnel FDM + sondage de
   statut.

---

## Reproductibilité

```bash
cd moyun-pos
cargo test --package pos-fiscal --test reports_fixtures -- --nocapture
```

S'exécute également dans le cadre de la suite de tests workspace
complète. Totaux actuels dans le workspace : **190 passants,
0 échec.**
