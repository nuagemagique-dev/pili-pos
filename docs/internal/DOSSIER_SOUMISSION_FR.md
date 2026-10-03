# Dossier de soumission - Checklist maîtresse

Checklist de haut niveau de ce qui est remis à SPF Finances à
`secr.gksce@minfin.fed.be`.

## Artefacts

| Artefact | Emplacement | Statut |
|---|---|---|
| Formulaire de demande de certification signé | `docs/original_pdfs/20250307_.../20250115_FORMULAIRE...pdf` (rempli + signé) | À effectuer Phase 12 |
| Image kiosk (clé USB) | `deploy/image/build.sh admission` (CI : `.github/workflows/image.yml`, artefact `kiosk-image-admission`) ; installation selon `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md` | Construite en CI |
| Binaire du simulateur FDM | `pos-fdm-mock`, construit dans le même conteneur linux/amd64 que le binaire certifié | OK Construit |
| ZIP du dossier | `scripts/make_submission_zip.sh` : `target/submission/pili-pos-dossier-v1.0.0.zip` (contenu listé par `docs/SUBMISSION_ARTIFACTS/MANIFEST.txt`, preuves sous `Report_Testing/`) | À régénérer après la build finale |
| Rapport de tests fixtures | `RAPPORT_TESTS_BASELINE_FR.md` + `COUVERTURE_FIXTURES_FR.md` + `COUVERTURE_RAPPORTS_FR.md` + `COUVERTURE_ADVANCED_SET_FR.md` | OK |
| Journal de répétition pré-soumission | `JOURNAL_TESTS_PRE_SOUMISSION_FR.md` | À remplir après la répétition live finale (binaire final, simulateur module2) |
| Archive source | `git archive` du tag cert (`v1.0.0`), fournie sur demande, non jointe d'office | À effectuer après la build finale |

## Déclarations sur le formulaire de demande

Selon le formulaire (`20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM.pdf`) :

| Champ | Valeur |
|---|---|
| Nom du produit | pili-pos |
| Version | 1.0.0 |
| Distributeur | Bruno Moyse / nuagemagique.dev (Bruxelles, BE) |
| Versions de spec | POS v2.2, FDM v1.4, Addendum 2.1, Basic Set v1.8, Reports v1.3, Advanced Set v1.6 (dossiers `docs/20260821_*`), API FPSFIN FDM-FinCloud v1.10 |
| Remboursement partiel mélangé à une vente | NON - nous déclarons **M112B** (le remboursement est émis comme événement fiscal séparé, jamais combiné à des lignes de nouvelle vente). Voir `FLUX_REMBOURSEMENT_FR.md` pour la justification. |
| FDM-BII (interface back-office) | NON (non implémenté ; hors périmètre cert) |
| Multi-FDM | NON (un seul FDM par POS) |
| Peppol B2B (événement I) | NON (prévu v1.1 - mise à jour certifiée post-cert). Le code UBL 2.1 BIS Billing 3.0 existe (`crates/pos-peppol`) mais n'est pas activé dans la build cert v1.0.0 (`cert = ["custom-protocol", "hubrise"]` dans `src-tauri/Cargo.toml` : seule la facturation Peppol est absente ; l'intégration HubRise est dans le binaire, activée ou non par installation via `hubrise_enabled`). En attendant, les factures B2B du restaurant sont émises via son outil de comptabilité/facturation, hors `pili-pos`. *Voir : DECISION_PEPPOL_AP_FR.md*. |
| Stratégie de compteurs | Par type d'événement, documentée dans `STRATEGIE_COMPTEURS_FR.md` |
| Matériel FDM pour l'admission | Simulateur en ligne module2 (PRACSYS), fourni avec la machine de test (manuel §IV, procédure de test, étape 1 : « fournissez un simulateur avec la machine test ») ; connexion et ports dans `SCHEMA_ASSEMBLAGE_FR.md` §7. En production : FDM module2 sur le LAN. |

## Gabarit d'email

```
Sujet : Demande de certification SCE 2.0 - pili-pos v1.0.0

Bonjour,

Veuillez trouver ci-joint la demande de certification pour le
système de caisse "pili-pos" v1.0.0.

Distributeur : Bruno Moyse / nuagemagique.dev (Bruxelles).

Pièces jointes :
- 20250115_FORMULAIRE_DEMANDE_CERTIF_POS-FDM_signe.pdf
- pili-pos-dossier-v1.0.0.zip (sortie de scripts/make_submission_zip.sh ;
  SHA-256 : <à copier de la sortie du script>)
- Lien de téléchargement de l'image kiosque d'admission (USB), valable
  jusqu'au <date + 30 jours, à fixer par le propriétaire> :
  <lien à fournir par le propriétaire>

SHA-1 du binaire pili-pos v1.0.0 (build Linux x86_64 de référence) :
<SHA-1 à copier de deploy/dist/build_info_*.txt>

FDM utilisé pour le rendez-vous d'admission : simulateur en ligne
module2 (PRACSYS), fourni avec la machine de test.

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
| Pré-intake virtuel (procédure, étape 3) | Les deux | réunion en ligne après réception du dossier |
| Invitation au rendez-vous d'admission | SPF Finances | ~ 4 semaines |
| Rendez-vous d'admission (avec machine de test) | Les deux | demi-journée, en présentiel |
| Décision + numéro de cert | SPF Finances | typiquement même semaine que l'admission ou jusqu'à 2 mois plus tard |
| Fenêtre de re-soumission en cas de refus | Distributeur | non précisée par la procédure 2026 ; l'ancienne introduction du manuel parlait de 3 mois (Art. 109), à confirmer auprès du secrétariat |
