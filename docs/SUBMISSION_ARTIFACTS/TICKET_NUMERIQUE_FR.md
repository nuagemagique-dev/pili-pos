# Ticket numérique par e-mail

**Référence :** §III.H "Emission des tickets numériques" du manuel de
certification.

> "Bien que la possibilité d'émettre des tickets sous forme numérique
> soit obligatoire, il n'existe pas de spécifications obligatoires
> concernant la mise en œuvre de cette mesure. Veuillez décrire
> comment cette mesure est mise en œuvre dans votre POS et comment la
> remise réelle du ticket numérique peut être garantie."

---

## 1. Vue d'ensemble

`moyun-pos` v1.0.0 émet le ticket numérique par **e-mail**, en plus du
QR Code de vérification imprimé sur tout ticket papier (URL publique
du FDM SPF Finances, cf. §12 ci-dessous). L'opérateur choisit, avant
l'encaissement (ou la copie, ou le remboursement), l'un des trois
supports :

| Support | Effet |
|---|---|
| **Papier** (par défaut) | Comportement inchangé : un ticket papier, pas d'e-mail. |
| **E-mail** | Le ticket est envoyé par e-mail. Selon la configuration et l'état du service (§4), le POS imprime aussi un papier en secours. |
| **Les deux** | Le ticket papier est **toujours** imprimé ; l'e-mail est envoyé en plus, en meilleur effort. |

Le support choisi (`ticketMedium` : `PAPER` / `DIGITAL` / `PAPER_DIGITAL`)
fait partie de la requête **signée** par le FDM (POS spec v2.2 §1) : il
est arrêté **avant** l'appel de signature, jamais après (voir §3). Sans
bloc `digital_ticket` dans `pos.json`, l'option "E-mail" n'est pas
proposée et le POS se comporte comme en v0.x (papier uniquement) — voir
`MANUEL_INSTALLATION_FR.md` §4.2 "Ticket numérique par e-mail
(optionnel)" pour la configuration.

Code source : `src-tauri/src/commands/delivery.rs` (résolution du
support, mise en file, impression/envoi), `crates/pos-mail` (rendu de
l'e-mail, client Scaleway TEM, file d'attente avec réessais),
`crates/pos-db/src/schema.rs` migrations v5 (`ticket_deliveries`) et v6
(`original_prints`).

## 2. Parcours opérateur

1. Au moment de l'encaissement (ou de la copie, ou du remboursement),
   l'opérateur choisit **Papier**, **E-mail** ou **Les deux**. Le choix
   "E-mail" seul n'est proposé que si le terminal est configuré et que
   le service a répondu à une sonde récente (§4).
2. S'il choisit "E-mail" ou "Les deux", un clavier virtuel à l'écran
   permet de saisir l'adresse. Une **adresse invalide ou vide** est
   refusée sur place, avant tout appel au FDM (`ERR_INVALID_EMAIL`) :
   l'opérateur corrige la saisie ou repasse sur "Papier".
3. Un écran de confirmation affiche l'adresse saisie pour que
   l'opérateur (ou le client, en la lui montrant) la valide avant la
   signature.
4. Le POS signe la transaction avec le support retenu, imprime et/ou
   envoie le ticket (§3-§4), puis affiche l'issue : e-mail envoyé,
   e-mail en attente (réessai en tâche de fond), ou repli papier — avec
   un bouton "**Imprimer le ticket**" si aucun papier n'a pu sortir
   (§6).

Si aucun `digital_ticket` n'est configuré, ces écrans n'apparaissent
pas : le flux reste "Papier" implicite, comme avant cette
fonctionnalité.

## 3. Résolution du support (avant signature)

La fonction pure `delivery::resolve_with` (testée unitairement,
`src-tauri/src/commands/delivery.rs::tests`) applique ces règles, dans
l'ordre :

