# Runbook de capture d'écrans pour le dossier de certification

**Référence :** §V (comportement aux réponses du FDM) et §VII.D
(rapports X/Z visibles et imprimables) du manuel de certification.

Ce document liste, dans l'ordre d'exécution, les captures d'écran à
produire à partir de l'application en cours d'exécution. Les fichiers
vont dans `docs/SUBMISSION_ARTIFACTS/screenshots/`, puis sont réunis
dans `CAPTURES_ECRAN_FR.pdf`. Les libellés cités sont ceux de l'écran
(voir `MANUEL_UTILISATEUR_FR.md`).

---

## Pré-requis

1. **FDM simulé.** Dans un terminal :
   ```bash
   cargo run --package pos-fdm-mock        # écoute sur :8081
   ```
   Le même binaire simule le service d'e-mail (TEM) sur la même
   adresse. `MOCK_TEM_OUTBOX=<dossier>` y écrit les e-mails simulés ;
   `MOCK_TEM_FAIL=1` fait échouer l'envoi (capture `09e`).

2. **Base de test et configuration**, dans un dossier jetable :
   ```bash
   export MOYUN_POS_DATA_DIR=/tmp/moyun-captures
   export MOYUN_POS_CONFIG=/tmp/moyun-captures/pos.json
   cargo run --package moyun-pos -- --init-db --seed src-tauri/seed_spf_test.sql
   ```
   `pos.json` : l'identité de test de l'installation
   (`MANUEL_INSTALLATION_FR.md` §4.2) avec `"fdm_url":
   "http://localhost:8081/graphql"`, et pour les captures `09a`-`09f`
   un bloc `digital_ticket` dont `"api_base"` vaut
   `"http://localhost:8081"`. Vérifier avec
   `cargo run --package moyun-pos -- --check-config`.

3. **Application.** Dans un autre terminal, avec les mêmes variables :
   ```bash
   cargo tauri dev
   ```
   Sur l'image d'admission (QEMU, `scripts/qemu/run-kiosk.sh`), les
   mêmes écrans se capturent avec `scripts/qemu/screendump.sh <nom>`.

4. **Comptes de test** (seed `seed_spf_test.sql`) : Gérant
   `94060242576`, PIN `1111` ; Serveur `84022899837`, PIN `2222` ;
   Apprenti `99012345667`, PIN `3333`.

5. **Captures macOS.** `Cmd + Shift + 4`, puis `Espace` et clic sur la
   fenêtre du POS.

6. **Nommage.** `01_` à `19_` : flux fonctionnels ; `09a_` à `09f_` :
   ticket numérique ; `20_` à `41_` : messages du FDM (§V) ; `50_` à
   `56_` : tickets imprimés.

---

## Section 1 - Écrans de flux fonctionnels

FDM simulé démarré sans erreur.

### `01_login.png`
- **Action :** ouvrir l'application.
- **Écran attendu :** nom de l'établissement, une tuile par employé (initiale, nom, profil), lien "Autre (INSZ)".
- **Capture :** plein écran.

### `02_login_pin.png`
- **Action :** toucher la tuile du Gérant.
- **Écran attendu :** pavé "Saisir le PIN" avec 4 points vides. Taper `1111` : la connexion part au 4e chiffre.
- **Capture :** plein écran, avant le 4e chiffre.

### `03_tables.png`
- **Action :** après la connexion, écran des tables.
- **Écran attendu :** grille des tables, "Libre" ou nombre d'articles et total.
- **Capture :** plein écran.

### `04_order_empty.png`
- **Action :** toucher la table 4.
- **Écran attendu :** en-tête (pastilles "Caisse" et "Ticket", nom, "Table 4", heure, boutons), catégories et articles à gauche, panier vide ("Aucun article") à droite.
- **Capture :** plein écran.

### `05_order_filled.png`
- **Action :** ajouter quelques produits du seed, dont un en deux exemplaires.
- **Écran attendu :** panier rempli, détail de la TVA par taux, total, boutons "Addition" et "PAYER".
- **Capture :** plein écran.

### `06_order_registered.png`
- **Action :** toucher "Tables" : la commande est enregistrée auprès du FDM (événement P, sans impression).
- **Écran attendu :** la table 4 apparaît occupée avec son nombre d'articles et son total.
- **Capture :** plein écran.

