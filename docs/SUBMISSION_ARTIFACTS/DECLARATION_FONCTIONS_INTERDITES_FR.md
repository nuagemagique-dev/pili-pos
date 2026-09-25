# Déclaration sur l'honneur - absence de fonctions interdites

Conformément au §III.C du Manuel de certification SCE 2.0
("Fonctionnalités interdites - Système de caisse enregistreuse"),
le soussigné déclare sur l'honneur que le système de caisse
**moyun-pos v1.0.0** (build de certification, `--no-default-features
--features cert`) ne comporte AUCUNE des fonctionnalités interdites
suivantes :

## 1. Suppression d'enregistrements fiscaux

Le système **ne permet pas** la suppression d'un enregistrement
fiscal déjà signé par le FDM.

**Preuves :**

- La table `fiscal_events` ne fait l'objet d'aucune instruction SQL
  `DELETE` dans le code de production.
- Vérification mécanique :
  ```bash
  grep -r "DELETE FROM fiscal_events" src-tauri/ crates/   # → 0 match
  ```
- Aucune commande `#[tauri::command]` n'expose une opération de
  suppression. Cf. `src-tauri/src/commands/mod.rs`.

## 2. Modification d'enregistrements fiscaux

Le système **ne permet pas** la modification d'un enregistrement
fiscal après sa signature par le FDM.

**Preuves :**

- La table `fiscal_events` ne fait l'objet d'aucune instruction SQL
  `UPDATE` dans le code de production (au-delà des
  écritures uniques au moment de l'INSERT initial).
- Vérification mécanique :
  ```bash
  grep -r "UPDATE fiscal_events" src-tauri/ crates/   # → 0 match
  ```
- Le champ `archived` du schéma est en lecture seule et n'est jamais
  muté par l'application (vestige d'un schéma plus ancien, candidat à
  la suppression dans une future migration).
- Les commandes Tauri qui "corrigent" un encaissement
  (`refund_checkout`, `payment_correction`) produisent un **nouvel
  événement fiscal séparé** référençant l'original via `fdmRef` - la
  ligne originale reste intacte.

## 3. Ajout rétroactif d'enregistrements antidatés

Le système **ne permet pas** l'ajout d'un événement fiscal portant
une date/heure antérieure à l'heure courante.

**Preuves :**

- Le champ `posDateTime` de chaque événement est calculé via
  `chrono::Utc::now()` au moment de la construction du `*Input`
  (`crates/pos-fiscal/src/event.rs`) et n'est jamais paramétré par
  l'utilisateur.
- Le système exige une horloge système synchronisée NTP via `chrony`
  (cf. `KIOSK_DEPLOYMENT_CHECKLIST.md`). L'heure ne peut pas être
  réglée manuellement par l'utilisateur du POS (image kiosque sans
  accès root, écran d'administration absent).
- La cohérence inter-événements est en outre validée par le FDM lui-
  même, qui rejette une mutation dont le `posDateTime` est antérieur
  à celui du dernier événement signé.

## 4. Modification d'un compteur fiscal

Le système **ne permet pas** la modification arbitraire d'un compteur
fiscal.

**Preuves :**

- Le compteur est manipulé exclusivement via les méthodes statiques
  `FiscalCounter::peek_next` et `FiscalCounter::next`
  (`crates/pos-fiscal/src/counter.rs`).
- Aucune commande Tauri n'expose d'opération `set_counter` ou
  équivalente.
- L'incrément est atomique avec l'archive (cf.
  `COUNTER_STRATEGY.md`) ; un compteur incrémenté ne peut pas
  exister sans la ligne `fiscal_events` correspondante.
- Décrément impossible : la requête `INSERT ... ON CONFLICT ... DO
  UPDATE current_value = +1` ne diminue jamais la valeur.

## 5. Désactivation programmatique du FDM

Le système **ne permet pas** de désactiver la communication avec le
FDM pour effectuer des ventes "hors-fiscalité".

**Preuves :**

- Toute commande mutant un événement fiscal appelle
  `FdmClient::sign_*` ; un échec FDM en build de certification
  remonte une erreur bloquante (`CheckoutResult { success: false }`,
  message "FDM not connected - cannot complete transaction").
