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

Les sections 1, 1 bis et 2 (captures `01` à `49`, `01a`, `03a`, `09a` à `09l`, `15a`, `60` à `92`) sont produites
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
ticket numérique ; `09h_` à `09l_`, `01a_`, `03a_`, `15a_` : terminal de paiement lié (EFT) ; `20_` à `49_` : messages du FDM (§V) et écrans
additionnels (`45_` à `49_` : assistance à distance, PIN, FDM_ANSWER_PENDING, remboursement par ligne) ; `50_` à `56_` : tickets imprimés ; `60_` à `92_` : fonctions de la v1.0 (transfert de table, correction de paiement, remise, note de cuisine, factures, produits épuisés, éditeur de carte, rapport de ventes, alertes, allergènes, service en plusieurs temps, bons cadeaux).

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

### Terminal de paiement lié (EFT), `payment_terminal` configuré

Le mock IPC simule le terminal (`eftScript`, `settingsWith({ paymentTerminal })`,
`check_payment_terminal`, `list_unsettled_card_payments` de
`ui/e2e/fixtures/mock-ipc.ts`) : aucun débit réel.

### `09h_checkout_eft_mode.png`
- **Action :** "PAYER", "CARTE" avec un terminal relié.
- **Écran attendu :** choix "Terminal" (actif) / "Saisie manuelle" et l'aide « Le montant est envoyé au terminal à la validation ».
- **Capture :** plein écran.

### `09i_eft_overlay.png`
- **Action :** "Valider" sur une part carte, le terminal attend la carte.
- **Écran attendu :** fenêtre « Paiement sur le terminal », le montant, « Le client présente sa carte sur le terminal », bouton "Annuler".
- **Capture :** plein écran.

### `09j_eft_declined.png`
- **Action :** le terminal refuse le paiement.
- **Écran attendu :** « Paiement refusé », le motif du terminal, « Rien n'a été débité. », bouton "OK".
- **Capture :** plein écran.

### `09k_eft_unknown.png`
- **Action :** le terminal ne répond pas (réponse inconnue).
- **Écran attendu :** « Réponse du terminal inconnue », le message « La carte a peut-être été débitée... », boutons "Vérifier" et "Fermer" ; la vente reste bloquée (« Un paiement par carte attend la réponse du terminal »).
- **Capture :** plein écran.

### `09l_eft_meal_voucher.png` (proposée, à produire par le script de captures puis à ajouter au PDF)
- **Mock :** `eftScript` avec un paiement accepté de marque « Edenred ».
- **Action :** « Carte », terminal, taper un montant partiel, « Valider ».
- **Écran attendu :** la part prise s'affiche « Part 1 · Chèques-repas · Edenred » avec la pastille « Accepté ».
- **Capture :** plein écran.

### `01a_login_terminal_row.png`
- **Action :** écran de connexion avec un terminal relié et joignable.
- **Écran attendu :** panneau "Démarrage" avec la ligne « Terminal de paiement : connecté ».
- **Capture :** plein écran.

### `03a_tables_terminal_card_charged.png`
- **Action :** écran des tables, terminal joignable, un paiement carte accepté pas encore enregistré dans une vente (table 4).
- **Écran attendu :** voyant "Terminal" vert dans l'en-tête et pastille « Carte encaissée » sur la table 4.
- **Capture :** plein écran.

### `15a_reports_z_unsettled_cards.png`
- **Action :** "Rapports", onglet "Journée", avec deux paiements carte encaissés mais pas encore enregistrés (une table, une vente à emporter).
- **Écran attendu :** l'avertissement « Paiements par carte encaissés mais pas encore enregistrés dans une vente : » avec ses lignes, et « La clôture reste possible » ; le bouton Z n'est pas bloqué.
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
- **Action :** "Config", section "Employés", connecté en Gérant. Préparer : un Serveur en service (arrivée enregistrée depuis 12:02), un Apprenti hors service, un employé « Présence seule » ; le lecteur de badges réglé sur « Lecteur USB » et un badge lié au Serveur.
- **Écran attendu :** la note « Personnel sans accès à la caisse (cuisine) : le Gérant enregistre son arrivée et son départ ici. » ; chaque employé (nom, profil, état « En service depuis 12:02 » ou « Hors service ») avec « Arrivée » ou « Départ », "Code PIN" et "Désactiver" ; la ligne « Présence seule » sans bouton "Code PIN" ; l'état du badge (« Badge lié le 17/10/2026 » ou « Aucun badge ») avec « Retirer le badge » ou « Lier un badge » ; pas de bouton d'arrivée sur la ligne du Gérant connecté ; formulaire "Ajouter" (nom, INSZ, profil Gérant / Serveur / Apprenti / Présence seule).
- **Capture :** plein écran.