| Cas | Résultat |
|---|---|
| Pas de choix envoyé, ou "Papier" | `PAPER`, pas d'e-mail. L'adresse éventuellement fournie est ignorée. |
| "E-mail" ou "Les deux" mais le terminal n'a pas de configuration `digital_ticket` utilisable (absente, ou clé API illisible au démarrage) | **Refusé** avant signature (`ERR_DIGITAL_UNAVAILABLE`, "Digital ticket is not configured on this terminal"). |
| Adresse manquante, vide ou invalide (`pos_mail::address::validate`) | **Refusé** avant signature (`ERR_INVALID_EMAIL`). L'adresse est acceptée telle quelle après un simple `trim()` ; la validation est pragmatique (un seul `@`, partie locale et domaine non vides, domaine avec un point, labels valides, ≤ 254 caractères) — elle couvre les fautes de frappe réelles au clavier tactile sans exiger une conformité RFC stricte, et accepte les adresses Unicode/IDN. |
| "E-mail" seul, `allow_digital_only=true` **et** une sonde récente (< 120 s) a confirmé le service joignable | `DIGITAL` : e-mail seul, pas de papier — sauf repli (§4). |
| "E-mail" seul dans tout autre cas (config par défaut `allow_digital_only=false`, ou service non confirmé joignable récemment) | `PAPER_DIGITAL` : le client reçoit le papier tout de suite, l'e-mail suit dès que possible. |
| "Les deux" | `PAPER_DIGITAL`, y compris hors ligne — le papier sort toujours, l'e-mail est mis en file. |

`allow_digital_only=false` est la valeur par défaut de
`pos.json::digital_ticket.allow_digital_only` : sur un terminal fraîchement
configuré, "E-mail" seul n'est jamais proposé tant que le distributeur
ne l'active pas explicitement — un ticket dine-in continue d'imprimer
un papier par défaut, ce qui est le choix le plus prudent pour un
terminal fiscal.

Ce choix est ensuite écrit dans le corps signé
(`sale_input.common.ticket_medium` / `copy_input.common.ticket_medium`)
**avant** l'appel au FDM (`checkout.rs`, `refund.rs`, `copy.rs`) : le
support numérique fait partie de ce que le FDM certifie, pas d'une
décision prise après coup.

## 4. Garantie de remise

1. **Mise en file dans la même transaction que l'archive fiscale.**
   Dès que la signature FDM revient, et **avant** que la transaction
   SQLite (compteur + `fiscal_events` + fermeture de commande) ne soit
   validée, une ligne est ajoutée à `ticket_deliveries`
   (`delivery::enqueue_in_tx`, appelé depuis `checkout.rs`,
   `refund.rs` et `copy.rs`, dans le même `BEGIN IMMEDIATE`). Un
   ticket signé avec un support numérique a donc **toujours** une ligne
   de suivi, même en cas de plantage juste après le commit — il ne peut
   pas se perdre silencieusement.
2. **Tentative immédiate, budget de 8 secondes.** Juste après le
   commit, `delivery::deliver` tente l'envoi tout de suite
   (`INLINE_SEND_TIMEOUT = 8 s`) :
   - `DIGITAL` : si l'envoi réussit dans ce budget, c'est terminé (pas
     de papier). Sinon, le POS **imprime immédiatement l'original en
     secours** (même mécanisme que pour un ticket "Papier", §5) — le
     client repart toujours avec un ticket. La ligne de suivi est
     marquée `fallback_printed` (l'adresse en clair est effacée aussitôt,
     §6).
   - `PAPER_DIGITAL` : le papier est imprimé dans tous les cas ;
     l'envoi de l'e-mail est tenté dans la foulée, en bonus.
