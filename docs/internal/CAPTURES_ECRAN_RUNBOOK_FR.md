# Runbook de capture d'écrans pour le dossier de certification

**Référence :** §V (comportement aux réponses du FDM) et §VII.D
(rapports X/Z visibles et imprimables) du manuel de certification.

Ce document liste, dans l'ordre d'exécution, les captures d'écran à
produire. Les fichiers vont dans
`docs/SUBMISSION_ARTIFACTS/screenshots/`, puis sont réunis dans
`CAPTURES_ECRAN_FR.pdf`. Les libellés cités sont ceux de l'écran (voir
`MANUEL_UTILISATEUR_FR.md`).

---

## Méthode

Les sections 1 et 2 (captures `01` à `49`, `09a` à `09g`) sont produites
automatiquement par une visite Playwright de l'interface, la couche
IPC Tauri étant simulée (comme le reste de la suite e2e de `ui/`) :
pas de FDM simulé, pas de base SQLite, pas de `cargo tauri dev`.

```bash
cd ui && npm run captures
```

Le test (`ui/e2e/captures.spec.ts`) rejoue chaque écran, vérifie le
texte attendu avant de capturer (un écran cassé fait donc échouer le
test au lieu de produire une image fausse), puis écrit le PNG sous le
nom attendu ci-dessous. Ce n'est pas la suite e2e normale (`npm run
test:e2e`) : `captures.spec.ts` tourne dans son propre projet
Playwright (`--project=captures`) et n'est jamais inclus dans la
passe de test habituelle.

Données de test utilisées par la visite (fictives, sans lien avec un
client réel) : l'établissement du jeu de test SPF (« The FPS Finances
Experience », `CPOS0031234567`, `POS-1-BAR`), employés « Gérant Test »
(INSZ `94060242576`), « Elisa » (INSZ `84022899837`, chef de salle du
jeu de test) et « Apprenti Test » (INSZ `99012345667`) - les rôles
du §1 de `PROFILS_UTILISATEURS_FR.md`.

**Image d'admission (QEMU).** La capture `00_boot_admission.png` (le
premier écran affiché au démarrage de l'image kiosk) n'est pas
couverte par la visite Playwright ci-dessus : elle est produite en CI
à partir de l'image QEMU (`scripts/qemu/run-kiosk.sh`) avec
`scripts/qemu/screendump.sh <nom> 00_boot_admission`, qui capture
l'écran via le socket QMP de l'instance en cours.

La section 3 (tickets imprimés) reste manuelle - voir plus bas.

**Nommage.** `01_` à `19_` : flux fonctionnels ; `09a_` à `09g_` :
ticket numérique ; `20_` à `49_` : messages du FDM (§V) et écrans
additionnels (`45_` à `49_` : assistance à distance, PIN, FDM_ANSWER_PENDING, remboursement par ligne) ; `50_` à `56_` : tickets imprimés.

---

## Section 1 - Écrans de flux fonctionnels

Chaque écran ci-dessous correspond à un état FDM sans erreur ni
avertissement. Les indications « Action » et « Écran attendu »
décrivent l'état capturé ; la visite automatique
(`ui/e2e/captures.spec.ts`) le produit en simulant l'IPC Tauri (pas
de FDM réel ni simulé), et vérifie le texte attendu avant chaque
capture.

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

### `09g_checkout_done_print_ticket.png`
- **Mock :** une vente dont le ticket papier n'est pas encore sorti (`needsPaper`, ex. imprimante hors ligne au moment de l'encaissement).
- **Action :** payer en espèces.
- **Écran attendu :** écran de fin d'encaissement avec le bouton « Imprimer le ticket » (une seule action d'impression : l'original manquant, jamais un second original) et « Terminer ».
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
- **Écran attendu :** bandeau violet "MODE FORMATION · Les transactions ne sont PAS enregistrées" et filigrane "TRAINING".
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

Les entrées `20` à `41` correspondent chacune à un code d'état ou de
warning FDM (colonne "Mock" : la variable du FDM simulé, pour une
reproduction manuelle). Le message s'affiche dans l'en-tête, à côté
de la pastille "Caisse". Les textes ci-dessous sont repris de
`ui/src/utils/fdmMessages.ts`, la source affichée.

La visite automatique (`ui/e2e/captures.spec.ts`) ne relance pas de
FDM simulé : elle simule directement, via l'IPC Tauri, la réponse
`check_fdm_status` (pour la pastille et le message d'en-tête) ou le
refus de `checkout` (pour le message au-dessus des modes de paiement),
puis vérifie le texte exact avant de capturer.

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

### Affichage facultatif (O), affiché par pili-pos

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

### Écrans additionnels (42-49)

#### `42_fdm_response_mismatch.png`
- **Mock :** la réponse du FDM à la vente ne correspond pas à la requête (le contrôle de cohérence de `pos-fiscal::response_check` échoue), ex. `FDM_RESPONSE_MISMATCH: posFiscalTicketNo attendu 42, reçu 41`.
- **Action :** payer par carte : le contrôle échoue, la vente n'est pas enregistrée.
- **Écran attendu :** « Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien. » au-dessus des modes de paiement.
- **Capture :** plein écran.

#### `43_first_login_pin.png`
- **Action :** toucher la tuile d'un Gérant qui n'a encore aucun PIN défini.
- **Écran attendu :** pavé "Choisissez votre PIN" avec le nom de l'employé et une aide de saisie, à la place du pavé de saisie du PIN habituel.
- **Capture :** plein écran.

#### `44_remote_support.png`
- **Action :** "Config", section "Assistance à distance", pendant qu'une session est en cours.
- **Écran attendu :** état "en cours" avec l'employé qui l'a démarrée et l'heure de début, plus le bandeau d'alerte affiché en permanence tant que la session tourne.
- **Capture :** plein écran, section assistance visible (faire défiler si besoin).

#### `45_login_remote_support_banner.png`
- **Action :** sur l'écran de connexion (personne n'est connecté), pendant qu'une session d'assistance à distance est en cours.
- **Écran attendu :** bandeau « Assistance à distance en cours » en bas de l'écran, sans le nom de l'employé qui l'a démarrée, avec le bouton « Arrêter la session » : n'importe qui peut l'arrêter depuis cet écran.
- **Capture :** plein écran.

#### `46_pin_locked.png`
- **Action :** toucher la tuile d'un employé et saisir plusieurs PIN incorrects de suite.
- **Écran attendu :** « Trop de PIN incorrects : réessayez dans 30 s. » (le délai affiché est le délai restant) ; les points du PIN sont vides.
- **Capture :** plein écran.

#### `47_first_pin_needs_manager.png`
- **Action :** toucher la tuile d'un Gérant qui n'a pas encore de PIN alors qu'un autre Gérant en a déjà un, saisir un PIN deux fois.
- **Écran attendu :** « Un gérant doit choisir le PIN de ce compte dans Paramètres. » : le compte ne peut pas choisir son premier PIN lui-même.
- **Capture :** plein écran.

#### `48_fdm_answer_pending.png`
- **Mock :** la réponse du FDM à la vente a été perdue (`FDM_ANSWER_PENDING`) : la caisse renverra la même requête avant d'enregistrer autre chose.
- **Action :** payer par carte.
- **Écran attendu :** « La caisse attend la réponse de la boîte noire pour l'opération précédente. Vérifiez la connexion et réessayez. » au-dessus des modes de paiement ; rien n'est enregistré.
- **Capture :** plein écran.

#### `49_refund_per_line_two_prices.png`
- **Action :** "Rapports", "Tickets", "Rembourser" sur une vente où le même produit figure sur deux lignes à deux prix (12,00 et 9,00 après un changement de prix) ; toucher la ligne à 9,00.
- **Écran attendu :** les deux lignes sont distinctes ; seule la ligne choisie est sélectionnée, et le bouton « Rembourser 9,00 » reprend son prix : chaque ligne est remboursée au prix auquel elle a été vendue.
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

Une fois toutes les captures dans `docs/SUBMISSION_ARTIFACTS/screenshots/`
(sections 1 et 2 via `npm run captures`, `00_boot_admission` via QEMU,
section 3 photographiée) :

```bash
scripts/make_captures_pdf.sh
```

Le script assemble `screenshots/*.png` et `*.jpg`, dans l'ordre du
nom de fichier, en `docs/SUBMISSION_ARTIFACTS/CAPTURES_ECRAN_FR.pdf`
avec `magick` (ImageMagick 7), et échoue clairement s'il ne trouve
aucune capture.

Puis vérifier que `CAPTURES_ECRAN_FR.pdf` figure dans les annexes de
`REPONSES_MANUEL_CERT_FR.md` §V et §VII.D.

---

## Estimation de temps

- Sections 1 et 2 (50 écrans) : quelques secondes, `npm run captures`.
- `00_boot_admission` (QEMU, CI) : automatique, aucune action manuelle.
- Section 3 (7 tickets) : 20 à 30 minutes (impression et photo).
- Compilation (`scripts/make_captures_pdf.sh`) : quelques secondes.

**Total : environ 20 à 30 minutes, dominé par la section 3.**
