# Dossier de soumission - Checklist maîtresse

Checklist de haut niveau de ce qui est remis à SPF Finances à
`secr.gksce@minfin.fed.be`.

## Artefacts

| Artefact | Emplacement | Statut |
|---|---|---|
| Formulaire de demande de certification signé | `docs/original_pdfs/20250307_.../20250115_FORMULAIRE...pdf` (rempli + signé) | À effectuer Phase 12 |
| Image kiosk (clé USB) | Construite selon `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md` | À effectuer Phase 12 |
| Binaire du simulateur FDM | `pos-fdm-mock` cross-compilé pour amd64 | OK Construit |
| ZIP du dossier | Contenu de `docs/SUBMISSION_ARTIFACTS/` | OK |
| Rapport de tests fixtures | `RAPPORT_TESTS_BASELINE_FR.md` + `COUVERTURE_FIXTURES_FR.md` + `COUVERTURE_RAPPORTS_FR.md` + `COUVERTURE_ADVANCED_SET_FR.md` | OK |
| Journal de répétition pré-soumission | `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` | À effectuer Phase 11 |
| Archive source | git-archive du tag de commit cert | À effectuer Phase 12 |

## Déclarations sur le formulaire de demande

Selon le formulaire (`20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM.pdf`) :

| Champ | Valeur |
|---|---|
| Nom du produit | moyun-pos |
| Version | 1.0.0 |
| Distributeur | Bruno Moyse / nuagemagique.dev (Bruxelles, BE) |
| Versions de spec | POS v2.2, FDM v1.4, Addendum 1, Basic Set v1.7, Reports v1.3, Advanced Set v1.5 |
| Remboursement partiel mélangé à une vente | NON - nous déclarons **M112B** (le remboursement est émis comme événement fiscal séparé, jamais combiné à des lignes de nouvelle vente). Voir `FLUX_REMBOURSEMENT_FR.md` pour la justification. |
| FDM-BII (interface back-office) | NON (non implémenté ; hors périmètre cert) |
| Multi-FDM | NON (un seul FDM par POS) |
| Peppol B2B (événement I) | NON (prévu v1.1 - mise à jour certifiée post-cert). Le code UBL 2.1 BIS Billing 3.0 existe (`crates/pos-peppol`) mais n'est pas activé dans la build cert v1.0.0 (`cert = ["custom-protocol"]` dans `src-tauri/Cargo.toml`). En attendant, les factures B2B du restaurant sont émises via son outil de comptabilité/facturation, hors `moyun-pos`. *Voir : DECISION_PEPPOL_AP_FR.md*. |
| Stratégie de compteurs | Par type d'événement, documentée dans `STRATEGIE_COMPTEURS_FR.md` |
| Matériel FDM pour l'admission | FDM physique de développement Checkbox (TAIGER BV, modèle certifié CBX01) ; émulateur Checkbox en secours (acceptable selon ligne 1202 du manuel de certification) |

## Gabarit d'email

```
Sujet : Demande de certification SCE 2.0 - moyun-pos v1.0.0

Bonjour,

Veuillez trouver ci-joint la demande de certification pour le
système de caisse "moyun-pos" v1.0.0.

Distributeur : Bruno Moyse / nuagemagique.dev (Bruxelles).

Pièces jointes :
- 20250115_FORMULAIRE_DEMANDE_CERTIF_POS-FDM_signe.pdf
- moyun-pos_dossier_cert_2026-05-14.zip
- Lien de téléchargement image kiosque (USB):
  https://... (lien expirant à 30j)

FDM utilisé pour le rendez-vous d'admission : FDM physique de
développement Checkbox (TAIGER BV, modèle certifié CBX01).

Le code source, les tests et le dossier complet sont disponibles
sur demande. Je reste à votre disposition pour fixer le rendez-vous
d'admission.

Cordialement,
Bruno Moyse
cloud@nuagemagique.dev
```

## Calendrier attendu post-soumission

| Étape | Responsable | Délai attendu |
|---|---|---|
| Accusé de réception | SPF Finances | ≤ 5 jours ouvrables |
| Invitation au rendez-vous d'admission | SPF Finances | ~ 4 semaines |
| Rendez-vous d'admission (avec machine de test) | Les deux | demi-journée, en présentiel |
| Décision + numéro de cert | SPF Finances | typiquement même semaine que l'admission ou jusqu'à 2 mois plus tard |
| Fenêtre de re-soumission en cas de refus | Distributeur | délai de 3 mois obligatoire (Art. 109) |