### `19a_admin_presence.png`
- **Action :** "Config", section "Employés" : toucher « Arrivée » sur la ligne de l'employé « Présence seule ».
- **Écran attendu :** message « Arrivée enregistrée · <nom> » ; la ligne passe à « En service depuis HH:MM » et son bouton devient « Départ ». Variante facultative : FDM débranché, les deux boutons grisés avec « FDM indisponible : pointage impossible ».
- **Capture :** plein écran, message visible.

### `19b_admin_badges.png` (proposée, à produire par le script de captures puis à ajouter au PDF)
- **Action :** "Config", connecté en Gérant, panneau « Lecteur de badges » (troisième colonne). Préparer : lecteur « Lecteur USB », « Connexion par badge » activée, un badge lié, deux lignes dans le journal.
- **Écran attendu :** les choix « Désactivé » et « Lecteur USB » (le second sélectionné), l'interrupteur « Connexion par badge » avec sa note sur le PIN des Gérants, les trois réglages du lecteur USB (écart 50 ms, longueurs 6 et 32), la note de confidentialité (seule une empreinte chiffrée est conservée) et les derniers événements (« Badge lié », « Réglages modifiés », avec le Gérant et l'heure).
- **Capture :** plein écran.

### `19c_badge_enrol.png` (proposée, à produire par le script de captures puis à ajouter au PDF)
- **Action :** "Config", section "Employés" : toucher « Lier un badge » sur la ligne d'un employé sans badge.
- **Écran attendu :** fenêtre « Lier un badge » avec « Présentez le badge de <nom> » et le bouton "Retour". Après le passage du badge, la fenêtre suivante demande le PIN du Gérant connecté.
- **Capture :** plein écran, fenêtre d'attente du badge.

### `03b_badge_toast.png` (proposée, à produire par le script de captures puis à ajouter au PDF)
- **Action :** un Serveur est connecté (écran des tables) ; un collègue passe son badge sur le lecteur.
- **Écran attendu :** message « Arrivée · <nom> · HH:MM » (heure de Bruxelles) ; le Serveur reste sur son écran, sa session n'est pas fermée.
- **Capture :** plein écran, message visible.

### `01b_login_badge.png` (proposée, à produire par le script de captures puis à ajouter au PDF)
- **Action :** ouvrir l'application avec le lecteur de badges activé et la connexion par badge activée.
- **Écran attendu :** les tuiles des employés et, dessous, l'invitation « Ou présentez votre badge ». `01_login.png` reste la capture du réglage par défaut (lecteur désactivé).
- **Capture :** plein écran.

---

## Section 1 bis - Écrans des fonctions de la v1.0 (60 à 92)

Produits par `ui/e2e/captures-features.spec.ts` (`npm run captures`, même projet Playwright, mêmes données de test que la section 1). Chaque test affirme l'écran attendu avant la capture. Les tickets imprimés de ces fonctions (bon cadeau, solde, fiche allergènes, bons de cuisine, note, transfert) figurent dans `TICKETS_EXEMPLES_FR.md`.

### `60_table_transfer_picker.png`
- **Action :** Table 1 ouverte avec un article : toucher « Changer de table », choisir la table 5.
- **Écran attendu :** « Changer de table · Table 1 », la liste des tables libres (la table courante et les tables occupées en sont absentes), la table 5 choisie, « Valider » actif.
- **Capture :** plein écran.

### `61_table_transfer_move_mode.png`
- **Action :** Plan de salle : « Déplacer une table », toucher la table 3 (occupée), puis la table libre 7.
- **Écran attendu :** L'invitation « Table 3 : touchez la table libre » et la confirmation « Déplacer la table 3 vers la table 7 ? ».
- **Capture :** plein écran.

### `62_payment_correction.png`
- **Action :** Rapports, onglet « Tickets » : « Corriger le paiement » sur une vente payée en espèces, choisir « Carte ».
- **Écran attendu :** La ligne de paiement « Espèces · 52,00 € », le résumé « Espèces 52,00 € → Carte 52,00 € » et le bouton « Corriger » ; le code PIN du Gérant est demandé ensuite.
- **Capture :** plein écran.

