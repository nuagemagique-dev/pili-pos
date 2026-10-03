# Journal de répétition pré-soumission

Ce document capture la répétition en direct contre le **simulateur en
ligne du FDM module2** (PRACSYS, FDM certifié SCE 2.0) avant l'envoi
du dossier à SPF Finances. Le même simulateur sert de FDM à la machine
de test à l'Intake : le manuel de certification (§IV, procédure de
test, étape 1) demande de fournir « un simulateur avec la machine
test », et le guide d'introduction (section « Intake ») en demande la
méthode de connexion et les ports (*SCHEMA_ASSEMBLAGE_FR.md* §7). Cette répétition prouve le
comportement de bout en bout avec des signatures non mockées.

Deux harnais, tous deux ignorés par défaut et jamais lancés en CI :

- `src-tauri/tests/live_fdm.rs` : une journée d'exploitation par les
  vraies commandes du POS (pointage entrée, commande de table,
  pré-addition, vente, remboursement, copie, entrée d'argent,
  ouverture du tiroir, comptage de caisse, vente en formation, rapport
  X, rapport X utilisateur, rapport Z, pointage sortie). Pour chaque
  événement : la commande réussit (donc la réponse a passé le contrôle
  de cohérence du POS), compteurs croissants par label et au total,
  shortSignature et URL de vérification sur les seuls événements N, et
  le remboursement et la copie citent la vente signée.
- `crates/pos-fiscal/tests/e2e_staging.rs` : la même séquence au
  niveau du client FDM, avec en plus
  `shortSignature == SHA-1(digitalSignature)` vérifié explicitement.

Les appels partent un à un, espacés d'une seconde, plafonnés à 60 par
exécution (le simulateur est un service partagé). Les numéros de
ticket du POS se poursuivent d'une exécution à l'autre. Les deux
harnais ont été validés à blanc contre le mock FDM local
(`pos-fdm-mock`, avec jeton et liste de posId autorisés).

## Checklist pré-répétition

- [ ] Accès développeur module2 : simulateur `FDM02000425` et posId
      `CPOS0031234567` autorisé dans ses paramètres.
- [ ] `PILI_LIVE_FDM_URL=http://fdm.module2.be/graphql/` et
      `PILI_LIVE_FDM_TOKEN` (jeton du simulateur) placés dans
      `.env.live` (exclu de git).
- [ ] Build cert en place (binaire final, tag `v1.0.0`) :
      `cargo build --release --no-default-features --features cert`
      (la feature `cert` inclut `hubrise`, désactivée par défaut à
      l'installation ; seule la facturation Peppol est absente).
- [ ] Jeux de tests de référence : Basic Set 1.8, Reports 1.3, Advanced
      Set 1.6 (`docs/20260821_*`), configuration de test
      `UC_TestSetup_20240806` (procédure 20260903, point 3.1).
- [ ] Depuis le réseau où aura lieu l'intake (ou un réseau extérieur
      équivalent) : le posId et le jeton du simulateur répondent.

## Commande de répétition

```bash
scripts/live_fdm.sh                                # journée complète (~17 appels)
scripts/live_fdm.sh live_fdm_status_and_one_sale   # statut + une vente (2 appels)
```

Chaque paire requête/réponse signée est écrite dans le dossier
Report_Testing (sous-dossier module2, créé à la première exécution ; la
sortie de cette répétition finale remplace celle du 2026-09-27, qui
précède le binaire final), et la sortie
complète dans `target/live-fdm.log`. Procédure détaillée et liste des
erreurs à provoquer depuis l'onglet « Developer » du portail module2 :
`docs/internal/LIVE_FDM_MODULE2.md`.