- La feature Cargo `allow-offline` qui permettrait un bypass de cette
  règle est **strictement dev-only** : elle n'est pas activée dans
  la build de certification.
  ```bash
  cargo build --no-default-features --features cert
  # → ne compile PAS `#[cfg(feature = "allow-offline")]`
  ```
- L'URL du FDM est chargée depuis `PosConfig.fdm_url` à chaque appel
  via `FdmClient::new` ; il n'existe pas de variable "FDM disabled"
  dans `PosConfig`.

## 6. Effacement du journal d'événements

Le système **ne permet pas** d'effacer le journal d'événements
fiscaux ni de réinitialiser la base de données via une opération
utilisateur.

**Preuves :**

- L'image kiosque ne contient aucun outil d'administration SQLite
  (pas de binaire `sqlite3`, pas d'écran d'admin DB).
- L'opérateur n'a pas les droits root sur la machine (image scellée).
- La sauvegarde quotidienne avant Z écrit dans un fichier daté
  (`pos-YYYYMMDD-HHMMSS.db`) sans jamais écraser un fichier
  existant.

## 7. Réimpression silencieuse d'un ticket comme s'il était neuf

Le système **ne permet pas** la réimpression d'un ticket original
sans marquage "COPIE".

**Preuves :**

- La commande `copy_ticket`
  (`src-tauri/src/commands/copy.rs::do_copy_ticket`) émet
  obligatoirement un événement C (`signCopy`) auprès du FDM, qui
  appose son propre `eventLabel = "C"` sur la réponse signée.
- Le format de ticket de copie
  (`crates/pos-print/src/receipt.rs::format_copy_receipt`) ajoute la
  mention "**COPIE - pas un original**" en en-tête et le bloc
  control-data porte `eventLabel: C`.
- Depuis l'ajout du ticket numérique (§III.H), cette garantie est
  également imposée au niveau base de données : la table
  `original_prints` (migration `006`) enregistre chaque impression
  d'un original, et `delivery::print_original_once` refuse
  d'imprimer un original déjà présent dans cette table
  (`PrintOutcome::AlreadyPrinted`). La commande de rattrapage
  `print_missing_original` refuse explicitement de réimprimer un
  original déjà sorti (`ERR_ALREADY_PRINTED`) ou déjà envoyé par
  e-mail (`ERR_EMAIL_SENT`), et renvoie vers l'écran "Copie de
  ticket" (événement C) dans les deux cas. L'ancienne possibilité
  d'une réimpression "libre" de l'original a été supprimée par cette
  fonctionnalité, pas seulement contournée par convention d'écran.

## 8. Tickets fiscaux exempts de signature ou de QR code

Le système **ne permet pas** d'imprimer un ticket fiscal exempt du
bloc control-data ou du QR code.

**Preuves :**

- La fonction `format_receipt`
  (`crates/pos-print/src/receipt.rs`) refuse de produire un ticket
  sans `SignResult` valide ; le panic est précédé d'une vérification
  des champs obligatoires.
- Les pré-tickets (Événement P, `signPreBill`) sont visuellement
  distincts et marqués "PRO FORMA - pas un ticket TVA valable".

## 9. Multi-utilisateur sans traçabilité

Le système **ne permet pas** d'effectuer un événement fiscal sans
identifier l'opérateur par son INSZ.

**Preuves :**

- Chaque commande Tauri reçoit un INSZ qui est inséré dans le champ
  `userInsz` de l'événement FDM et dans la colonne `user_insz` de la
  table `fiscal_events`.
- Le format de l'INSZ (11 chiffres + mod-97) est validé par
  `pos_core::user::validate_insz` avant tout appel commande.
- Le robot/automatisme utilise l'INSZ réservé `00000000029` (cf.
  spec POS v2.2 §II.3).

## 10. Données fiscales en dehors du périmètre du FDM

Le système **n'envoie jamais directement** de données fiscales au
SPF Finances : toute la communication réglementaire passe par le FDM
agréé, lequel transmet ensuite les données au cloud SPF Finances
selon le protocole `FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.09`.

**Preuves :**

- Vérification mécanique :
  ```bash
  grep -ri "finances\.fed\.be\|minfin\.fed\.be" src-tauri/ crates/   # → 0 match
  ```
- Le client HTTP `FdmClient` ne pointe que vers une URL configurée
  par l'opérateur (la FDM en LAN). Aucun client SPF Finances
  n'existe dans le code.

---

**Signature de la déclaration** : à apposer numériquement avec le
certificat qualifié au moment de l'envoi du dossier au SPF Finances.

**Distributeur** : Bruno Moyse / nuagemagique.dev, Bruxelles, Belgique.

**Date** : à compléter à l'envoi.
