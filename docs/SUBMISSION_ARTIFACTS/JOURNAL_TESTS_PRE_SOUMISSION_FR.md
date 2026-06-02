# Journal de répétition pré-soumission

Ce document capture la répétition en direct contre le **FDM virtuel
zwartedoos.be** avant l'envoi du dossier cert à SPF Finances. Selon
le §1202 du manuel cert, le FDM virtuel est une cible acceptable pour
l'admission ; cette répétition prouve le comportement de bout en bout
avec des signatures non-mockées.

Le harness de répétition est dans
`crates/pos-fiscal/tests/e2e_staging.rs` (opt-in via `--ignored`).

## Checklist pré-répétition

- [ ] Compte FDM virtuel zwartedoos.be provisionné.
- [ ] Credentials placés dans `.env.staging` (gitignored).
- [ ] `MOYUN_STAGING_FDM_URL` exporté dans le shell qui lance les tests
      de répétition.
- [ ] Build cert en place :
      `cargo build --release --no-default-features --features cert`.

## Commande de répétition

```bash
MOYUN_STAGING_FDM_URL=https://... \
MOYUN_STAGING_DUMP=1 \
    cargo test --package pos-fiscal --test e2e_staging -- --ignored --nocapture \
    | tee -a docs/SUBMISSION_ARTIFACTS/JOURNAL_TESTS_PRE_SOUMISSION_FR.md
```

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
Vérifié via secp256r1 : À effectuer OUI / NON
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
reconnexion ; le retry réussit avec le même n° de ticket et le FDM
renvoie la même signature.

```
Durée déconnexion : À effectuer s
Comportement UI POS : À effectuer modale bloquante affichée ? OUI / NON
Retry reconnexion :   À effectuer idempotent ? OUI / NON
```

### 10. Vérification de signature

Pour chaque `SignResult` capturé ci-dessus, exécuter
`signature::verify_signature` avec la clé publique secp256r1 publiée
par le FDM (lue depuis la même réponse de Status) et enregistrer :

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
- [ ] Toutes les signatures vérifient localement.
- [ ] Compteurs strictement monotones par label.
- [ ] Inspection visuelle du ticket (scan QR + bloc de contrôle
      lisible).
- [ ] Gestion de déconnexion testée.

Si toutes les cases sont cochées → procéder à la soumission
Phase 12. Sinon → ouvrir des issues, corriger, re-répéter avant
l'envoi de l'email.