3. **Réessais en tâche de fond.** Un e-mail non envoyé dans le budget
   de 8 s (ou tenté hors du flux de caisse) reste `pending` et est
   repris par le worker `run_outbox_once`, exécuté toutes les 30
   secondes (`OUTBOX_INTERVAL`, câblé dans `src-tauri/src/main.rs`).
   Backoff exponentiel après chaque échec retryable (réseau, 5xx,
   429) : 1 min, 5 min, 15 min, 60 min, pour un total de 5 tentatives
   avant abandon (`pos_mail::outbox::DeliveryPolicy`). Une erreur
   permanente (4xx hors 429, réponse malformée) abandonne
   immédiatement sans consommer les 90 minutes de réessai. Une ligne
   restée bloquée à `sending` plus de 90 secondes (ex. plantage de
   l'app en plein envoi) est automatiquement remise à `pending` au tour
   suivant.
4. **Reconstruction identique à chaque tentative.** Le worker ne
   reconstruit **pas** l'e-mail depuis le ticket d'origine : il relit
   `ticket_text` et `verification_url`, déjà figés dans la ligne
   `ticket_deliveries` au moment de la mise en file, et appelle
   `pos_mail::render::build_email` — la **même** fonction que celle
   utilisée à l'encaissement. Il n'existe pas de logique de "renvoi"
   séparée à maintenir à jour.
5. **Contre les envois en double.** Chaque tentative commence par une
   réclamation atomique de la ligne (`UPDATE ... WHERE status =
   'pending'`, `pos_db::queries::deliveries::claim`) : si deux passes
   du worker (ou le worker et l'envoi immédiat) se chevauchent, une
   seule gagne la ligne.

## 5. Contenu de l'e-mail

- **Sujet :** `Votre ticket — <nom du restaurant>`.
- **Corps texte :** le ticket **au caractère près identique** à ce qui
  s'imprime sur papier — `pos_mail::render::render_ticket_text`
  appelle `format_receipt` (le même formateur ESC/POS que le papier)
  puis le retranscrit en texte, exactement comme pour les
  transcriptions de `TICKETS_EXEMPLES_FR.md`. Un ticket **T**
  (formation) émis par e-mail conserve donc ses mentions "TRAINING".
- **URL de vérification :** uniquement pour les événements **N**
  (vente, remboursement) — le FDM ne renvoie `verificationUrl` que
  pour ces événements-là (POS spec v2.2 §2.4). Quand elle existe, elle
  est ajoutée comme ligne de texte séparée après le corps du ticket, et
  reprise comme lien cliquable dans la version HTML.
- **Version HTML :** le même texte dans un bloc `<pre>` (échappé), plus
  le lien de vérification s'il existe.
- **QR Code en pièce jointe :** uniquement quand une URL de
  vérification existe (donc jamais pour une copie C ou un ticket de
  formation T) — un PNG (`ticket-qr.png`, module 8 px, plus grand que
  le raster imprimé pour rester lisible sur un écran de téléphone),
  généré par `pos_fiscal::qr::generate_qr_png`. **Aucune pièce jointe
  PDF n'est jamais produite** (vérifié par test).
- Les copies (C) et les tickets de formation (T) sont donc envoyés
  comme le texte du ticket seul, sans lien ni QR — cohérent avec le
  papier, qui ne réimprime pas non plus la `shortSignature` pour ces
  types (cf. `TICKETS_EXEMPLES_FR.md`).

Exemple de transcription complète : *voir TICKETS_EXEMPLES_FR.md* §9.

## 6. Original imprimé une seule fois (POS spec v2.2 §8.1)

La spec impose qu'un ticket **original** ne soit imprimé qu'**une
seule fois** ; toute impression ultérieure doit être une **copie**
(événement C). `moyun-pos` l'applique désormais au niveau de la base :

- Migration v6, table `original_prints` : une ligne par
  `fiscal_event_id` dont l'original a été imprimé (clé primaire =
  `fiscal_event_id`), remplie **après** que l'imprimante a confirmé
  l'impression (`print_original_once`), jamais avant. Table
  append-only (triggers SQL interdisant `DELETE` et `UPDATE`).
- `print_original_once` vérifie d'abord `original_prints` : si
  l'original a déjà été imprimé, il **refuse** de réimprimer
  (`PrintOutcome::AlreadyPrinted`) plutôt que d'imprimer un second
  original silencieusement. C'est ce même mécanisme que le checkout,
  le remboursement et la copie appellent pour sortir le premier papier.
- La commande `print_missing_original` (menu "Imprimer le ticket",
  affichée quand `needs_paper=true` dans l'issue du checkout — c'est-à-
  dire quand un papier était dû mais que l'impression a échoué, ex.
  imprimante sans papier) sert **exclusivement** à rattraper une
  impression manquée, jamais à réimprimer un original déjà sorti :
  - Refusée si l'original a déjà été imprimé
    (`ERR_ALREADY_PRINTED` : *"The original ticket was already
    printed; print a copy (C) instead"*).
  - Refusée pour un ticket `DIGITAL` dont l'e-mail a déjà été envoyé
    (`ERR_EMAIL_SENT` : *"The ticket was sent by email; print a copy
    (C) instead"*) — l'e-mail **est** l'original numérique ; en
    demander une version papier après coup en ferait un second
    original.
  - Sur succès, la ligne `ticket_deliveries` correspondante (s'il y en
    a une) est marquée `fallback_printed` par le même chemin que le
    repli automatique du §4.

Conséquence pour l'écran "Copie de ticket" (`MANUEL_UTILISATEUR_FR.md`
§7) : toute reproduction d'un ticket dont l'original est déjà sorti
(papier ou e-mail) passe par cet écran et produit un événement **C**,
jamais une seconde impression "libre" de l'original.

## 7. Rétention RGPD de l'adresse e-mail

- L'adresse en clair (`ticket_deliveries.recipient`) n'est conservée
  que le temps nécessaire à l'envoi ou au réessai, et **72 heures au
  maximum** dans tous les cas (`DeliveryPolicy::GDPR_RETENTION`,
  purgée par le worker à chaque passage). En pratique la politique de
  réessai atteint un état terminal en ~1h30 maximum ; la purge à 72 h
  couvre le cas résiduel (redémarrage tardif, terminal éteint le
  week-end...).
- L'adresse est effacée **immédiatement**, avant même les 72 h, dès
  qu'elle n'a plus d'usage : envoi réussi (`mark_sent`), ou repli
  papier définitif (`mark_fallback_printed`).
- Un indice masqué (`recipient_hint`, ex. `j***@e***.be` —
  `pos_mail::address::mask`) est conservé indéfiniment pour la
  lisibilité du support/du dossier ; il ne permet pas de reconstituer
  l'adresse d'origine.
- **Aucune adresse en clair dans les journaux applicatifs** ni dans
  l'écran "Tickets numériques" des Paramètres (`list_ticket_deliveries` /
  `DeliveryView`) : cet écran n'expose que l'indice masqué, le statut,
  le nombre de tentatives et la dernière erreur — jamais le champ
  `recipient`. Voir `PROFIL_ENTREPRISE_FR.md` §6 pour la base légale
  (demande du client) et `FEUILLE_ACQUISITION_DONNEES_FR.md` pour ce
  qu'un contrôleur peut lire dans `ticket_deliveries`.

## 8. Configuration

Voir `MANUEL_INSTALLATION_FR.md` §4.2 "Ticket numérique par e-mail
(optionnel)" pour le bloc `digital_ticket` de `pos.json` (Scaleway
Transactional Email : `project_id`, `from_email`, `from_name`,
`api_key_file`, `allow_digital_only`, `region`). Sans ce bloc, le POS
n'affiche pas le choix du support et n'ouvre aucune connexion sortante
au-delà du FDM. Une configuration présente mais inutilisable (fichier
de clé absent ou illisible au démarrage) désactive silencieusement
l'e-mail — le POS journalise l'anomalie et repasse en papier seul,
sans bloquer le démarrage d'un terminal fiscal pour une dépendance
non-fiscale (`MailState::from_config`).