Restent **manuels** : le test de déconnexion/reconnexion (section 9,
nécessite l'app complète) et l'inspection visuelle du ticket imprimé
(scan du QR code).

## Journal de répétition

> À remplir au moment de la répétition. Les sections ci-dessous
> montrent la structure.

### 1. Requête de statut

```
Date :          À effectuer
FDM ID :        À effectuer
Version SW :    À effectuer
Buffer utilisé : À effectuer %
Erreurs :       À effectuer
Résultat :      À effectuer PASS / FAIL
```

### 2. Pointage entrée (événement S)

```
posDateTime :       À effectuer
INSZ :              À effectuer
Compteur :          À effectuer (S, séquentiel)
shortSignature :    À effectuer
Réponse acceptée par le contrôle de cohérence : À effectuer OUI / NON
```

### 3. Commande (événement P) - table 4

Trois lignes, TVA B.

```
Compteur :          À effectuer (P)
shortSignature :    À effectuer (attendu absent pour P)
```

### 4. Vente (événement N) - même table

```
Compteur :          À effectuer (N)
shortSignature :    À effectuer (attendu présent)
fdmRef[0] :         À effectuer → lie au P
Vérifié :           À effectuer
```

### 5. Copie (événement C)

```
fdmRef[0] :         À effectuer → vente originale
shortSignature :    À effectuer
```

### 6. Rapport X (R)

```
Compteur :          À effectuer (R)
Ticket imprimé :    À effectuer OUI / NON
```

### 7. Rapport Z (R + R)

```
Compteur Turnover-Z : À effectuer
Compteur User-Z :     À effectuer
Commit atomique :     À effectuer les deux archivés ou aucun
Fichier backup DB :   À effectuer
```

### 8. Pointage sortie (événement S)

```
posDateTime :       À effectuer
shortSignature :    À effectuer
```

### 9. Répétition déconnexion / reconnexion

Simuler une coupure réseau ; assert que le checkout bloque ;
reconnexion ; l'opération suivante réussit. Si la requête avait été
envoyée sans réponse, elle est renvoyée à l'identique (même n° de
ticket, même contenu) et le FDM renvoie sa signature avec
`DUPLICATE_REQUEST` ; sinon le numéro libre est réutilisé par une
nouvelle requête.

Automatisé (sans FDM réel) : `src-tauri/tests/spf_e2e_signing_serialized.rs`,
14 tests (courses entre opérations, réponses perdues, Z manuel puis Z
automatique, renvoi, ligne ouverte) contre `pos-fdm-mock` placé derrière
un relais qui perd les réponses (`src-tauri/tests/common/fdm_drop.rs`).

```
Durée déconnexion : À effectuer s
Comportement UI POS : À effectuer modale bloquante affichée ? OUI / NON
Retry reconnexion :   À effectuer doublon reconnu (DUPLICATE_REQUEST) ? OUI / NON
```

### 10. Contrôle des réponses du FDM réel

Pour chaque `SignResult` capturé ci-dessus, vérifier que le contrôle
de cohérence du POS (`pos_fiscal::response_check::check_sign_result`,
exécuté par le test `e2e_staging`) l'a accepté : écho de la requête,
opération et label attendus et, pour N, shortSignature égale à
Hex(SHA1(Base64Decode(digitalSignature))). Enregistrer :

| Événement | Compteur | Vérifié |
|---|---|---|
| S pointage entrée | À effectuer | À effectuer |
| P commande | À effectuer | À effectuer |
| N vente | À effectuer | À effectuer |
| C copie | À effectuer | À effectuer |
| R rapport X | À effectuer | À effectuer |
| R Z-turnover | À effectuer | À effectuer |
| R Z-user | À effectuer | À effectuer |
| S pointage sortie | À effectuer | À effectuer |

## Go/no-go final

- [ ] Toutes les vérifications de statut passent.
- [ ] Tous les types d'événements exercés end-to-end.
- [ ] Toutes les réponses du FDM passent le contrôle de cohérence.
- [ ] Compteurs strictement monotones par label.
- [ ] Inspection visuelle du ticket (scan QR + bloc de contrôle
      lisible).
- [ ] Gestion de déconnexion testée.

Si toutes les cases sont cochées → procéder à la soumission. Sinon →
corriger, puis répéter à nouveau avant l'envoi du dossier.