### `07_checkout_pay_method.png`
- **Action :** revenir sur la table 4, toucher "PAYER".
- **Écran attendu :** total à payer, boutons "CASH" et "CARTE".
- **Capture :** plein écran.

### `08_checkout_cash.png`
- **Action :** "CASH", taper `50` sur le pavé.
- **Écran attendu :** "À payer", "Reçu", "Rendu", raccourcis "Exact" et billets ; total arrondi aux 5 cents avec le montant exact "avant arrondi" si différent.
- **Capture :** plein écran.

### `09_checkout_done.png`
- **Action :** "Valider".
- **Écran attendu :** "OK", type de ticket, total, rendu, identifiant FDM, "Ticket imprimé", bouton "Terminer".
- **Capture :** plein écran.

### Ticket numérique par e-mail (§III.H), sur une nouvelle vente, avec `digital_ticket` configuré

### `09a_ticket_medium_picker.png`
- **Action :** "PAYER" sur une nouvelle commande.
- **Écran attendu :** boutons "Papier" (sélectionné), "E-mail" (si `allow_digital_only`) et "Les deux" au-dessus de "CASH" / "CARTE".
- **Capture :** plein écran.

### `09b_ticket_medium_email_keyboard.png`
- **Action :** "E-mail", taper une adresse.
- **Écran attendu :** clavier "Adresse e-mail du client" avec l'adresse en cours de saisie et les raccourcis `.be`, `.com`, `@gmail.com`.
- **Capture :** plein écran.

### `09c_ticket_medium_email_confirm.png`
- **Action :** valider la saisie.
- **Écran attendu :** "Confirmer l'adresse", l'adresse en grand, boutons "Confirmer l'adresse", "Modifier", "Annuler".
- **Capture :** plein écran.

### `09d_checkout_done_email_sent.png`
- **Action :** "Confirmer l'adresse", puis payer.
- **Écran attendu :** "Ticket envoyé à" suivi de l'adresse masquée ; rien ne s'imprime.
- **Capture :** plein écran.

### `09e_checkout_done_paper_fallback.png`
- **Mock :** redémarrer avec `MOCK_TEM_FAIL=1 cargo run --package pos-fdm-mock`, puis refaire une vente en "E-mail" seul.
- **Écran attendu :** le ticket papier sort à la place de l'e-mail ; l'écran le confirme.
- **Capture :** plein écran, si possible avec le ticket sorti.

### `09f_admin_ticket_deliveries.png`
- **Action :** "Config", section "Tickets numériques".
- **Écran attendu :** "Configuré", "Joignable" ou "Injoignable", et la liste des envois : support, adresse masquée (ex. `j***@e***.be`), statut, "Tentatives", "Dernière erreur". Jamais l'adresse en clair.
- **Capture :** plein écran.

### `10_pre_bill.png`
- **Action :** sur une autre table avec des articles, toucher "Addition".
- **Écran attendu :** "Addition provisoire imprimée" ; l'addition sort marquée "PRO FORMA".
- **Capture :** plein écran.

### `11_refund.png`
- **Action :** "Rapports", "Remboursement", choisir la vente de `09`, toucher un article et ajuster sa quantité.
- **Écran attendu :** "Articles à rembourser", article sélectionné avec sa quantité "/ vendue", bouton "Rembourser" avec le montant.
- **Capture :** plein écran.

### `12_copy.png`
- **Action :** "Rapports", "Copie de ticket".
- **Écran attendu :** "Ticket à copier" : numéro, type ("Vente", "Remboursement", "Addition", "Formation"), table, montant, heure. Toucher la vente de `09`, puis "Valider" : la copie sort marquée "COPIE".
- **Capture :** plein écran, sur la liste.

### `13_training.png`
- **Action :** encaisser ou vider les tables ouvertes, puis "Config", "Mode Formation" (passe à "ACTIF"), puis revenir à la commande.
- **Écran attendu :** bandeau violet "MODE FORMATION : les transactions ne sont PAS enregistrées" et filigrane "TRAINING".
- **Capture :** plein écran. Désactiver ensuite le mode formation.

### `14_reports_x.png`
- **Action :** "Rapports", "Générer X".
- **Écran attendu :** "Rapport généré" ; le rapport X sort.
- **Capture :** plein écran.

### `15_reports_z_confirm.png`
- **Action :** "Générer Z" une fois.
- **Écran attendu :** le bouton passe à l'orange : "Confirmer la clôture ?".
- **Capture :** plein écran. Toucher à nouveau : "Rapport généré (Z #n)", le Z sort et apparaît dans "Historique Z".

