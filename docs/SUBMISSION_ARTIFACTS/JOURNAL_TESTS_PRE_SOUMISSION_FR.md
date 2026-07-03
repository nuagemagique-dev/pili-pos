# Journal de répétition pré-soumission

Ce document capture la répétition en direct contre le **FDM virtuel
zwartedoos.be** avant l'envoi du dossier cert à SPF Finances. Selon
le §1202 du manuel cert, le FDM virtuel est une cible acceptable pour
l'admission ; cette répétition prouve le comportement de bout en bout
avec des signatures non-mockées.

Le harness de répétition est dans
`crates/pos-fiscal/tests/e2e_staging.rs` (opt-in via `--ignored`).
La séquence complète est **automatisée** : le test
`staging_basic_set_smoke` rejoue S work-in → P order → P pre-bill →
N vente → N remboursement → C copie → F tiroir → F entrée d'argent →
R rapport X → S work-out, et vérifie pour chaque `SignResult` :
label attendu, `totalCounter` strictement croissant, `eventCounter`
monotone par label, `digitalSignature` présent, et
`shortSignature == SHA-1(digitalSignature)` quand présent.

Le harness a été validé à blanc contre le mock FDM local
(`pos-fdm-mock`) — les 10 événements passent. Seules les credentials
du FDM virtuel manquent pour la répétition réelle.

## Checklist pré-répétition

- [ ] Compte FDM virtuel provisionné (Checkbox / module2 /
      zwartedoos — emails envoyés, en attente).
- [ ] Credentials placés dans `.env.staging` (gitignored).
- [ ] `MOYUN_STAGING_FDM_URL` exporté dans le shell qui lance les tests
      de répétition.
- [ ] Si le FDM virtuel est provisionné pour une identité précise :
      exporter `MOYUN_STAGING_POS_ID`, `MOYUN_STAGING_VAT`,
      `MOYUN_STAGING_EST_NO` en conséquence (défauts = identité de
      test SPF `BE0000000097` / `2000000042`).
- [ ] Sur re-run : bumper `MOYUN_STAGING_TICKET_START` pour garder la
      numérotation strictement croissante côté FDM.
- [ ] Build cert en place :
      `cargo build --release --no-default-features --features cert`.

## Commande de répétition

```bash
MOYUN_STAGING_FDM_URL=https://... \
MOYUN_STAGING_DUMP=1 \
    cargo test -p pos-fiscal --test e2e_staging -- --ignored --nocapture
```

Avec `MOYUN_STAGING_DUMP=1`, chaque paire requête/réponse est écrite
dans `docs/SUBMISSION_ARTIFACTS/evidence/staging/rehearsal_log.md`
(horodaté, URL FDM incluse) — à committer après la répétition réelle
et à référencer depuis ce journal.

Restent **manuels** (hors périmètre du smoke automatisé) : le rapport Z
(réinitialise les compteurs FDM — section 7), le test de
déconnexion/reconnexion (section 9, nécessite l'app complète), et
l'inspection visuelle du ticket imprimé (scan QR).

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
