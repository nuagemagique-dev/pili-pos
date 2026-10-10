<!-- Dérivé de docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md @ 86088e4 -->
# Runbook de capture d'écrans pour le dossier de certification - pili-pos-android

**Référence :** §V (comportement aux réponses du FDM) et §VII.D (rapports X/Z visibles et imprimables) du manuel de certification. Preuve `À effectuer (EV-09 : captures du profil sunmi_d3_pro_gms_api33)`.

Liste des 92 captures de P8b, dans l'ordre des sections de MANUEL_UTILISATEUR_FR.md (numérotation Linux du manuel) ; les écrans propres à Android viennent après A79, puis les compléments A88 à A92. Chaque id est cité dans MANUEL_UTILISATEUR_FR.md pour le même écran. Les fichiers vont dans le dossier screenshots du dossier Android, puis sont réunis dans CAPTURES_ECRAN_FR.pdf. Les libellés cités sont ceux de l'écran (français).

## Méthode
- **Terminal.** AVD `sunmi_d3_pro_gms_api33` (1920x1080 paysage, 160 dpi, Android 13), démarré par `PILI_AVD=sunmi_d3_pro_gms_api33 scripts/android/emulator.sh ensure` (profil à créer s'il manque ; un seul émulateur à la fois). Aucune capture de l'interface Linux n'est réutilisée.
- **Mise en service.** Paquet signé de développement fait par `scripts/android/make_dev_bundle.sh --seed --fdm-url http://10.0.2.2:8081/graphql --printer 10.0.2.2:9100` (jeu de test SPF ; sans ces options le paquet vise 127.0.0.1:19380 et 127.0.0.1:19390, injoignables depuis l'émulateur ; 10.0.2.2 est l'hôte, 8081 et 9100 sont les ports par défaut des mocks), importé comme dans `scripts/android/app_e2e.sh`. Employés du jeu : « Gerant Test » (gérant, PIN 1111), « Elisa » (serveur, PIN 2222), « Apprenti Test » (apprenti, PIN 3333).
- **Mocks de l'hôte.** `cargo run --package pos-fdm-mock` et `cargo run --package pos-printer-mock`. Les codes du FDM se règlent par `MOCK_FDM_ERROR` ou `MOCK_FDM_WARNING`, ou sans redémarrage par `POST /mock/control`.
- **Prise.** `adb exec-out screencap -p` plein écran, un PNG par id, nom `A<nn>_<ecran>.png` ; ne jamais recadrer. Vérifier à l'œil le texte attendu avant de garder le fichier. Un avertissement non obligatoire se referme seul après 10 s : capturer avant.
- **Tickets imprimés.** Les tickets se lisent dans TICKETS_EXEMPLES_FR.md (EV-08), non dans cette liste. Assistance à distance : sans objet sur Android, aucune capture.

## Captures
| Id | Fichier | État à préparer et écran attendu | § manuel |
|---|---|---|---|
| A01 | `A01_login_tuiles.png` | Ouvrir l'application, aucun employé connecté : panneau « Démarrage » (FDM, imprimantes, e-mail), version, tuiles des employés « en service » ou « hors service », « Autre personne (INSZ) », boutons de langue | 2.1 |
| A02 | `A02_login_pin.png` | Toucher la tuile « Gerant Test » : pavé « Code PIN », 4 points vides (avant le 4e chiffre) | 2.1 |
| A03 | `A03_login_insz.png` | Toucher « Autre personne (INSZ) » : saisie des 11 chiffres, « Connexion » | 2.1 |
| A04 | `A04_login_premier_pin.png` | Base sans PIN de gérant, toucher sa tuile : « Choisissez votre PIN » | 2.1 |
| A05 | `A05_pin_propre.png` | Connecté, toucher son nom dans l'en-tête (mention « PIN ») : « Saisir le PIN » | 2.2 |
| A06 | `A06_pin_verrou.png` | Cinq PIN faux de suite : « Trop de PIN incorrects : réessayez dans 30 s. » | 2.2 |
| A07 | `A07_premier_pin_gerant_requis.png` | Second gérant sans PIN alors qu'un autre en a un, PIN saisi deux fois : « Un gérant doit choisir le PIN de ce compte dans Paramètres. » | 2.2 |
| A08 | `A08_login_en_service.png` | « Verrouiller » : retour à la connexion, tuile « en service » | 2.3 |
| A09 | `A09_fin_service.png` | Tuile « en service », « Fin de service », PIN : « Service terminé · Gerant Test » | 3 |
| A10 | `A10_commande_vide.png` | Toucher la table 4 : en-tête (voyants FDM, Ticket, Cuisine, nom, « Table 4 », heure, boutons), catégories et articles, panier « Aucun article » | 4 |
| A11 | `A11_tables_plan.png` | « Tables » : tables « Libre », occupées (articles, minutes, total), une avec « Addition » | 5.1 |
| A12 | `A12_tables_emporter_en_cours.png` | Vente à emporter commencée puis quittée : liste « À emporter en cours », « Nouvelle vente à emporter » | 5.2 |
| A13 | `A13_commande_remplie.png` | Ajouter des articles dont un en double : panier, « Pas encore en cuisine », détail de TVA, total, « Envoyer en cuisine », « Addition », « PAYER » | 5.3 |
| A14 | `A14_prix_libre.png` | Toucher un article à prix libre : fenêtre « Prix libre », pavé lu en cents | 5.3 |
| A15 | `A15_options_produit.png` | HubRise activé, article à options : fenêtre « Options », règles de groupe, « Ajouter » grisé | 5.3 |
| A16 | `A16_envoye_cuisine.png` | « Envoyer en cuisine » : « Envoyé en cuisine · n articles » | 6.1 |
| A17 | `A17_addition_imprimee.png` | Sur une autre table, « Addition » : « Addition provisoire imprimée » | 6.2 |
| A18 | `A18_paiement_moyens.png` | « PAYER » : total à payer, « Espèces », « Carte », « Chèques-repas » (espèces choisi d'office) | 7 |
| A19 | `A19_ticket_support.png` | Ticket numérique configuré : ligne « Ticket » avec Papier, E-mail, Les deux | 7.1 |
| A20 | `A20_ticket_email_clavier.png` | « E-mail » : clavier « Adresse e-mail du client », raccourcis, adresse en cours de saisie | 7.1 |
| A21 | `A21_ticket_email_confirmer.png` | Saisie validée : « Confirmer l'adresse », « Modifier », « Annuler » | 7.1 |
| A22 | `A22_paiement_especes.png` | « Espèces », saisir 50 : « Montant reçu », « Rendu », « Exact », arrondi aux 5 cents | 7.2 |
| A23 | `A23_paiement_parts.png` | « Partager en » 2 parts égales : pastilles « Part 1 » et « Part 2 », « Valider part 1 » | 7.2 |
| A24 | `A24_paiement_reste.png` | Un paiement pris (chèques-repas) : montant pris, « Reste à payer », « Annuler les paiements » | 7.2 |
| A25 | `A25_fin_vente.png` | « Valider » : « OK », type de ticket, total, rendu, identifiant FDM, « Ticket imprimé », « Terminer » | 7.3 |
| A26 | `A26_fin_vente_email.png` | Vente payée avec ticket par e-mail : « Ticket envoyé à » et l'adresse masquée | 7.3 |
| A27 | `A27_fin_vente_repli_papier.png` | Service d'e-mail en panne (MOCK_TEM_FAIL=1), vente en « E-mail » seul : le papier sort à la place, l'écran le dit | 7.3 |
| A28 | `A28_fin_vente_imprimer.png` | Imprimante hors ligne à l'encaissement, en espèces : bouton « Imprimer le ticket » et « Terminer » | 7.3 |
| A29 | `A29_fdm_voile.png` | FDM simulé arrêté : voile rouge sur Tables, Commande et Paiement, message, « Réessayer maintenant » | 7.4 |
| A30 | `A30_sans_paiement_pin.png` | Gérant, panier non vide, « Client parti sans payer » : « Confirmez avec votre PIN » | 7.5 |
| A31 | `A31_sans_paiement_ok.png` | PIN accepté : « Vente clôturée sans paiement », table libre | 7.5 |
| A32 | `A32_rapports_tickets.png` | « Rapports », onglet « Tickets » : « Tickets récents » avec « Copie » et « Rembourser » | 8 |
| A33 | `A33_copie_support.png` | « Copie » sur une vente : choix Papier, E-mail, Les deux, « Valider » | 8 |
| A34 | `A34_remboursement_lignes.png` | « Rembourser » : « Articles à rembourser », ligne sélectionnée avec quantité, bouton « Rembourser » avec le montant | 9 |
| A35 | `A35_remboursement_deux_prix.png` | Vente avec un produit à deux prix : deux lignes distinctes, la ligne choisie seule sélectionnée | 9 |
| A36 | `A36_rapports_journee.png` | « Rapports », onglet « Journée » : « Rapport X », « Rapport utilisateurs X », « Clôturer la journée (Z) », « Réimprimer un Z » | 10 |
| A37 | `A37_rapport_x.png` | « Rapport X » : « Rapport généré (X) » | 10.1 |
| A38 | `A38_rapport_z_confirmer.png` | « Clôturer la journée (Z) » une fois : « Confirmer la clôture ? » (orange) | 10.2 |
| A39 | `A39_rapport_z_genere.png` | Second appui dans les 5 secondes : « Rapport généré (Z #n) » | 10.2 |
| A40 | `A40_historique_z.png` | « Réimprimer un Z » : « Historique Z » (numéro et date) | 10.3 |
| A41 | `A41_caisse_mouvement.png` | Onglet « Caisse » : « 1 · Sens », « 2 · Motif », « 3 · Montant », « Récapitulatif », « Enregistrer l'entrée » | 11.1 |
| A42 | `A42_caisse_enregistre.png` | Entrée de 20 € « Fond de caisse » enregistrée : « Mouvement enregistré » | 11.1 |
| A43 | `A43_tiroir_ouvert.png` | « Ouvrir le tiroir » : « Tiroir ouvert » | 11.2 |
| A44 | `A44_tiroir_declaration.png` | « Déclarer le tiroir » : fenêtre « Déclarer le contenu du tiroir », pavé | 11.3 |
| A45 | `A45_reglages.png` | « Config » (gérant) : page Paramètres, panneaux mode formation, langue, informations (POS ID, Terminal, FDM, Imprimante), section « Autres réglages » | 12 |
| A46 | `A46_employes.png` | Panneau « Employés » : nom, profil, « PIN défini », « Code PIN », « Désactiver », formulaire « Ajouter » | 12.3 |
| A47 | `A47_config_invalide.png` | Configuration avec un numéro de TVA mal formé : bandeau rouge « Configuration invalide : le FDM refusera les tickets. Appelez l'installateur. » | 12.4 |
| A48 | `A48_tickets_email.png` | Panneau « Tickets par email » : « Configuré », « Joignable », envois avec adresse masquée | 12.5 |
| A49 | `A49_integrations.png` | Panneau « Intégrations » : « Activer » ou « Désactiver », « Historique » | 12.7 |
| A50 | `A50_hubrise_connexion.png` | « Se connecter à HubRise » : QR code et champ « Code d'autorisation » | 12.7 |
| A51 | `A51_hubrise_catalogue.png` | HubRise connecté : « Connecté », compte, catalogue, « Resynchroniser », canaux | 12.7 |
| A52 | `A52_commandes_en_ligne.png` | « En ligne » : colonnes « Nouvelles », « En cuisine », « Prêtes », filtre « Terminées » | 12.8 |
| A53 | `A53_export_nocturne_a_jour.png` | Réglages, « Export nocturne », export configuré : état « À jour », destination, dernier envoi | 12.9 |
| A54 | `A54_export_nocturne_attente.png` | Récepteur arrêté : « n exports en attente », « Dernière erreur », « Réessayer maintenant » | 12.9 |
| A55 | `A55_formation_actif.png` | « Mode Formation » actif (commandes vidées avant) : bandeau violet, filigrane « TRAINING » | 13 |
| A56 | `A56_buffer_full.png` | MOCK_FDM_ERROR=BUFFER_FULL : bloquant | 14.1 |
| A57 | `A57_fdm_locked.png` | MOCK_FDM_ERROR=FDM_LOCKED : bloquant | 14.1 |
| A58 | `A58_fdm_not_operational.png` | MOCK_FDM_ERROR=FDM_NOT_OPERATIONAL : bloquant | 14.1 |
| A59 | `A59_unknown_pos.png` | MOCK_FDM_ERROR=UNKNOWN_POS : bloquant | 14.1 |
| A60 | `A60_buffer_near_full.png` | MOCK_FDM_WARNING=BUFFER_NEAR_FULL : avertissement obligatoire | 14.1 |
| A61 | `A61_client_cert_expired.png` | MOCK_FDM_WARNING=CLIENT_CERT_EXPIRED : avertissement obligatoire | 14.1 |
| A62 | `A62_client_cert_near_expiration.png` | MOCK_FDM_WARNING=CLIENT_CERT_NEAR_EXPIRATION : avertissement | 14.1 |
| A63 | `A63_unauthorized.png` | MOCK_FDM_ERROR=UNAUTHORIZED : erreur du FDM, vente refusée | 14.1 |
| A64 | `A64_invalid_request.png` | MOCK_FDM_ERROR=INVALID_REQUEST : erreur du FDM, vente refusée | 14.1 |
| A65 | `A65_internal_error.png` | MOCK_FDM_ERROR=INTERNAL_ERROR : erreur du FDM, vente refusée | 14.1 |
| A66 | `A66_undefined_error.png` | MOCK_FDM_ERROR=UNDEFINED_ERROR : erreur du FDM, vente refusée | 14.1 |
| A67 | `A67_undefined_other.png` | MOCK_FDM_ERROR=UNDEFINED_OTHER : erreur du FDM, vente refusée | 14.1 |
| A68 | `A68_rtc_sync_failed.png` | MOCK_FDM_WARNING=RTC_SYNC_FAILED : avertissement | 14.1 |
| A69 | `A69_update_urls_failed.png` | MOCK_FDM_WARNING=UPDATE_URLS_FAILED : avertissement | 14.1 |
| A70 | `A70_update_trust_cert_failed.png` | MOCK_FDM_WARNING=UPDATE_TRUST_CERT_FAILED : avertissement | 14.1 |
| A71 | `A71_update_task_list_failed.png` | MOCK_FDM_WARNING=UPDATE_TASK_LIST_FAILED : avertissement | 14.1 |
| A72 | `A72_task_feedback_failed.png` | MOCK_FDM_WARNING=TASK_FEEDBACK_FAILED : avertissement | 14.1 |
| A73 | `A73_nop_failed.png` | MOCK_FDM_WARNING=NOP_FAILED : avertissement | 14.1 |
| A74 | `A74_server_cert_resolve_failed.png` | MOCK_FDM_WARNING=SERVER_CERT_RESOLVE_FAILED : avertissement | 14.1 |
| A75 | `A75_transaction_upload_failed.png` | MOCK_FDM_WARNING=TRANSACTION_UPLOAD_FAILED : avertissement | 14.1 |
| A76 | `A76_initialization_failed.png` | MOCK_FDM_WARNING=INITIALIZATION_FAILED : avertissement | 14.1 |
| A77 | `A77_fdm_unreachable.png` | FDM simulé arrêté puis tentative d'encaissement : « Connexion perdue avec la boîte noire. Vérifiez le câble réseau. », vente refusée | 14.1 |
| A78 | `A78_fdm_response_mismatch.png` | Réponse du FDM qui ne correspond pas à la requête : « Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien. » | 14.1 |
| A79 | `A79_fdm_answer_pending.png` | Réponse du FDM à une vente perdue (FDM_ANSWER_PENDING) : « La caisse attend la réponse de la boîte noire pour l'opération précédente. » | 14.1 |
| A80 | `A80_mise_en_service.png` | Caisse sans configuration : « Mise en service de la caisse », « Choisir le dossier » | 14.2 |
| A81 | `A81_mise_en_service_refusee.png` | Paquet de configuration à signature invalide : « La signature du paquet de configuration est absente ou invalide. » | 14.2 |
| A82 | `A82_a_propos.png` | Réglages, « À propos » : version, SHA-1 et SHA-256 de base.apk, SHA-1 de libpos_ffi.so, SHA-256 du certificat de signature, deviceId, posId, terminalId, stockage | 12.10 |
| A83 | `A83_export_spf_dossier.png` | Réglages, « Export pour le SPF », support inséré : « Choisir le dossier », puis « Exporter » | 12.11 |
| A84 | `A84_export_spf_termine.png` | Export fini : « Export terminé », fichiers, empreinte SHA-256 de MANIFEST.sha256 | 12.11 |
| A85 | `A85_mise_a_jour_accueil.png` | Réglages, « Mise à jour » : version installée, « Vérifier en ligne », « Depuis une clé USB » | 12.12 |
| A86 | `A86_mise_a_jour_verifiee.png` | Mise à jour déclarée vérifiée : « Mise à jour disponible et vérifiée », « Installer la mise à jour » | 12.12 |
| A87 | `A87_mise_a_jour_refusee.png` | Version non plus récente ou empreinte fausse : « Mise à jour refusée » et le détail | 12.12 |
| A88 | `A88_update_pos_vatno.png` | MOCK_FDM_WARNING=UPDATE_POS_VATNO : avertissement obligatoire | 14.1 |
| A89 | `A89_update_pos_estno.png` | MOCK_FDM_WARNING=UPDATE_POS_ESTNO : avertissement obligatoire | 14.1 |
| A90 | `A90_fdm_not_initialized.png` | MOCK_FDM_INITIALIZED=false : voyant orange, « La boîte noire n'est pas encore initialisée. » | 14.1 |
| A91 | `A91_en_ligne_retarder.png` | « En ligne », commande acceptée, « Retarder » : fenêtre « +15 min », « +30 min », « +45 min » | 12.8 |
| A92 | `A92_mise_a_jour_bloquee.png` | Réglages, « Mise à jour », une commande ouverte : « La mise à jour ne peut pas démarrer maintenant : » et la liste | 12.12 |

Lignes A80 à A87 : écrans propres à Android, sans équivalent Linux (mise en service, à propos, export pour le SPF, mise à jour), décrits dans MANUEL_INSTALLATION_FR.md et MANUEL_UTILISATEUR_FR.md. Lignes A88 à A92 : compléments pour les écrans que le manuel décrit et qu'aucune ligne plus haut ne couvrait.

**Compilation finale.** Une fois toutes les captures dans le dossier screenshots, `scripts/make_captures_pdf.sh` (adapté au dossier Android en P8b) les assemble dans l'ordre du nom de fichier en CAPTURES_ECRAN_FR.pdf. Vérifier ensuite que ce PDF figure dans les annexes de REPONSES_MANUEL_CERT_FR.md §V et §VII.D.
