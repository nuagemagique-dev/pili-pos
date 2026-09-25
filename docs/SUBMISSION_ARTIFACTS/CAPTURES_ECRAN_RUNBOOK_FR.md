# Runbook de capture d'écrans pour le dossier de certification

**Référence :** §V (Comportement aux réponses du FDM) et §VII.D
(Rapports X/Z visibles/imprimables) du manuel de certification.

Ce document liste, dans l'ordre d'exécution, les captures d'écran à
produire à partir de l'application en cours d'exécution. Les fichiers
résultants sont sauvegardés dans
`docs/SUBMISSION_ARTIFACTS/screenshots/` puis compilés dans
`CAPTURES_ECRAN_FR.pdf`.

---

## Pré-requis

1. **Mock FDM disponible.** Le binaire `pos-fdm-mock` doit être
   compilé. Si nécessaire :
   ```bash
   cargo build --package pos-fdm-mock
   ```

1bis. **Mock TEM (ticket numérique, §III.H).** Pour les captures
   `09a`-`09f`, le terminal doit avoir un bloc `digital_ticket`
   configuré (`MANUEL_INSTALLATION_FR.md` §4.2) pointant vers le mock
   TEM embarqué dans `pos-fdm-mock`
   (`crates/pos-fdm-mock/src/tem.rs` - même binaire que le mock FDM,
   pas de service séparé à lancer). `MOCK_TEM_OUTBOX` fixe le
   répertoire où les e-mails simulés sont écrits ; `MOCK_TEM_FAIL=1`
   fait échouer l'envoi/la sonde pour démontrer le repli papier
   (`09e`).

2. **App lancée en mode dev.** Dans un terminal séparé :
   ```bash
   cargo tauri dev
   ```

3. **Format des captures.** Macros macOS :
   - `Cmd + Shift + 4` puis sélection rectangulaire de la fenêtre POS
   - Ou `Cmd + Shift + 4` puis `Espace` puis clic sur la fenêtre pour capturer toute la fenêtre

