<!-- Variante Android, sans équivalent Linux -->
# Index du dossier de demande - pili-pos-android

**Distributeur :** Bruno Moyse / nuagemagique.dev (Bruxelles, BE)
**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)

---

## 1. Contenu du dossier

Ce document est la liste des pièces remises à l'appui de la demande (article 101 de l'arrêté ministériel du 29 avril 2024 : « une liste avec un aperçu des documents et informations remis »). Le cadrage de la demande est dans `CADRE_DEMANDE_FR.md` (annexe 35). Le fichier exact du ZIP, pièce par pièce, est `MANIFEST.txt`.

Dans le texte des annexes, un renvoi à un fichier `.md` du dossier désigne l'annexe PDF du même nom.

Natures : **document** (description, manuel ou réponse du producteur) et **preuve** (pièce remplie avec les résultats des tests et captures).

| # | Fichier | Section du manuel couverte | Nature |
|---|---|---|---|
| 0 | INDEX_DOSSIER_FR.pdf (ce document) | liste des documents (article 101) | document |
| 1 | REPONSES_MANUEL_CERT_FR.pdf | parcours linéaire §I-VIII | document |
| 2 | LISTE_DISTRIBUTEURS_FR.pdf | §I.E | document |
| 3 | PROFIL_ENTREPRISE_FR.pdf | §I.F | document |
| 4 | MANUEL_UTILISATEUR_FR.pdf | §I.C | document |
| 5 | MANUEL_INSTALLATION_FR.pdf | §I.C | document |
| 6 | MANUEL_PROGRAMMATION_FR.pdf | §I.C | document |
| 7 | DECLARATION_FONCTIONS_INTERDITES_FR.pdf | §III.C | document |
| 8 | PROFILS_UTILISATEURS_FR.pdf | §III.C | document |
| 9 | DOCUMENT_PROCESSUS_TECHNIQUE_FR.pdf | §II + §III.E | document |
| 10 | FLUX_DONNEES_FR.pdf | §II.C + §III | document |
| 11 | SCHEMA_BD_FR.pdf | §II.C | document |
| 12 | SECURITE_BD_FR.pdf | §II.C, accès du SPF aux données | document |
| 13 | SCHEMA_ER_FR.pdf | §II.C | document |
| 14 | SCHEMA_ASSEMBLAGE_FR.pdf | §I.C + §II.G | document |
| 15 | LISTE_FONCTIONNALITES_FR.pdf | §III | document |
| 16 | LIGNES_PRODUITS_FR.pdf | §III, lignes de produit | document |
| 17 | STRATEGIE_COMPTEURS_FR.pdf | §III.B (numérotation) + FDM non opérationnel | document |
| 18 | FLUX_REMBOURSEMENT_FR.pdf | §III.F | document |
| 19 | TICKETS_EXEMPLES_FR.pdf | §VI | preuve |
| 20 | DECISION_PEPPOL_AP_FR.pdf | §III.F (encadré B2B) | document |
| 21 | MAPPING_CONFORMITE_FR.pdf | §IV + §V | document |
| 22 | FEUILLE_ACQUISITION_DONNEES_FR.pdf | §VII (acquisition des données) | document |
| 23 | RAPPORT_TESTS_BASELINE_FR.pdf | §IV + §VIII | document |
| 24 | COUVERTURE_FIXTURES_FR.pdf | §IV | document |
| 25 | COUVERTURE_RAPPORTS_FR.pdf | §IV | document |
| 26 | COUVERTURE_ADVANCED_SET_FR.pdf | §IV | document |
| 27 | RESULTATS_TESTS_UC_FR.pdf | §IV, résultats des cas d'utilisation | preuve |
| 28 | CAPTURES_ECRAN_FR.pdf | §V + §VII.D | preuve |
| 29 | JOURNAL_TESTS_PRE_SOUMISSION_FR.pdf | §IV, répétition pré-soumission | preuve |
| 30 | CHECKLIST_DEPLOIEMENT_FR.pdf | machine de test | document |
| 31 | GARANTIE_MAJ_LANGUES_FR.pdf | §I.D, §II.D, §II.E, §VI | document |
| 32 | TICKET_NUMERIQUE_FR.pdf | §III.H | document |
| 33 | COMMANDES_EN_LIGNE_FR.pdf | §III.M, commandes en ligne | document |
| 34 | EXPORT_NOCTURNE_FR.pdf | Configuration Internet + §VII.B + acquisition des données (export nocturne facultatif) | document |
| 35 | CADRE_DEMANDE_FR.pdf | cadrage de la demande (article 99) | document |
| 36 | PLATEFORME_ANDROID_FR.pdf | plateforme, verrouillage, identifiant de l'appareil, stockage | document |
| 37 | FACTURATION_PEPPOL_FR.pdf | §III.F (facturation B2B, événement I, envoi Peppol) | document |
| 38 | EDITEUR_CARTE_FR.pdf | §III (édition de la carte à l'écran) | document |
| 39 | PRODUIT_EPUISE_FR.pdf | §III (produit épuisé) | document |
| 40 | RAPPORTS_VENTES_FR.pdf | §III.L (rapport de ventes non fiscal) | document |
| 41 | ALERTES_PROPRIETAIRE_FR.pdf | §III (alertes du propriétaire sur téléphone, non fiscales) | document |
| 42 | ALLERGENES_FR.pdf | §III (allergènes, non fiscaux) | document |
| 43 | SERVICES_CUISINE_FR.pdf | §III (service en plusieurs temps, cuisine) | document |
| 44 | BONS_CADEAUX_FR.pdf | §III.F (bons à usage multiple, UC260 à UC262) | document |

Plus :

- le formulaire de demande de certification signé (remis avec le ZIP, hors des documents ci-dessus) ;
- les preuves brutes du dossier Report_Testing (section 2) ;
- l'empreinte SHA-1 de l'APK signé, déclarée au formulaire §I.D et dans `REPONSES_MANUEL_CERT_FR.md` : À effectuer (EV-01 : SHA-1 et SHA-256 de l'APK signé pili-pos-android 1.0.0) ;
- l'empreinte SHA-1 de la bibliothèque du moteur contenue dans l'APK : À effectuer (EV-02 : SHA-1 de la bibliothèque libpos_ffi.so de l'APK) ;
- l'empreinte du certificat de signature : À effectuer (EV-03 : empreinte SHA-256 du certificat de signature de l'APK).

## 2. Preuves jointes

Le dossier Report_Testing suit le point 3.2 de la procédure du 3 septembre 2026 : requête JSON, réponse JSON et photo ou PDF de chaque ticket imprimé. Dans le dépôt, les preuves sont sous un répertoire evidence ; le ZIP les place sous Report_Testing sans les dupliquer.

| Pièce | Contenu | Valeur |
|---|---|---|
| Report_Testing/uc | requête et réponse JSON de chaque cas d'utilisation (Basic Set 1.8, Advanced Set 1.6, Reports 1.3), jouées sur l'émulateur Android | À effectuer (EV-06 : résultats UC sur l'émulateur Android, requête et réponse JSON par UC) |
| Report_Testing/tickets | tickets imprimés sur l'imprimante simulée, octets de statut retirés, imprimante Epson réelle si disponible | fait : `TICKETS_EXEMPLES_FR.md` et `evidence/tickets/` (impression sur l'Epson réelle : À effectuer (EV-15 : imprimante réelle)) |
| Report_Testing/fdm_errors | requête officielle M110 et réponses du simulateur FDM du dépôt pour 8 codes de message (4 erreurs et 2 avertissements à affichage obligatoire, 2 codes optionnels) | fait : `evidence/fdm_errors/` (`README.md` du répertoire) |
| Report_Testing/build | rapports de construction : empreinte de l'image de construction et SHA-1 de deux APK non signés identiques au commit de la version | À effectuer (EV-05 : empreinte de l'image de construction et SHA-1 des deux APK non signés identiques) |
| CAPTURES_ECRAN_FR.pdf (annexe 28) | captures d'écran du profil sunmi_d3_pro_gms_api33 | captures A01 à A196 du profil D3 Pro émulé, liste dans `internal/CAPTURES_ECRAN_RUNBOOK_FR.md` |
| RESULTATS_TESTS_UC_FR.pdf (annexe 27) | résultats par cas d'utilisation, avec les requêtes et réponses JSON de Report_Testing/uc | À effectuer (EV-06 : résultats UC sur l'émulateur Android) |
| JOURNAL_TESTS_PRE_SOUMISSION_FR.pdf (annexe 29) | déroulement complet sur l'appareil, 24 h de fonctionnement, reprise après coupure | À effectuer (EV-10 : déroulement sur les profils D3 Pro émulés), À effectuer (EV-11 : essai de 24 h du service au premier plan), À effectuer (EV-12 : reprise après coupure d'alimentation) |

Les pièces sur le terminal de test (matériel, simulateur FDM, imprimante, résultats) sont dans `CADRE_DEMANDE_FR.md` §8, et l'état de l'export nocturne sur ce terminal dans son §7. Les mesures propres au D3 Pro avec services Google (applications masquées, politique de mise à jour système, Play Protect) sont dans `PLATEFORME_ANDROID_FR.md` §3 : mesurées le 15.10.2026 sur le profil émulé (EV-13, `evidence/gms/EV13_2026-10-23.md`). Play Protect et la mise à jour du système sur un vrai terminal : À effectuer (EV-15 : D3 Pro réel).