### `16_cash_in_out.png`
- **Action :** "Rapports", section "Gestion caisse" : choisir 20 €.
- **Écran attendu :** montants 10 à 200 €, rangées "Fond de caisse / Retrait / Pourboire" en "+" et en "-", "Ouvrir tiroir", "Remboursement", "Copie de ticket".
- **Capture :** plein écran. Toucher "Fond de caisse +" : "Mouvement enregistré".

### `17_drawer_open.png`
- **Action :** "Ouvrir tiroir".
- **Écran attendu :** "Tiroir ouvert" (et le tiroir s'ouvre si l'imprimante est reliée).
- **Capture :** plein écran.

### `18_logout.png`
- **Action :** "Sortir" : la commande en cours est enregistrée et le départ est pointé.
- **Écran attendu :** retour à l'écran de connexion.
- **Capture :** plein écran.

### `19_admin_users.png`
- **Action :** "Config", section "Employés".
- **Écran attendu :** chaque employé (nom, profil, INSZ) avec "PIN" et "Désactiver" ; formulaire "Ajouter" (nom, INSZ, profil Gérant / Serveur / Apprenti).
- **Capture :** plein écran.

---

## Section 2 - Messages du FDM (§V)

Pour chaque capture : arrêter le FDM simulé, le relancer avec la
variable indiquée, puis attendre que la caisse revérifie l'état du FDM
(toutes les 30 s) ou revenir à l'écran de commande. Le message
s'affiche dans l'en-tête, à côté de la pastille "Caisse". Les textes
ci-dessous sont repris de `ui/src/utils/fdmMessages.ts`, la source
affichée.

### Affichage obligatoire (M)

#### `20_buffer_full.png`
- **Mock :** `MOCK_FDM_ERROR=BUFFER_FULL cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" rouge, et à côté : « La boîte noire est pleine. Contactez votre technicien. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `21_fdm_locked.png`
- **Mock :** `MOCK_FDM_ERROR=FDM_LOCKED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" rouge, et à côté : « La boîte noire est verrouillée. Contactez votre technicien. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `22_fdm_not_operational.png`
- **Mock :** `MOCK_FDM_ERROR=FDM_NOT_OPERATIONAL cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" rouge, et à côté : « La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `23_unknown_pos.png`
- **Mock :** `MOCK_FDM_ERROR=UNKNOWN_POS cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" rouge, et à côté : « Cette caisse n'est pas reconnue par la boîte noire. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `24_buffer_near_full.png`
- **Mock :** `MOCK_FDM_WARNING=BUFFER_NEAR_FULL cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « La boîte noire est presque pleine. Programmez une intervention. »
- **Capture :** plein écran.

#### `25_client_cert_expired.png`
- **Mock :** `MOCK_FDM_WARNING=CLIENT_CERT_EXPIRED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Le certificat de la boîte noire a expiré. Contactez votre technicien. »
- **Capture :** plein écran.

### Affichage facultatif (O), affiché par moyun-pos

#### `26_client_cert_near_expiration.png`
- **Mock :** `MOCK_FDM_WARNING=CLIENT_CERT_NEAR_EXPIRATION cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Le certificat de la boîte noire expire bientôt. »
- **Capture :** plein écran.

#### `27_unauthorized.png`
- **Mock :** `MOCK_FDM_ERROR=UNAUTHORIZED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « La caisse n'est pas autorisée à parler à la boîte noire. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `28_invalid_request.png`
- **Mock :** `MOCK_FDM_ERROR=INVALID_REQUEST cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Erreur de communication avec la boîte noire. Contactez votre technicien. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `29_internal_error.png`
- **Mock :** `MOCK_FDM_ERROR=INTERNAL_ERROR cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Erreur technique de la boîte noire. Contactez votre technicien. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `30_undefined_error.png`
- **Mock :** `MOCK_FDM_ERROR=UNDEFINED_ERROR cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Erreur indéfinie de la boîte noire. Contactez votre technicien. »
- **En plus :** tenter un encaissement : il est refusé et le même message s'affiche au-dessus des modes de paiement (rien n'est enregistré).
- **Capture :** plein écran.

#### `31_undefined_other.png`
- **Mock :** `MOCK_FDM_ERROR=UNDEFINED_OTHER cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Erreur de la boîte noire. Contactez votre technicien. »
- **Capture :** plein écran.