4. **Convention de nommage.** Tous les fichiers sauvegardés dans
   `docs/SUBMISSION_ARTIFACTS/screenshots/` avec préfixe :
   - `01_` à `19_` pour les écrans de flux fonctionnels
   - `09a_` à `09f_` pour le ticket numérique par e-mail (§III.H,
     intercalés après l'encaissement papier)
   - `20_` à `40_` pour les messages FDM §V

---

## Section 1 - Écrans de flux fonctionnels (§I.C manuels)

Démarrer le mock sans erreur injectée, lancer `cargo tauri dev`, puis :

### `01_login.png`
- **Action :** ouverture initiale de l'app.
- **Écran attendu :** liste des utilisateurs ou pavé numérique INSZ.
- **Capture :** plein écran.

### `02_login_pin.png`
- **Action :** taper l'INSZ du Gérant de test (`94060242576`) puis le PIN `1111`.
- **Écran attendu :** overlay numérique de saisie PIN.
- **Capture :** plein écran.

### `03_tables.png`
- **Action :** après login Admin, écran TablesView.
- **Écran attendu :** grille des tables (numéros, statuts occupé / libre).
- **Capture :** plein écran.

### `04_order_empty.png`
- **Action :** taper Table 4 (vide).
- **Écran attendu :** OrderView avec menu à gauche, panier vide à droite.
- **Capture :** plein écran.

### `05_order_filled.png`
- **Action :** ajouter 2x Saumon roll, 1x Coca, 1x Saké.
- **Écran attendu :** OrderView avec panier rempli, totaux TVA visibles.
- **Capture :** plein écran.

### `06_kitchen_sent.png`
- **Action :** taper "Envoyer en cuisine".
- **Écran attendu :** toast de confirmation + ticket cuisine imprimé (mock).
- **Capture :** plein écran (focus sur le toast).

### `07_checkout_pay_method.png`
- **Action :** taper "Payer".
- **Écran attendu :** CheckoutView avec choix de mode de paiement (Cash / Carte).
- **Capture :** plein écran.

### `08_checkout_cash.png`
- **Action :** Choisir Cash, saisir 50,00 € sur le pavé numérique.
- **Écran attendu :** affichage du rendu de monnaie (arrondi belge).
- **Capture :** plein écran.

### `09_checkout_done.png`
- **Action :** Confirmer le paiement.
- **Écran attendu :** écran de succès + ticket imprimé (signature courte affichée).
- **Capture :** plein écran.

### Ticket numérique par e-mail (§III.H) - à refaire pour une vente distincte, sur un terminal avec `digital_ticket` configuré (mock TEM, §4.2)

### `09a_ticket_medium_picker.png`
- **Action :** taper "Payer" sur une commande, avant le choix du mode de paiement.
- **Écran attendu :** choix du support Papier / E-mail / Les deux (Papier pré-sélectionné).
- **Capture :** plein écran.

### `09b_ticket_medium_email_keyboard.png`
- **Action :** sélectionner "E-mail", taper une adresse sur le clavier virtuel.
- **Écran attendu :** clavier virtuel à l'écran avec l'adresse en cours de saisie.
- **Capture :** plein écran.

### `09c_ticket_medium_email_confirm.png`
- **Action :** valider la saisie.
- **Écran attendu :** écran de confirmation affichant l'adresse saisie avant de continuer.
- **Capture :** plein écran.

### `09d_checkout_done_email_sent.png`
- **Action :** terminer le paiement avec "E-mail" seul, mock TEM démarré normalement (`cargo run --package pos-fdm-mock`).
- **Écran attendu :** message "E-mail envoyé", pas de ticket papier.
- **Capture :** plein écran.

### `09e_checkout_done_paper_fallback.png`
- **Mock :** redémarrer le mock avec `MOCK_TEM_FAIL=1 cargo run --package pos-fdm-mock`, puis refaire une vente avec "E-mail" seul.
- **Écran attendu :** repli papier automatique - le ticket papier sort, message indiquant l'e-mail en attente (réessai en tâche de fond).
- **Capture :** plein écran, si possible avec le ticket papier sorti visible.

### `09f_admin_ticket_deliveries.png`
- **Action :** menu Admin > Paramètres > "Tickets numériques".
- **Écran attendu :** liste des derniers envois avec indice d'adresse masqué (ex. `j***@e***.be`), statut, tentatives - jamais l'adresse en clair.
- **Capture :** plein écran.

### `10_pre_bill.png`
- **Action :** retourner sur une autre table, ajouter des items, taper "Pré-facture".
- **Écran attendu :** OrderView avec mention "PRO FORMA" + ticket pré-facture imprimé.
- **Capture :** plein écran.

### `11_refund.png`
- **Action :** depuis ReportsView, taper "Remboursement", choisir un ticket récent, sélectionner les items à rembourser.
- **Écran attendu :** UI de remboursement avec items cochés, mention "REMBOURSEMENT TOTAL/PARTIEL".
- **Capture :** plein écran.

### `12_copy.png`
- **Action :** depuis ReportsView, sélectionner "Copie de ticket", choisir le ticket M110 récent.
- **Écran attendu :** confirmation + ticket copie imprimé (mention "COPIE - pas un original").
- **Capture :** plein écran.

### `13_training.png`
- **Action :** bouton "Mode formation" (depuis Admin ou ReportsView).
- **Écran attendu :** bandeau "TRAINING" rouge en haut, OrderView avec watermark.
- **Capture :** plein écran.

### `14_reports_x.png`
- **Action :** ReportsView, taper "Rapport X intermédiaire".
- **Écran attendu :** résumé X avec totaux par TVA, par moyen de paiement, par utilisateur.
- **Capture :** plein écran.

### `15_reports_z.png`
- **Action :** ReportsView, taper "Clôture journalière (Z)".
- **Écran attendu :** confirmation + impression du Z + UI marquant la journée close.
- **Capture :** plein écran.

### `16_cash_in_out.png`
- **Action :** ReportsView, ouvrir le panel "Mouvements caisse".
- **Écran attendu :** boutons +/- avec preset montants (fonds de caisse, retraits).
- **Capture :** plein écran.

### `17_drawer_open.png`
- **Action :** taper "Ouvrir tiroir" (event F-WAR).
- **Écran attendu :** toast de confirmation + ouverture physique du tiroir (visible sur l'imprimante connectée si disponible).
- **Capture :** plein écran.

### `18_work_out.png`
- **Action :** menu utilisateur > "Pointer sortie".
- **Écran attendu :** toast de confirmation + retour à l'écran de login.
- **Capture :** plein écran.

### `19_admin_users.png`
- **Action :** menu Admin > Gestion des utilisateurs.
- **Écran attendu :** liste des utilisateurs avec rôles, possibilité d'ajouter/modifier.
- **Capture :** plein écran.

---

## Section 2 - Messages FDM (§V manuel)

Pour chaque capture, **arrêter le mock**, le **redémarrer avec la variable d'environnement appropriée**, **redémarrer/rafraîchir le POS pour qu'il refasse un appel Status**, et capturer.

### Mandatoires (M dans le manuel)

#### `20_buffer_full.png`
- **Mock :** `MOCK_FDM_ERROR=BUFFER_FULL cargo run --package pos-fdm-mock`
- **Action côté POS :** rafraîchir le statut FDM ou tenter un checkout.
- **Écran attendu :** modale bloquante "La boîte noire est pleine. Contactez votre technicien." + pilule FDM rouge dans le bandeau.
- **Capture :** plein écran avec modale visible.

#### `21_fdm_locked.png`
- **Mock :** `MOCK_FDM_ERROR=FDM_LOCKED cargo run --package pos-fdm-mock`
- **Action :** idem.
- **Écran attendu :** modale "Le FDM est verrouillé. Contactez votre technicien." + pilule rouge.
- **Capture :** plein écran.

#### `22_fdm_not_operational.png`
- **Mock :** `MOCK_FDM_ERROR=FDM_NOT_OPERATIONAL cargo run --package pos-fdm-mock`
- **Action :** idem.
- **Écran attendu :** modale "La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes." + pilule rouge.
- **Capture :** plein écran.

#### `23_unknown_pos.png`
- **Mock :** `MOCK_FDM_ERROR=UNKNOWN_POS cargo run --package pos-fdm-mock`
- **Action :** idem.
- **Écran attendu :** modale "Ce POS n'est pas enregistré sur ce FDM. Vérifiez la configuration."
- **Capture :** plein écran.

#### `24_buffer_near_full.png`
- **Mock :** `MOCK_FDM_WARNING=BUFFER_NEAR_FULL MOCK_FDM_BUFFER_PCT=72.5 cargo run --package pos-fdm-mock`
- **Action :** rafraîchir statut.
- **Écran attendu :** **pilule FDM JAUNE** (warning, pas erreur) avec tooltip "L'utilisation du buffer du FDM dépasse 70%".
- **Capture :** plein écran focus sur la pilule.

#### `25_client_cert_expired.png`
- **Mock :** `MOCK_FDM_WARNING=CLIENT_CERT_EXPIRED cargo run --package pos-fdm-mock`
- **Action :** rafraîchir statut.
- **Écran attendu :** pilule jaune avec tooltip "Le certificat du client a expiré".
- **Capture :** plein écran.

### Optionnels (O dans le manuel)

#### `26_unauthorized.png`
- **Mock :** `MOCK_FDM_ERROR=UNAUTHORIZED cargo run --package pos-fdm-mock`
- **Action :** rafraîchir statut.
- **Écran attendu :** modale "Le POS n'est pas autorisé à parler à ce FDM. Vérifiez la configuration."
- **Capture :** plein écran.

#### `27_invalid_request.png`
- **Mock :** `MOCK_FDM_ERROR=INVALID_REQUEST cargo run --package pos-fdm-mock`
- **Action :** tenter un checkout (l'erreur surgit à l'appel signSale).
- **Écran attendu :** message d'erreur dans la modale de checkout.
- **Capture :** plein écran.

#### `28_internal_error.png`
- **Mock :** `MOCK_FDM_ERROR=INTERNAL_ERROR cargo run --package pos-fdm-mock`
- **Action :** idem.
- **Écran attendu :** message "Le FDM a rencontré un problème technique".
- **Capture :** plein écran.

#### `29_undefined_error.png`
- **Mock :** `MOCK_FDM_ERROR=UNDEFINED_ERROR cargo run --package pos-fdm-mock`
- **Action :** idem.
- **Capture :** plein écran.

#### `30_client_cert_near_expiration.png`
- **Mock :** `MOCK_FDM_WARNING=CLIENT_CERT_NEAR_EXPIRATION cargo run --package pos-fdm-mock`
- **Capture :** pilule jaune + tooltip.

---

## Section 3 - Tickets imprimés (annexe §VI)

Pour les captures de tickets papier, capturer :
- Soit le rendu d'aperçu sur l'écran (avant impression physique).
- Soit le ticket physique sorti de l'imprimante connectée, photographié de face.

Liste :

### `40_ticket_n.jpg`
- Ticket de vente normale (event N) imprimé via M110.

### `41_ticket_n_refund.jpg`
- Ticket de remboursement (event N) imprimé via M111.

### `42_ticket_p_prebill.jpg`
- Ticket pré-facture (event P) imprimé via M123.

### `43_ticket_c_copy.jpg`
- Ticket copie (event C) imprimé via M160.

### `44_ticket_t_training.jpg`
- Ticket formation (event T) imprimé via M171.

### `45_report_x.jpg`
- Ticket de rapport X imprimé via M180.

### `46_report_z.jpg`
- Ticket de rapport Z imprimé via M181.

### `47_kitchen_ticket.jpg`
- Ticket cuisine imprimé via "Envoyer en cuisine" (event P sans prix).

---

## Compilation finale

Une fois toutes les captures dans `docs/SUBMISSION_ARTIFACTS/screenshots/`, regrouper dans un PDF :

```bash
cd docs/SUBMISSION_ARTIFACTS
# Conversion images vers PDF avec ImageMagick (à installer si nécessaire)
magick screenshots/*.png screenshots/*.jpg CAPTURES_ECRAN_FR.pdf
# Ou avec sips (macOS natif, image par image)
# Ou via Aperçu (Cmd+P > PDF)
```

Et ajouter `CAPTURES_ECRAN_FR.pdf` à la liste des annexes dans `REPONSES_MANUEL_CERT_FR.md` §V et §VII.D.

---

## Estimation de temps

- Section 1 (19 écrans flux) : ~30-45 minutes (dépend de la stabilité de l'app).
- Ticket numérique (6 captures `09a`-`09f`) : ~15-20 minutes (inclut un redémarrage du mock avec `MOCK_TEM_FAIL=1`).
- Section 2 (10 captures messages FDM) : ~30 minutes (1-2 min par capture, redémarrage mock entre chaque).
- Section 3 (8 tickets physiques) : ~20-30 minutes (impression + photo).

**Total : 1h45 à 2h15** pour une session complète de capture.