## 9. Comportement hors-ligne et réseau

- Le seul trafic sortant ajouté par cette fonctionnalité est du HTTPS
  (443) vers `api.scaleway.com` — voir `REPONSES_MANUEL_CERT_FR.md`
  §III.B.
- Une sonde de joignabilité (`check_mail_status`, réutilisée si elle a
  moins de 60 s) alimente l'écran de choix : "E-mail" seul n'est
  affiché comme option que si le service a répondu positivement il y
  a moins de 120 s **et** que `allow_digital_only=true`. Dans le cas
  contraire, un choix "E-mail" se résout en `PAPER_DIGITAL` (§3) — le
  client a toujours son papier.
- Une panne du service Scaleway pendant le service n'immobilise jamais
  la caisse : le pire cas est un `PAPER_DIGITAL` où l'e-mail reste en
  file et part dès le retour du service (§4).

## 10. Mock pour l'admission SPF Finances

Pour les démonstrations et les captures d'écran hors connexion
Internet, `crates/pos-fdm-mock` embarque un simulateur minimal de
l'API Scaleway TEM (`crates/pos-fdm-mock/src/tem.rs`) : il répond comme
le service réel (`{"emails":[{...}]}`), écrit chaque e-mail simulé sur
disque (`MOCK_TEM_OUTBOX`, un `.json` + un `.txt` par envoi) et refuse
les requêtes sans `X-Auth-Token` (401), comme l'API réelle. La variable
`MOCK_TEM_FAIL=1` fait répondre 503 à tout (envoi et sonde), pour
démontrer le repli papier sans dépendre d'une vraie panne — voir
`CAPTURES_ECRAN_RUNBOOK_FR.md` pour la capture correspondante.

