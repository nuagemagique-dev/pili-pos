<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/DECISION_PEPPOL_AP_FR.md @ 86088e4 -->
# E-facturation B2B Peppol - Décision d'Access Point et déploiement - pili-pos-android

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)

**État à la date de dépôt :** la facturation B2B Peppol et l'événement fiscal **I (Invoice)** sont **absents de la build Android 1.0**. Ils ne sont ni désactivés par un réglage, ni masqués dans l'interface : le code correspondant n'est pas compilé dans l'application. Un éventuel ajout serait une nouvelle version de `pili-pos-android`, déclarée selon l'article 113 (`REPONSES_MANUEL_CERT_FR.md` §II.E).

## Contexte réglementaire

À partir du 2026-01-01, la facture B2B d'un assujetti à la TVA belge est un document électronique structuré échangé par le réseau Peppol (profil UBL BIS Billing 3.0). Cette obligation est indépendante de la certification SCE 2.0, qui porte sur la piste d'audit entre la caisse et le FDM, pas sur la remise des factures B2B.

## Périmètre à la date de dépôt

Les établissements ciblés sont presque tous en B2C (service en salle et vente à emporter) ; une facture B2B est exceptionnelle. Le dossier déclare donc, comme pour `pili-pos` sur Linux (deux demandes initiales distinctes, `CADRE_DEMANDE_FR.md` §7) : **Peppol et l'événement I sont hors du périmètre de la version 1.0.0**. Les factures B2B de l'établissement sont émises en dehors de la caisse, avec son outil de comptabilité ou de facturation. Le choix d'un Access Point (candidats, tarifs, plan de déploiement, risques) est traité dans `docs/SUBMISSION_ARTIFACTS/DECISION_PEPPOL_AP_FR.md` pour Linux ; il est **sans objet pour la build Android 1.0**.

## État de l'implémentation sur Android

- **Moteur construit sans Peppol.** `android/rust/pos-ffi/Cargo.toml` déclare `pos-engine` avec `default-features = false, features = ["cert"]`. Le jeu de fonctions `cert` active HubRise (commandes en ligne) et laisse la fonction `peppol` éteinte, comme la build de certification Linux (`DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` §8). Le crate `pos-peppol` n'entre donc pas dans la bibliothèque `libpos_ffi.so` de l'APK.
- **Pas de commande Peppol dans l'interface du moteur.** Les commandes d'émission de facture B2B et de signature d'une facture ne font pas partie des commandes appelables depuis Kotlin ; le contrôle `scripts/android/check_command_parity.py --hygiene` (règle H7) refuse tout identifiant `peppol`, `sign_invoice` ou `b2b_invoice` dans la couche `pos-ffi`.
- **Pas de Peppol dans l'arbre des dépendances.** Le contrôle `ffi_deps` de `scripts/android/p2_gate.sh` échoue si `pos-peppol` apparaît dans `cargo tree` de `pos-ffi`. Résultat sur l'APK déclaré : À effectuer (EV-02 : bibliothèque `libpos_ffi.so` de l'APK de dépôt, sans symbole `pos_peppol`).
- **Pas d'écran de facturation.** L'interface Compose n'appelle aucune commande de facture (liste des commandes appelées : `LISTE_FONCTIONNALITES_FR.md` §7).
- **Base de données.** Le schéma est commun aux deux produits : la table `peppol_deliveries` y figure, et la build Android n'y écrit rien (`SECURITE_BD_FR.md` §1.3).

## Pourquoi ce report est acceptable

- L'obligation Peppol est séparée de SCE 2.0 : la revue du SPF Finances porte sur les événements N, P, T, C, F, S et R, pas sur l'événement I, qui est optionnel dans SCE 2.0.
- Le périmètre déclaré dans le formulaire de demande est « E-facturation B2B Peppol : non », pour les deux produits, et se vérifie sur la bibliothèque du moteur de l'APK (aucun symbole `pos_peppol`).
- Aucune facture n'est donc émise ni signée par la caisse Android : le gérant n'a pas de moyen de produire un document Peppol à partir de `pili-pos-android`.