### `63_discount_dialog.png`
- **Action :** Commande de deux articles (Gérant) : « Remise », 50 %, motif « Repas du personnel ».
- **Écran attendu :** « Remise sur la commande », les onglets %, Montant, Offert, l'aperçu « Total 24,00 € → 12,00 € » et « Appliquer la remise ».
- **Capture :** plein écran.

### `64_cart_discount.png`
- **Action :** La même commande après la remise.
- **Écran attendu :** La ligne « Remise 50 % » sous l'article, le total net 12,00 € et le bouton « Retirer la remise ».
- **Capture :** plein écran.

### `65_kitchen_note.png`
- **Action :** Commande avec un article non encore envoyé : « Note » sur la ligne, saisir « sans wasabi ».
- **Écran attendu :** « Note pour la cuisine », le texte saisi, le compteur 11/60 et « Imprimée en cuisine avec la commande » ; la note s'affiche sous le nom de l'article.
- **Capture :** plein écran.

### `66_reports_tickets_invoice.png`
- **Action :** Rapports, onglet « Tickets » : cocher deux ventes.
- **Écran attendu :** Les cases cochées, « Facture (2) », les quatre onglets Journée, Caisse, Tickets, Ventes, la vente déjà facturée sans case (« Facture 20260001 ») et la liste « Factures ».
- **Capture :** plein écran.

### `67_invoice_customer.png`
- **Action :** « Facture (2) » : saisir le numéro de TVA, « Vérifier », compléter l'adresse.
- **Écran attendu :** Étape « 1 · Client », la puce « Inscrit sur Peppol », le clavier à l'écran et « Continuer » actif.
- **Capture :** plein écran.

### `68_invoice_preview.png`
- **Action :** « Continuer » à l'étape client.
- **Écran attendu :** Étape « 2 · Aperçu » : le client, les tickets repris, les lignes, le détail de la TVA, « Total TTC 39,00 € » et la mention « Déjà payé ».
- **Capture :** plein écran.

### `69_invoice_result.png`
- **Action :** « Établir et imprimer la facture ».
- **Écran attendu :** Étape « 3 · Facture » : « Facture 20260002 établie », « Facture imprimée. » et « Envoi Peppol en file d'attente. ».
- **Capture :** plein écran.

### `70_invoice_peppol_required.png`
- **Action :** Même parcours sur une installation sans section `peppol`, client à numéro de TVA belge.
- **Écran attendu :** « La facture n'a pas été établie. » et le motif en français (la facture doit passer par Peppol) ; aucun code technique affiché.
- **Capture :** plein écran.

### `71_invoice_list.png`
- **Action :** Rapports, onglet « Tickets », liste « Factures » en bas de page.
- **Écran attendu :** Quatre factures avec l'état Peppol de chacune : en attente, remise, bloquée, papier seulement.
- **Capture :** plein écran.

### `72_settings_peppol.png`
- **Action :** Config, panneau « Peppol » avec un envoi bloqué.
- **Écran attendu :** Fournisseur, identifiant d'envoi, compteurs (en attente, remises, bloquées), l'envoi bloqué avec son motif et le bouton « Réessayer ».
- **Capture :** plein écran.

### `73_soldout_mode.png`
- **Action :** Commande : « Épuisé ».
- **Écran attendu :** Le mode actif avec « Touchez un produit pour changer sa disponibilité. », deux produits grisés avec la pastille « Épuisé », « Produits épuisés (2) ».
- **Capture :** plein écran.

### `74_soldout_dialog.png`
- **Action :** Mode « Épuisé » : toucher un produit épuisé pour la journée.
- **Écran attendu :** La disponibilité du produit, l'état « Jusqu'à la fin de la journée (18/10 04:00) », les trois choix et « Retour ».
- **Capture :** plein écran.

### `75_soldout_list.png`
- **Action :** Commande : « Produits épuisés (2) ».
- **Écran attendu :** La liste des deux produits avec leur durée et le bouton « Remettre en vente ».
- **Capture :** plein écran.

### `76_settings_carte.png`
- **Action :** Config, panneau « Carte » (carte non gérée par un catalogue).
- **Écran attendu :** Le panneau « Carte » et le bouton « Modifier la carte ».
- **Capture :** plein écran.

### `77_menu_editor_product.png`
- **Action :** « Modifier la carte », toucher un produit, faire défiler jusqu'aux allergènes.
- **Écran attendu :** Trois colonnes (catégories, produits, fiche), la section « Allergènes » avec « Liste déclarée », « Contient » (Poissons, Soja) et « Traces possibles » (Sésame).
- **Capture :** plein écran.