#### `32_rtc_sync_failed.png`
- **Mock :** `MOCK_FDM_WARNING=RTC_SYNC_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « L'horloge de la boîte noire n'a pas pu se synchroniser. »
- **Capture :** plein écran.

#### `33_update_urls_failed.png`
- **Mock :** `MOCK_FDM_WARNING=UPDATE_URLS_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Mise à jour des URLs de la boîte noire échouée. »
- **Capture :** plein écran.

#### `34_update_trust_cert_failed.png`
- **Mock :** `MOCK_FDM_WARNING=UPDATE_TRUST_CERT_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Mise à jour des certificats de confiance échouée. »
- **Capture :** plein écran.

#### `35_update_task_list_failed.png`
- **Mock :** `MOCK_FDM_WARNING=UPDATE_TASK_LIST_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Téléchargement des tâches FDM échoué. »
- **Capture :** plein écran.

#### `36_task_feedback_failed.png`
- **Mock :** `MOCK_FDM_WARNING=TASK_FEEDBACK_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « La boîte noire n'a pas pu signaler le résultat d'une tâche. »
- **Capture :** plein écran.

#### `37_nop_failed.png`
- **Mock :** `MOCK_FDM_WARNING=NOP_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Communication intermédiaire avec la boîte noire échouée. »
- **Capture :** plein écran.

#### `38_server_cert_resolve_failed.png`
- **Mock :** `MOCK_FDM_WARNING=SERVER_CERT_RESOLVE_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « Impossible de résoudre le certificat du serveur FDM. »
- **Capture :** plein écran.

#### `39_transaction_upload_failed.png`
- **Mock :** `MOCK_FDM_WARNING=TRANSACTION_UPLOAD_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « La boîte noire échoue à transmettre les transactions au SPF Finances. »
- **Capture :** plein écran.

#### `40_initialization_failed.png`
- **Mock :** `MOCK_FDM_WARNING=INITIALIZATION_FAILED cargo run --package pos-fdm-mock`
- **Action :** attendre la vérification de l'état du FDM (30 s au plus) ou revenir à l'écran de commande.
- **Écran attendu :** pastille "Caisse" orange, et à côté : « L'initialisation de la boîte noire a échoué. »
- **Capture :** plein écran.

#### `41_fdm_unreachable.png`
- **Mock :** arrêter le FDM simulé.
- **Action :** attendre la vérification suivante, puis tenter un encaissement.
- **Écran attendu :** pastille "Caisse" rouge, « Connexion perdue avec la boîte noire. Vérifiez le câble réseau. » dans l'en-tête et au-dessus des modes de paiement ; la vente est refusée.
- **Capture :** plein écran.

---

## Section 3 - Tickets imprimés (annexe §VI)

Photographier de face le ticket sorti de l'imprimante reliée. Les
transcriptions texte de ces tickets sont dans `TICKETS_EXEMPLES_FR.md`.

### `50_ticket_n.jpg`
- Ticket de vente (N), capture `09`.

### `51_ticket_n_refund.jpg`
- Ticket de remboursement (N), capture `11`.

### `52_ticket_p_prebill.jpg`
- Addition (P, "PRO FORMA"), capture `10`.

### `53_ticket_c_copy.jpg`
- Copie (C), capture `12`.

### `54_ticket_t_training.jpg`
- Ticket de formation (T), vente faite pendant la capture `13`.

### `55_report_x.jpg`
- Rapport X, capture `14`.

### `56_report_z.jpg`
- Rapport Z, capture `15`.

---

## Compilation finale

Une fois toutes les captures dans `docs/SUBMISSION_ARTIFACTS/screenshots/` :

```bash
cd docs/SUBMISSION_ARTIFACTS
magick screenshots/*.png screenshots/*.jpg CAPTURES_ECRAN_FR.pdf
```

Puis vérifier que `CAPTURES_ECRAN_FR.pdf` figure dans les annexes de
`REPONSES_MANUEL_CERT_FR.md` §V et §VII.D.

---

## Estimation de temps

- Section 1 (25 écrans) : 45 minutes environ.
- Section 2 (22 captures) : 45 minutes environ (1 à 2 minutes par capture, relance du FDM simulé entre chaque).
- Section 3 (7 tickets) : 20 à 30 minutes (impression et photo).

**Total : 2 h à 2 h 15.**
