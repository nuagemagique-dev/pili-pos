<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/JOURNAL_TESTS_PRE_SOUMISSION_FR.md @ 86088e4 -->
# Journal de répétition pré-soumission - pili-pos-android

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)

Ce document capture la répétition en direct contre le **simulateur en ligne du FDM module2** (FDM certifié SCE 2.0) avant l'envoi du dossier à SPF Finances. Le même simulateur sert de FDM à la machine de test à l'Intake (manuel de certification §IV, étape 1 ; connexion décrite dans SCHEMA_ASSEMBLAGE_FR.md §7). La répétition prouve le comportement de bout en bout avec des signatures non mockées.

Aucune répétition en ligne n'a encore eu lieu : tout appel au simulateur attend l'accord explicite du producteur. Le journal ci-dessous est un squelette. Identité proposée sur le simulateur : posId `CPOS0031234567` et un terminalId propre à la caisse Android (`À effectuer (EV-14 : répétition en ligne sur le simulateur, identité du terminal)`). La répétition est jouée avec l'application et le moteur embarqué sur le terminal de test (`À effectuer (EV-15 : terminal réel, ou mention du profil émulé)`).

Les harnais de test du moteur, ignorés par défaut et jamais lancés en CI, sont `crates/pos-engine/tests/live_fdm.rs` (une journée d'exploitation par les vraies commandes) et `crates/pos-fiscal/tests/e2e_staging.rs` (la même séquence au niveau du client FDM). Les appels partent un à un, plafonnés à 60 par exécution (le simulateur est un service partagé).

## Checklist pré-répétition

- [ ] Accès développeur module2 : simulateur `FDM02000425` et posId `CPOS0031234567` autorisé dans ses paramètres.
- [ ] Jeton du simulateur et adresse du FDM livrés dans le paquet de configuration signé du terminal, jamais dans l'APK (SCHEMA_ASSEMBLAGE_FR.md §7).
- [ ] APK final installé (étiquette `android-v1.0.0`) : `À effectuer (EV-01 : empreintes de l'APK signé)`.
- [ ] Jeux de tests de référence : Basic Set 1.8, Reports 1.3, Advanced Set 1.6 (`docs/20260821_*`), configuration de test `UC_TestSetup_20240806`.
- [ ] Depuis le réseau de l'intake (ou un réseau équivalent) : le posId et le jeton du simulateur répondent.

## Commande de répétition

`À effectuer (EV-14 : parcours de la répétition sur le terminal et dossier Report_Testing recevant chaque paire requête et réponse signée)`

Restent **manuels** : le test de déconnexion et reconnexion (section 9) et l'inspection visuelle du ticket imprimé (scan du QR code).

## Journal de répétition

> À remplir au moment de la répétition. Les sections ci-dessous montrent la structure.

### 1. Requête de statut
Date, FDM ID, version SW, buffer utilisé, erreurs, résultat PASS ou FAIL : `À effectuer (EV-14)`.

### 2. Pointage entrée (événement S)
posDateTime, INSZ, compteur S séquentiel, shortSignature, réponse acceptée par le contrôle de cohérence : `À effectuer (EV-14)`.

### 3. Commande (événement P) - table 4
Trois lignes, TVA B. Compteur P, shortSignature (attendu absent pour P) : `À effectuer (EV-14)`.

### 4. Vente (événement N) - même table
Compteur N, shortSignature (attendu présent), fdmRef[0] lié au P, vérifié : `À effectuer (EV-14)`.

### 5. Copie (événement C)
fdmRef[0] vers la vente originale, shortSignature : `À effectuer (EV-14)`.

### 6. Rapport X (R)
Compteur R, ticket imprimé OUI ou NON : `À effectuer (EV-14)`.

### 7. Rapport Z (R + R)
Compteur Turnover-Z, compteur User-Z, commit atomique des deux (archivés tous deux ou aucun), fichier de sauvegarde de la base : `À effectuer (EV-14)`.

### 8. Pointage sortie (événement S)
posDateTime, shortSignature : `À effectuer (EV-14)`.

### 9. Répétition déconnexion / reconnexion
Simuler une coupure réseau ; le paiement bloque ; reconnexion ; l'opération suivante réussit. Une requête envoyée sans réponse est renvoyée à l'identique (même n° de ticket, même contenu) et le FDM renvoie sa signature avec `DUPLICATE_REQUEST`. Automatisé sans FDM réel : `crates/pos-engine/tests/spf_e2e_signing_serialized.rs`, contre `pos-fdm-mock` derrière un relais qui perd les réponses (`crates/pos-engine-testkit/src/fdm_drop.rs`).
Durée de la déconnexion, comportement de l'interface Android (fenêtre bloquante affichée OUI ou NON), renvoi reconnu comme doublon OUI ou NON : `À effectuer (EV-14)`.

### 10. Contrôle des réponses du FDM réel
Pour chaque `SignResult` capturé, le contrôle de cohérence du moteur (`pos_fiscal::response_check::check_sign_result`, exécuté par le test `e2e_staging`) l'a accepté : écho de la requête, opération et label attendus et, pour N, shortSignature égale à Hex(SHA1(Base64Decode(digitalSignature))).

| Événement | Compteur | Vérifié |
|---|---|---|
| S pointage entrée, P commande, N vente, C copie, R rapport X, R Z-turnover, R Z-user, S pointage sortie | À effectuer (EV-14) | À effectuer (EV-14) |

## Go/no-go final

- [ ] Toutes les vérifications de statut passent.
- [ ] Tous les types d'événements exercés de bout en bout.
- [ ] Toutes les réponses du FDM passent le contrôle de cohérence.
- [ ] Compteurs strictement monotones par label.
- [ ] Inspection visuelle du ticket (scan QR + bloc de contrôle lisible).
- [ ] Gestion de déconnexion testée.

Si toutes les cases sont cochées, procéder à la soumission. Sinon, corriger, puis répéter avant l'envoi du dossier.