### `78_menu_editor_category.png`
- **Action :** Éditeur de carte : toucher une catégorie.
- **Écran attendu :** La fiche de la catégorie, ses noms, sa couleur et « Temps du service en cuisine » (Entrées choisi).
- **Capture :** plein écran.

### `79_menu_editor_history.png`
- **Action :** Éditeur de carte : « Historique ».
- **Écran attendu :** La liste des modifications avec l'auteur, la date et les champs modifiés.
- **Capture :** plein écran.

### `80_reports_sales.png`
- **Action :** Rapports, onglet « Ventes ».
- **Écran attendu :** « Document non fiscal », les préréglages de période, la concordance avec les Z (identique, écart, période en cours), les ventes par produit et par heure, le total.
- **Capture :** plein écran.

### `81_settings_owner_alerts.png`
- **Action :** Config, panneau « Alertes sur téléphone » (robot configuré, alertes actives).
- **Écran attendu :** « Robot Telegram configuré », la conversation, la langue, les cinq événements surveillés et les délais ; aucun jeton affiché.
- **Capture :** plein écran.

### `82_order_allergens.png`
- **Action :** Commande : « Allergènes ».
- **Écran attendu :** Le mode actif, la légende des 14 allergènes numérotés et les pastilles sur les produits (six au plus, puis « +2 »).
- **Capture :** plein écran.

### `83_allergen_sheet.png`
- **Action :** Mode « Allergènes » : toucher un produit.
- **Écran attendu :** La fiche : « Contient », « Traces possibles », les options (« Information non disponible, demandez au responsable » pour une option non déclarée) et les boutons d'impression.
- **Capture :** plein écran.

### `84_settings_courses.png`
- **Action :** Config, panneau « Service en plusieurs temps ».
- **Écran attendu :** L'interrupteur, une ligne de quatre boutons (Direct, Entrées, Plats, Desserts) par catégorie et « Enregistrer ».
- **Capture :** plein écran.

### `85_cart_courses.png`
- **Action :** Commande avec des articles à plusieurs temps.
- **Écran attendu :** Le panier groupé par temps (Direct, Entrées, Plats), la pastille « EN ATTENTE » et le bouton « Envoyer la suite (Plats) ».
- **Capture :** plein écran.

### `86_course_picker.png`
- **Action :** Toucher la pastille d'un article en attente.
- **Écran attendu :** Le choix du temps à quatre boutons, le temps courant marqué.
- **Capture :** plein écran.

### `87_tables_held_course.png`
- **Action :** Plan de salle avec une table dont un temps est retenu.
- **Écran attendu :** La pastille « Suite : Plats » sur la table 2.
- **Capture :** plein écran.

### `88_checkout_held_course.png`
- **Action :** « Payer » sur la commande aux temps retenus.
- **Écran attendu :** L'avertissement « Suite non envoyée : Plats » et le bouton « Envoyer maintenant » au-dessus des modes de paiement.
- **Capture :** plein écran.

### `89_order_gift_voucher.png`
- **Action :** Commande : « Bon cadeau », saisir 25,00 €.
- **Écran attendu :** « Vendre un bon cadeau », le montant, et la ligne « Bon cadeau » avec la pastille « Bon » sans boutons de quantité.
- **Capture :** plein écran.

### `90_checkout_gift_voucher.png`
- **Action :** Paiement : « Bon cadeau », saisir le code à 12 chiffres, « Chercher ».
- **Écran attendu :** « Payer avec un bon cadeau », le code masqué (**** **** 9095), « Valide », le solde, la date de validité et le montant utilisé.
- **Capture :** plein écran.

### `91_reports_gift_vouchers.png`
- **Action :** Rapports, onglet « Bons cadeaux ».
- **Écran attendu :** « Vérifier un bon », la liste « Bons émis » avec ses filtres et les mouvements d'un bon déplié.
- **Capture :** plein écran.

### `92_settings_gift_vouchers.png`
- **Action :** Config, panneau « Bons cadeaux ».
- **Écran attendu :** L'interrupteur « Actif », la validité en mois et le montant maximum.
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
(sections 1, 1 bis et 2 via `npm run captures`, `00_boot_admission` via QEMU,
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

- Sections 1, 1 bis et 2 (95 écrans) : quelques secondes, `npm run captures`.
- `00_boot_admission` (QEMU, CI) : automatique, aucune action manuelle.
- Section 3 (7 tickets) : 20 à 30 minutes (impression et photo).
- Compilation (`scripts/make_captures_pdf.sh`) : quelques secondes.

**Total : environ 20 à 30 minutes, dominé par la section 3.**
