<!-- Dérivé de docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md @ 86088e4 -->
# Runbook de capture d'écrans pour le dossier de certification - pili-pos-android

**Référence :** §V (comportement aux réponses du FDM) et §VII.D (rapports X/Z visibles et imprimables) du manuel de certification. Preuve EV-09 : captures du profil sunmi_d3_pro_gms_api33, prises le 15.10.2026.

Liste des 188 captures A01 à A194 (A01 à A123 de P8b, puis A124 à A194 des voies suivantes), dans l'ordre des sections de MANUEL_UTILISATEUR_FR.md (numérotation Linux du manuel) ; les écrans propres à Android viennent après A79, puis les compléments A88 à A98, les écrans du terminal de paiement A99 à A111 et des compléments A112 à A123. Chaque id est cité dans MANUEL_UTILISATEUR_FR.md pour le même écran. Les fichiers vont dans le dossier screenshots du dossier Android, puis sont réunis dans CAPTURES_ECRAN_FR.pdf. Les libellés cités sont ceux de l'écran (français).

## Méthode
- **Terminal.** AVD `sunmi_d3_pro_gms_api33` (1920x1080 paysage, 160 dpi, Android 13), démarré par `PILI_AVD=sunmi_d3_pro_gms_api33 scripts/android/emulator.sh ensure` (profil à créer s'il manque ; un seul émulateur à la fois). Aucune capture de l'interface Linux n'est réutilisée.
- **Mise en service.** Paquet signé de développement fait par `scripts/android/make_dev_bundle.sh --seed --fdm-url http://10.0.2.2:8081/graphql --printer 10.0.2.2:9100` (jeu de test SPF ; sans ces options le paquet vise 127.0.0.1:19380 et 127.0.0.1:19390, injoignables depuis l'émulateur ; 10.0.2.2 est l'hôte, 8081 et 9100 sont les ports par défaut des mocks), importé comme dans `scripts/android/app_e2e.sh`. Employés du jeu : « Gerant Test » (gérant, PIN 1111), « Elisa » (serveur, PIN 2222), « Apprenti Test » (apprenti, PIN 3333).
- **Mocks de l'hôte.** `cargo run --package pos-fdm-mock` et `cargo run --package pos-printer-mock`. Les codes du FDM se règlent par `MOCK_FDM_ERROR` ou `MOCK_FDM_WARNING`, ou sans redémarrage par `POST /mock/control`.
- **Prise.** `adb exec-out screencap -p` plein écran, un PNG par id, nom `A<nn>_<ecran>.png` ; ne jamais recadrer. Vérifier à l'œil le texte attendu avant de garder le fichier. Un avertissement non obligatoire se referme seul après 10 s : capturer avant. Un avertissement obligatoire (BUFFER_NEAR_FULL, CLIENT_CERT_EXPIRED, UPDATE_POS_VATNO, UPDATE_POS_ESTNO) reste affiché sans compte à rebours : si le bandeau en montre un, la caisse a lu le code dans une réponse de vente avant la prochaine lecture de l'état de la boîte noire (cause trouvée en P8b, corrigée : le bandeau reste alors ouvert). Pour la capture, régler le code par `POST /mock/control` AVANT l'ouverture de session (ou toucher « Réessayer » sur l'état de la boîte noire), puis attendre la lecture de l'état (30 s au plus) sans passer de vente.
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
| A15 | `A15_options_produit.png` | Article à groupe d'options obligatoire (catalogue du mock) : fenêtre « Options », « Choisir 1 », « Ajouter » grisé | 5.3 |
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
| A27 | `A27_fin_vente_repli_papier.png` | Service d'e-mail en panne (MOCK_TEM_FAIL=1), vente en « E-mail » seul : le bouton « E-mail » disparaît et l'écran annonce « Service e-mail indisponible : le ticket papier sera imprimé, l'e-mail sera envoyé dès que possible » ; la panne survenue à l'envoi n'affiche que « Ticket imprimé » | 7.3 |
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
| A46 | `A46_employes.png` | Panneau « Employés » en Gérant, avec un Serveur en service, un Apprenti hors service et un employé « Présence seule » : note sur le personnel sans accès à la caisse, état « En service depuis HH:MM » ou « Hors service », « Arrivée » ou « Départ », « Code PIN » (absent pour « Présence seule »), « Désactiver », formulaire « Ajouter » à quatre profils (même vue d'écran que A45, qui contient déjà le panneau) ; lecteur de badges activé : sur chaque ligne « Badge lié le JJ/MM/AAAA » ou « Aucun badge » et le bouton « Lier un badge » ou « Retirer le badge » | 12.3 |
| A47 | `A47_config_invalide.png` | Configuration avec un numéro de TVA mal formé : bandeau rouge « Configuration invalide : le FDM refusera les tickets. Appelez l'installateur. » | 12.4 |
| A48 | `A48_tickets_email.png` | Panneau « Tickets par email » : « Configuré », « Joignable », envois avec adresse masquée | 12.5 |
| A49 | `A49_integrations.png` | Panneau « Intégrations » : « Activer » ou « Désactiver », « Historique » | 12.7 |
| A50 | `A50_hubrise_connexion.png` | « Se connecter à HubRise » : QR code et champ « Code d'autorisation » | 12.7 |
| A51 | `A51_hubrise_catalogue.png` | HubRise connecté : « Connecté », compte, catalogue, « Resynchroniser », canaux | 12.7 |
| A52 | `A52_commandes_en_ligne.png` | « En ligne » : colonnes « Nouvelles », « En cuisine », « Prêtes », filtre « Terminées » | 12.8 |
| A53 | `A53_export_nocturne_a_jour.png` | Réglages, « Export nocturne », export configuré : état « À jour », destination, dernier envoi | 12.8 |
| A54 | `A54_export_nocturne_en_attente.png` | Récepteur arrêté : « n exports en attente », « Dernière erreur », « Réessayer maintenant » | 12.8 |
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
| A65 | `A65_internal_error.png` | MOCK_FDM_ERROR=INTERNAL_ERROR, tentative d'encaissement : résultat inconnu, la caisse garde la requête. Texte attendu : « Erreur technique de la boîte noire. Contactez votre technicien. La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire. » | 14.1 |
| A66 | `A66_undefined_error.png` | MOCK_FDM_ERROR=UNDEFINED_ERROR : « Erreur indéfinie de la boîte noire. Contactez votre technicien. La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire. » | 14.1 |
| A67 | `A67_undefined_other.png` | MOCK_FDM_ERROR=UNDEFINED_OTHER : avec le simulateur, la vente aboutit (il ne refuse pas la signature pour ce code) ; le bandeau « Erreur de la boîte noire. Contactez votre technicien. » s'affiche avec son compte à rebours (erreur facultative) | 14.1 |
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
| A78 | `A78_fdm_response_mismatch.png` | Réponse du FDM qui ne correspond pas à la requête : résultat inconnu, la caisse garde la requête. Texte attendu : « Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien. La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire. » | 14.1 |
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
| A93 | `A93_fdm_certificate_refused.png` | Banc : MOCK_FDM_TLS_DIR=crates/pos-fdm-mock/fixtures/tls avec un certificat serveur non reconnu par la caisse, ou MOCK_FDM_TLS_REQUIRE_CLIENT=1 sans certificat client valide : « Connexion sécurisée à la boîte noire refusée (certificat). Contactez votre technicien. » | 14.1 |
| A94 | `A94_pointage_non_enregistre.png` | Arrêter le mock FDM, puis ouvrir un service (ou y mettre fin) : message rouge « Pointage non enregistré » et la raison | 3 |
| A95 | `A95_bandeau_rapport_z.png` | Journée ouverte, horloge de l'AVD après 22 h (ou journée précédente restée ouverte) : bandeau « Rapport Z non effectué aujourd'hui » sur l'écran de commande | 4 |
| A96 | `A96_export_spf_aucun_support.png` | Réglages, « Export pour le SPF », aucun support inséré : « Insérez la clé USB ou la carte SD. » | 12.11 |
| A97 | `A97_export_spf_interrompu.png` | Retirer le support pendant l'export : « Export interrompu » avec l'étape, le fichier et le détail | 12.11 |
| A98 | `A98_remboursement_support_ticket.png` | Rapports, « Tickets », « Rembourser », lignes choisies, étape du support : Papier / E-mail / Les deux, puis « Rembourser » avec le montant | 9 |
| A99 | `A99_demarrage_terminal_paiement.png` | Terminal de paiement lié, écran de connexion : ligne « Terminal de paiement : connecté » du panneau « Démarrage » | 2.1 |
| A100 | `A100_voyant_terminal.png` | Connecté, terminal lié : voyant « TERMINAL » vert dans l'en-tête | 4 |
| A101 | `A101_paiement_carte_terminal.png` | Écran de paiement, « Carte » : choix « Terminal » ou « Saisie manuelle » | 7.2 |
| A102 | `A102_paiement_sur_le_terminal.png` | « Valider » avec « Terminal » : fenêtre « Paiement sur le terminal », montant, « Annuler » ; voyant TERMINAL orange | 7.2 |
| A103 | `A103_carte_acceptee_vente_non_enregistree.png` | Carte acceptée puis FDM hors service : ligne « Carte · Visa Accepté », reste à payer 0,00, message rouge de la boîte noire | 7.2 |
| A104 | `A104_carte_acceptee_boite_noire_hs_modale.png` | Même cas : fenêtre « La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes. », « Réessayer maintenant » | 7.2 |
| A105 | `A105_pastille_carte_encaissee.png` | Retour au plan des tables : la table porte la pastille « Carte encaissée » | 5.1 |
| A106 | `A106_z_cartes_non_enregistrees.png` | Rapports, « Journée » : « Paiements par carte encaissés mais pas encore enregistrés dans une vente : » avec la table, le montant et la marque ; la clôture reste possible | 10.2 |
| A107 | `A107_carte_refusee.png` | Terminal qui refuse : « Paiement refusé », motif, « Rien n'a été débité. », « OK » | 7.2 |
| A108 | `A108_reponse_terminal_inconnue.png` | Terminal sans réponse exploitable : « Réponse du terminal inconnue », « Vérifier », « Fermer » | 7.2 |
| A109 | `A109_paiement_carte_a_verifier.png` | Après « Fermer » : ligne « Carte · Réponse inconnue » avec « Vérifier », « Valider » grisé, message rouge | 7.2 |
| A110 | `A110_verification_terminal.png` | Après « Vérifier » : ligne « Carte · Bancontact Accepté », reste à payer 0,00, « Valider » disponible | 7.2 |
| A111 | `A111_config_terminal_paiement.png` | Réglages, panneau « Informations » : ligne « Terminal de paiement » (type de terminal lié) | 12 |
| A112 | `A112_en_ligne_a_vérifier.png` | Onglet « En ligne », colonne « Nouvelles » : commandes Webshop et Deliveroo marquées « À vérifier » (« Produit absent du menu. »), boutons « Refuser » et « Resynchroniser » | 12.8 |
| A113 | `A113_en_ligne_refuser_confirmation.png` | Commande en ligne, « Refuser » : confirmation | 12.8 |
| A114 | `A114_en_ligne_terminees.png` | Onglet « En ligne », colonne « Terminées » (bouton « Terminées · 2 ») : commandes terminées | 12.8 |
| A115 | `A115_export_spf_bloque_commande_ouverte.png` | « Export pour le SPF » refusé : « Une commande ou une vente à emporter est ouverte » | 12.11 |
| A116 | `A116_export_spf_pret.png` | « Export pour le SPF » prêt : support détecté, « Exporter » | 12.11 |
| A117 | `A117_fin_vente_email_en_attente.png` | « Les deux » avec service d'e-mail en panne : « E-mail en attente, nouvel essai automatique » | 7.3 |
| A118 | `A118_mise_a_jour_rapport_z_du.png` | Écran de mise à jour avec rapport Z dû : blocage de la mise à jour | 12.12 |
| A119 | `A119_mise_en_service_config_invalide.png` | Mise en service avec paquet refusé (TVA du restaurant invalide) : détail de la configuration invalide | 14.2 |
| A120 | `A120_options_groupe_facultatif.png` | Article à groupe d'options facultatif : fenêtre « Options », « Ajouter » disponible | 5.3 |
| A121 | `A121_paiement_email_indisponible.png` | Paiement avec service d'e-mail en panne : mention « Service e-mail indisponible » | 7.3 |
| A122 | `A122_rapport_z_pas_encore_fait.png` | Écran de commande ou de rapports : « Rapport Z pas encore fait aujourd'hui » | 10.2 |
| A123 | `A123_reprise_menu_refusee.png` | HubRise connecté, une commande ouverte, Config, « Reprendre le menu » : message « Clôturez d'abord toutes les commandes ouvertes. » | 12.7 |
| A124 | `A124_pointage_par_le_gerant.png` | Config, panneau « Employés » : toucher « Arrivée » sur la ligne de l'employé « Présence seule » : message « Arrivée enregistrée · <nom> », la ligne passe à « En service depuis HH:MM » et son bouton devient « Départ » | 3, 12.3 |
| A125 | `A125_cheques_repas_terminal.png` | Écran de paiement avec un terminal relié : part prise « Part 1 · Chèques-repas · Edenred » avec la pastille « Accepté » (paiement accepté de marque Edenred) | 7.2 |
| A126 | `A126_lecteur_badges.png` | Config, panneau « Lecteur de badges » déplié, lecteur USB choisi (sur un appareil avec NFC : le choix « NFC intégré » est aussi proposé), « Connexion par badge » activée, les trois réglages du lecteur USB, le pavé numérique sur « Écart maximal entre caractères », bouton « Enregistrer », journal des badges, note de confidentialité | 12.10 |
| A127 | `A127_lier_badge.png` | Config, panneau « Employés », toucher « Lier un badge » sur la ligne d'un employé : fenêtre « Présentez le badge de <nom> » et bouton « Retour » | 12.3 |
| A128 | `A128_pointage_badge.png` | Écran de commande, un badge présenté pendant qu'un caissier est connecté : message « Arrivée · <nom> · HH:MM » (puis « Départ · <nom> · HH:MM » au passage suivant, après dix secondes) | 3 |
| A129 | `A129_connexion_badge.png` | Écran de connexion, lecteur de badges activé et connexion par badge autorisée : mention « Ou présentez votre badge » sous les tuiles ; en mode « Fin de service », mention « Présentez votre badge pour terminer le service » sous le pavé | 2.1, 2.3 |

| A112 | `A112_en_ligne_a_verifier.png` | Écran « En ligne » : deux commandes « Nouvelles » (Webshop et Deliveroo, payées) avec l'encadré « À vérifier · Produit absent du menu » et les boutons « Refuser » et « Resynchroniser » | 12.8 |
| A127b | `A127b_lier_badge_pin.png` | Config, lecteur de badges : fenêtre « Saisir le PIN » (« Confirmez avec votre PIN de gérant ») ouverte avant de lier un badge, sur le panneau « Lecteur de badges » réglé sur « Lecteur USB », « Connexion par badge » activée et réglages du lecteur | 12.13 |
| A130 | `A130_service_plusieurs_temps_inactif.png` | Config, panneau « Service en plusieurs temps » à « Inactif » : liste « Temps de chaque catégorie » (Direct, Entrées, Plats, Desserts) ; à côté, les panneaux « Plan de salle », « Lecteur de badges » et « Alertes sur téléphone » (inactives, « Robot Telegram absent ») | 12 ter.2 |
| A131 | `A131_service_plusieurs_temps_actif.png` | Même panneau à « Actif » : la catégorie « Starters » est réglée sur « Entrées » | 12 ter.2 |
| A132 | `A132_bons_cadeaux_reglage.png` | Config, panneau « Bons cadeaux » à « Actif » : « Validité (mois) » 12, « Montant maximum (€) » 500, bouton « Enregistrer » | 12 ter.3 |
| A133 | `A133_alertes_telephone_reglage.png` | Config, panneau « Alertes sur téléphone » : « Prévenir quand » (boîte noire, imprimante, terminal de paiement, Z du jour, commande en ligne), « Z attendu avant » 6 h, « Commande en ligne en attente depuis » 5 min, « Rappel toutes les » 60 min, « Enregistrer » et « Envoyer un message test » | 12 ter.1 |
| A136 | `A136_commande_changer_de_table.png` | Commande de la table 4 : bouton « Changer de table » en haut du panier | 5.1 bis |
| A137 | `A137_commande_par_temps.png` | Panier groupé par temps : « Direct », « Entrées », « Plats » (pastille « EN ATTENTE ») ; bouton « Envoyer la suite (Plats) » | 4 bis |
| A138 | `A138_selecteur_de_table.png` | Fenêtre « Changer de table · Table 4 » : « Choisissez la table libre », grille des tables libres (la 7 est choisie), « Retour » et « Valider » | 5.1 bis |
| A139 | `A139_tables_apres_transfert.png` | Plan des tables après le transfert : la table 7 porte 76,52 € (5 art., 2 min) et « Suite : Plats » ; bouton « Déplacer une table » | 5.1 bis |
| A140 | `A140_tables_deplacer_mode.png` | Plan des tables, mode « Déplacer une table » allumé : « Touchez la table à déplacer » | 5.1 bis |
| A141 | `A141_tables_deplacer_source.png` | Table 7 choisie (contour orange) : « Table 7 : touchez la table libre » | 5.1 bis |
| A142 | `A142_tables_deplacer_confirmation.png` | Fenêtre « Déplacer la table 7 vers la table 17 ? » avec « Retour » et « Valider » | 5.1 bis |
| A143 | `A143_remise_pourcentage.png` | Fenêtre « Remise sur la commande », onglet « % » (10 %, raccourcis 5 à 50 %), motif « Geste commercial », « Total 76,52 € → 68,87 € » | 5.3 |
| A144 | `A144_remise_montant.png` | Même fenêtre, onglet « Montant » : remise de 5,00 €, « Total 76,52 € → 71,52 € » | 5.3 |
| A145 | `A145_remise_offert.png` | Même fenêtre, onglet « Offert » : « Toute la commande est offerte : le ticket est à 0,00 € », « Total 76,52 € → 0,00 € » | 5.3 |
| A146 | `A146_remise_pin.png` | « Saisir le PIN » : « La remise figure sur le ticket fiscal. Confirmez avec votre PIN. » | 5.3 |
| A147 | `A147_commande_remise_commande.png` | Panier après une remise de 10 % : chaque ligne porte « Remise 10 % » et son montant négatif, le bouton « Remise » devient « Retirer la remise » | 5.3 |
| A148 | `A148_note_dialogue.png` | Fenêtre « Note pour la cuisine » : note « sans glace », compteur « 10/60 », « Imprimée en cuisine maintenant », clavier du terminal | 5.3 |
| A149 | `A149_commande_ligne_note.png` | Panier : la note « sans glace » s'affiche en italique sous le nom de la ligne et le bouton « Note » est en surbrillance | 5.3 |
| A150 | `A150_offrir_ligne_dialogue.png` | Fenêtre « Offrir la ligne » : quatre motifs, précision libre, « Total 20,00 € → 0,00 € » | 5.3 |
| A151 | `A151_commande_ligne_offerte.png` | Panier : la ligne offerte est à 0,00 € avec « Offert » et « -20,00 », bouton « Retirer la remise » | 5.3 |
| A152 | `A152_commande_remise_et_offert.png` | Panier avec une remise de 10 % sur les autres lignes et une ligne offerte : total 61,67 € | 5.3 |
| A153 | `A153_choix_temps_service.png` | Fenêtre « Temps du service » ouverte par la pastille d'une ligne : Direct, Entrées, Plats, Desserts, « Annuler » | 4 bis |
| A154 | `A154_envoyer_la_suite.png` | Message « Suite envoyée : Plats » après « Envoyer la suite » : le temps Plats n'est plus « EN ATTENTE », les Desserts le sont encore | 4 bis |
| A155 | `A155_addition_avec_remise.png` | Message « Addition provisoire imprimée » sur une commande remisée (lignes « Remise 10 % » et « Offert ») | 6.2 |
| A156 | `A156_paiement_avec_remise.png` | Écran de paiement d'une commande remisée : lignes avec leur remise, « Sous-total 88,52 € », « Remise -26,85 € », « TOTAL 61,67 € », quatre moyens dont « Bon cadeau » | 7.2 |
| A159 | `A159_correction_dialogue.png` | Rapports, onglet « Tickets » : fenêtre « Corriger le paiement » d'une vente, « Quelle ligne de paiement faut-il corriger ? », la ligne en espèces, « Payé en réalité par » (Espèces, Carte, Chèques-repas) | 9 bis |
| A160 | `A160_correction_choix.png` | Même fenêtre, « Carte » choisie : résumé « Espèces … → Carte … » et bouton « Corriger » actif | 9 bis |
| A162 | `A162_tickets_vente_cochee_facture.png` | Rapports, onglet « Tickets » : la vente #1 est cochée, bouton « Facture (1) » ; onglets « Journée », « Caisse », « Tickets », « Ventes » et « Bons cadeaux » | 8 bis |
| A163 | `A163_facture_client.png` | Fenêtre « Facture entreprise », étape « 1 · Client » : « Client connu », « N° de TVA » (BE0123456789), « Vérifier », nom, adresse, code postal, ville, pays | 8 bis |
| A165 | `A165_facture_apercu.png` | Étape « 2 · Aperçu » : client ACME SRL, pastille « Peppol inconnu (hors ligne) », lignes, TVA par taux, « Total TTC 61,67 € », « Déjà payé 61,67 €. Reste à payer 0,00 € », « Établir et imprimer la facture » | 8 bis |
| A166 | `A166_facture_refus_peppol.png` | « La facture n'a pas été établie. » : « Ce client a un numéro de TVA belge : la facture doit passer par Peppol, qui n'est pas configuré sur cette installation. Contactez le support. » | 8 bis |
| A167 | `A167_facture_etablie.png` | « Facture 20260001 établie », « Facture imprimée. », « Pas d'envoi Peppol pour cette facture. », « Terminer » | 8 bis |
| A168 | `A168_factures_liste.png` | Onglet « Tickets » : la vente #1 porte « Facture 20260001 » ; tableau « Factures » (20260001, ACME SARL, 18/10/2026, 61,67 €, « Papier seulement », « Copie ») | 8 bis |
| A169 | `A169_rapport_ventes.png` | Onglet « Ventes » : « Rapport de ventes » (« Document non fiscal… »), périodes, « Concordance avec les Z », 6 tickets, total 184,16 €, ventes par produit et par heure, « Imprimer » | 10.4 |
| A171 | `A171_commande_ligne_bon_cadeau.png` | Commande de la table 2 : la ligne « Bon cadeau » (pastille « Bon », 1 × 50,00 €) à la place des boutons de quantité | 4 bis |
| A172 | `A172_fin_vente_bon_cadeau.png` | Fin de vente d'un bon : « OK », ticket de caisse T.V.A. de 50,00 €, « Bon imprimé », « Ticket imprimé », « Terminer » | 7.3 |
| A173 | `A173_paiement_bon_cadeau_code.png` | « Payer avec un bon cadeau » : « Code du bon (12 chiffres) » saisi (6471 0477 4259), « Retour » et « Chercher » | 7.2 bis |
| A174 | `A174_paiement_bon_cadeau_valide.png` | Bon trouvé : code masqué (**** **** 4259), « Valide », « Solde 50,00 € », « Valable jusqu'au 18.10.2027 », « Montant utilisé 50,00 € », « Utiliser ce bon » | 7.2 bis |
| A175 | `A175_paiement_bon_cadeau_et_especes.png` | Paiement de 60,00 € en deux parts : « Part 1 · Bon cadeau · 50,00 € » validée, « Part 2 · Espèces · 10,00 € », « Reste à payer 10,00 € » | 7.2 bis |
| A176 | `A176_fin_vente_payee_bon_cadeau.png` | Fin de vente de 60,00 € payée avec le bon et des espèces : « OK », « Ticket imprimé », « Terminer » | 7.3 |
| A177 | `A177_bons_cadeaux_liste.png` | Rapports, onglet « Bons cadeaux » : « Vérifier un bon », « Bons émis » avec les filtres (Tous, Valide, Épuisé, Expiré, Annulé) ; le bon **** **** 4259 est « Épuisé », 0,00 € / 50,00 € | 10.5 |
| A178 | `A178_bons_cadeaux_verification.png` | Même onglet après « Chercher » : bon 6471 0477 4259, « Épuisé », « Solde 0,00 € », « Valable jusqu'au 18.10.2027 » | 10.5 |
| A179 | `A179_epuise_mode.png` | Écran de commande, mode « Épuisé » allumé : « Touchez un produit pour changer sa disponibilité. » | 5.4 |
| A180 | `A180_epuise_dialogue.png` | Fenêtre du produit « Dry Martini » : « Épuisé jusqu'à la fin de la journée », « Épuisé jusqu'à réactivation », « Disponible », « Retour » | 5.4 |
| A181 | `A181_produit_epuise_refus.png` | Produit grisé avec la pastille « Épuisé », message « Ce produit est épuisé. », bouton « Produits épuisés (1) » | 5.4 |
| A182 | `A182_produits_epuises_liste.png` | Fenêtre « Produits épuisés » : « Dry Martini, Jusqu'à la fin de la journée (19/10 05:00) », « Remettre en vente », « Retour » | 5.4 |
| A183 | `A183_editeur_carte_liste.png` | Config, « Édition de la carte » : catégories (« Menus » et « Vouchers » masquées), produits de la catégorie, « + Nouvelle catégorie », « + Nouveau produit », « Historique » | 12 bis |
| A184 | `A184_editeur_carte_produit.png` | Fiche du produit « Dry Martini » : noms par langue, catégorie, « Prix (€) », « TVA sur place », « TVA à emporter », « Code du menu », « Annuler » et « Enregistrer » | 12 bis |
| A185 | `A185_allergenes_grille.png` | Même fiche, section « Allergènes » : « Liste déclarée », les 14 allergènes (Lait et Sulfites « Contient », Céréales « Traces possibles »), « Groupes d'options » | 12 ter.4 |
| A186 | `A186_editeur_produit_enregistre.png` | Même fiche après « Enregistrer » : message « Carte enregistrée » | 12 bis |
| A187 | `A187_editeur_categorie_temps.png` | Fiche de la catégorie « Starters » : noms par langue, couleur, « Temps du service en cuisine » (Entrées), position, « Visible à la caisse » | 12 ter.2 |
| A188 | `A188_commande_allergenes.png` | Écran de commande, mode « Allergènes » allumé : légende des 14 numéros, pastilles 7 et 12 sur « Dry Martini », « Touchez un produit pour voir ses allergènes » | 4 bis |
| A189 | `A189_fiche_allergenes_produit.png` | Fiche d'allergènes du produit : « Règlement (UE) n° 1169/2011, annexe II », « Contient 7 Lait, 12 Sulfites », « Traces possibles 1 Céréales contenant du gluten », « Imprimer cette fiche », « Imprimer la carte complète », « Fermer » | 4 bis |
| A190 | `A190_ticket_paiement_corrige.png` | Rapports, onglet « Tickets » : la vente #1 porte la mention « Paiement corrigé : Espèces → Carte » à la place du bouton « Corriger le paiement » | 9 bis |
| A191 | `A191_correction_refusee_terminal.png` | Fenêtre « Corriger le paiement » d'une vente payée sur le terminal lié : « Une carte prise sur le terminal lié ne peut pas être corrigée. », « Retour » | 9 bis |
| A192 | `A192_facture_client_peppol_inconnu.png` | Facture entreprise, étape « 1 · Client » : numéro BE0123456749, pastille « Peppol inconnu (hors ligne) », « Peppol n'a pas répondu : la facture peut être établie, l'envoi sera retenu. » | 8 bis |
| A193 | `A193_bon_cadeau_vente_montant.png` | Fenêtre « Vendre un bon cadeau » : « Montant du bon » 50,00 €, pavé numérique, « Retour » et « Valider » | 4 bis |
| A194 | `A194_rapport_ventes_periode_libre.png` | Onglet « Ventes », « Période libre » : saisie « Du __/__/____ » et « Au __/__/____ » au pavé, bouton « Afficher » | 10.4 |
| A50b | `A50b_hubrise_code_refuse.png` | Connexion à HubRise, code refusé : QR code et lien d'autorisation, code « abc123 » saisi, message rouge « HubRise a refusé ce code ou ne répond pas : vérifiez le code et la connexion, puis réessayez » avec le détail de la réponse (400) | 12.7 |

Lignes A80 à A87 : écrans propres à Android, sans équivalent Linux (mise en service, à propos, export pour le SPF, mise à jour), décrits dans MANUEL_INSTALLATION_FR.md et MANUEL_UTILISATEUR_FR.md. Lignes A88 à A129 : compléments pour les écrans que le manuel décrit et qu'aucune ligne plus haut ne couvrait.

**Compilation finale.** Une fois toutes les captures dans le dossier screenshots, `scripts/make_captures_pdf.sh` (adapté au dossier Android en P8b) les assemble dans l'ordre du nom de fichier en CAPTURES_ECRAN_FR.pdf. Vérifier ensuite que ce PDF figure dans les annexes de REPONSES_MANUEL_CERT_FR.md §V et §VII.D.

Captures reprises le 2026-10-18 pour la version rc3 : toutes les lignes ci-dessus dont l'écran a changé (connexion, tables, commande, paiement, rapports, configuration) ont été refaites sur le jeu de test SPF ; les lignes A130 et suivantes sont nouvelles.