## 11. QR Code (rappel)

Indépendamment du support choisi pour l'e-mail, **tout** ticket
fiscal continue de porter le QR Code imprimé (Model 2, correction
d'erreur M, URL de 38 caractères fournie par le FDM). La description
POS v2.2 §6.5 cite 25×25 modules (version 2) ; cette version ne peut
pas contenir l'URL (préfixe en minuscules, valeur en base 62, donc mode
octet ISO/IEC 18004, limité à 34 octets en version 2 même avec la
correction la plus faible). `moyun-pos` utilise donc la version 3
(29×29 modules), la plus petite qui contienne les 38 caractères avec
la correction M — point signalé au SPF pour confirmation
(`crates/pos-fiscal/src/qr.rs`).

## 12. Table exigence → code → test

| Exigence | Code | Test |
|---|---|---|
| Choix du support avant signature, encodé dans la requête signée | `commands/delivery.rs::resolve` / `resolve_with` ; `checkout.rs`/`refund.rs`/`copy.rs` (`sale_input.common.ticket_medium = resolved.medium` avant l'appel FDM) | `delivery.rs::tests::{no_request_is_paper, digital_only_when_allowed_and_reachable, digital_becomes_paper_digital_when_not_allowed, digital_becomes_paper_digital_when_offline, paper_digital_is_kept_even_offline}` |
| Adresse invalide/manquante refusée avant signature | `delivery::resolve_with` + `pos_mail::address::validate` | `delivery.rs::tests::{a_missing_or_blank_address_is_refused, a_malformed_address_is_refused, the_address_is_trimmed}` ; `address.rs::tests::rejects_*` ; e2e `spf_e2e_digital::{email_without_setup_is_refused_before_signing, a_bad_address_is_refused_before_signing}` |
| Mise en file dans la même transaction que l'archive fiscale | `delivery::enqueue_in_tx`, appelé dans le `BEGIN IMMEDIATE` de `checkout.rs`/`refund.rs`/`copy.rs` | e2e `spf_e2e_digital::a_failed_outbox_write_rolls_the_whole_sale_back` |
| Envoi immédiat borné à 8 s, repli papier de l'original sinon | `delivery::deliver`, `INLINE_SEND_TIMEOUT` | e2e `spf_e2e_digital::{digital_only_emails_without_printing, a_failed_digital_email_prints_the_original_once, a_rejected_digital_email_also_falls_back_to_paper}` |
| PAPER_DIGITAL imprime toujours, envoie en bonus | `delivery::deliver` (branche `PaperDigital`) | e2e `spf_e2e_digital::{paper_digital_prints_and_emails_the_same_ticket, paper_digital_keeps_paper_and_retries_the_email_later, paper_digital_is_kept_while_the_email_service_is_down}` |
| Réessais avec recul exponentiel, abandon après 5 tentatives | `pos_mail::outbox::{DeliveryPolicy, run_once, attempt_send}` | `outbox.rs::tests::{backoff_follows_the_documented_schedule, backoff_gives_up_at_max_attempts, max_attempts_matches_the_backoff_table_plus_one}` |
| Reprise d'une ligne bloquée en `sending` (crash) | `pos_db::queries::deliveries::recover_stale_sending` | `deliveries.rs::tests::{stale_sending_rows_are_recovered_to_pending, fresh_sending_rows_are_not_recovered}` |
| Réclamation atomique (pas de double envoi) | `pos_db::queries::deliveries::claim` | `deliveries.rs::tests::{claim_transitions_pending_to_sending, a_second_claim_on_the_same_row_fails}` |
| E-mail = même texte que le papier | `pos_mail::render::render_ticket_text` (`format_receipt` partagé) | `render.rs::tests::render_ticket_text_matches_the_printed_receipt` |
| URL/QR uniquement pour les événements N | `pos_mail::render::build_email` | `render.rs::tests::a_ticket_without_url_is_the_text_alone` ; e2e `spf_e2e_digital::a_training_ticket_email_keeps_its_training_marks` |
| Aucune pièce jointe PDF | `pos_mail::render::build_email` | `render.rs::tests::no_pdf_attachment_is_ever_produced` |
| Original imprimé une seule fois | `pos_db::queries::prints`, `delivery::print_original_once` | `prints.rs::tests::a_print_is_recorded_once` ; e2e `spf_e2e_digital::{a_printed_original_can_not_be_printed_again, an_unprinted_original_prints_once}` |
| Rattrapage d'impression refusé si déjà imprimé / e-mail envoyé | `delivery::do_print_missing_original` | e2e `spf_e2e_digital::{printing_the_missing_original_of_a_digital_ticket_stops_its_email, an_emailed_digital_ticket_can_not_be_printed_as_an_original, printing_an_unknown_event_is_refused}` |
| Adresse effacée dès envoi/repli réussi, purge 72 h sinon | `pos_db::queries::deliveries::{mark_sent, mark_fallback_printed, purge_recipients_older_than}` | `deliveries.rs::tests::{mark_sent_nulls_recipient_and_sets_status, mark_fallback_printed_clears_the_recipient_and_stops_retries, purge_nulls_old_recipients, purge_flips_a_failed_row_to_purged}` |
| Adresse jamais exposée en clair dans l'écran de support | `commands::delivery::{DeliveryView, do_list_ticket_deliveries}` (pas de champ `recipient`) | Revue de code : `DeliveryView` ne porte pas de champ `recipient`, seulement `recipient_hint` |
| Copie (C) et remboursement (N) peuvent aussi être envoyés par e-mail | `commands/copy.rs`, `commands/refund.rs` | e2e `spf_e2e_digital::{a_copy_can_be_emailed_only, a_paper_copy_is_printed_and_recorded, a_refund_ticket_can_be_emailed, a_refund_with_a_bad_address_is_refused_before_signing}` |
| Terminal sans configuration se comporte comme avant (papier seul) | `MailState::disabled` / `from_config` | `delivery.rs::tests::{a_disabled_mail_state_is_never_reachable, a_config_without_digital_ticket_disables_mail, a_missing_key_file_disables_mail}` |

**30 tests** end-to-end couvrent ce fichier (`src-tauri/tests/spf_e2e_digital.rs`),
en plus des tests unitaires cités ci-dessus dans `delivery.rs`,
`crates/pos-mail` et `crates/pos-db/src/queries/{deliveries,prints}.rs`.

## 13. Déclaration officielle pour la cert

L'émission numérique du ticket est implémentée par e-mail (Scaleway
Transactional Email), en plus du QR Code de vérification imprimé sur
chaque ticket papier. La remise est garantie par :

- l'écriture de la demande d'envoi dans la même transaction que
  l'archive fiscale (aucun ticket numérique promis ne peut se perdre
  silencieusement) ;
- un envoi immédiat borné, suivi d'un repli papier automatique de
  l'original si l'e-mail n'est pas parti à temps ;
- des réessais en tâche de fond avec recul exponentiel pendant
  ~1h30, journalisés (statut, tentatives, dernière erreur) ;
- l'impossibilité, au niveau base de données, d'imprimer deux fois un
  original (POS spec v2.2 §8.1) — toute reproduction ultérieure est
  une copie C, explicitement marquée comme telle.

Cette implémentation satisfait les deux obligations du §III.H :
- Émission numérique possible : **OUI** (e-mail avec lien + QR de
  vérification, et QR Code imprimé sur tout ticket papier).
- Remise effective garantie : **OUI** (transaction unique, envoi
  borné + repli papier automatique, réessais journalisés, impression
  unique de l'original).
