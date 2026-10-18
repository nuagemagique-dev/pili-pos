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

Natures : **variante Android** (document propre à ce dossier, dérivé du dossier Linux ou nouveau), **document commun** (moteur seul, repris du dossier Linux au même état du moteur, section 2), **preuve** (pièce remplie avec les résultats des tests et captures).

| # | Fichier | Section du manuel couverte | Nature |
|---|---|---|---|
| 0 | INDEX_DOSSIER_FR.pdf (ce document) | liste des documents (article 101), lecture des documents communs | variante Android |
| 1 | REPONSES_MANUEL_CERT_FR.pdf | parcours linéaire §I-VIII | variante Android |
| 2 | LISTE_DISTRIBUTEURS_FR.pdf | §I.E | variante Android |
| 3 | PROFIL_ENTREPRISE_FR.pdf | §I.F | variante Android |
| 4 | MANUEL_UTILISATEUR_FR.pdf | §I.C | variante Android |
| 5 | MANUEL_INSTALLATION_FR.pdf | §I.C | variante Android |
| 6 | MANUEL_PROGRAMMATION_FR.pdf | §I.C | variante Android |
| 7 | DECLARATION_FONCTIONS_INTERDITES_FR.pdf | §III.C | variante Android |
| 8 | PROFILS_UTILISATEURS_FR.pdf | §III.C | variante Android |
| 9 | DOCUMENT_PROCESSUS_TECHNIQUE_FR.pdf | §II + §III.E | variante Android |
| 10 | FLUX_DONNEES_FR.pdf | §II.C + §III | variante Android |
| 11 | SCHEMA_BD_FR.pdf | §II.C | document commun |
| 12 | SECURITE_BD_FR.pdf | §II.C, accès du SPF aux données | variante Android |
| 13 | SCHEMA_ER_FR.pdf | §II.C | document commun |
| 14 | SCHEMA_ASSEMBLAGE_FR.pdf | §I.C + §II.G | variante Android |
| 15 | LISTE_FONCTIONNALITES_FR.pdf | §III | variante Android |
| 16 | LIGNES_PRODUITS_FR.pdf | §III, lignes de produit | variante Android |
| 17 | STRATEGIE_COMPTEURS_FR.pdf | §III.B (numérotation) + FDM non opérationnel | document commun |
| 18 | FLUX_REMBOURSEMENT_FR.pdf | §III.F | variante Android |
| 19 | TICKETS_EXEMPLES_FR.pdf | §VI | preuve |
| 20 | DECISION_PEPPOL_AP_FR.pdf | §III.F (encadré B2B) | variante Android |
| 21 | MAPPING_CONFORMITE_FR.pdf | §IV + §V | variante Android |
| 22 | FEUILLE_ACQUISITION_DONNEES_FR.pdf | §VII (acquisition des données) | variante Android |
| 23 | RAPPORT_TESTS_BASELINE_FR.pdf | §IV + §VIII | variante Android |
| 24 | COUVERTURE_FIXTURES_FR.pdf | §IV | document commun |
| 25 | COUVERTURE_RAPPORTS_FR.pdf | §IV | document commun |
| 26 | COUVERTURE_ADVANCED_SET_FR.pdf | §IV | variante Android |
| 27 | RESULTATS_TESTS_UC_FR.pdf | §IV, résultats des cas d'utilisation | preuve |
| 28 | CAPTURES_ECRAN_FR.pdf | §V + §VII.D | preuve |
| 29 | JOURNAL_TESTS_PRE_SOUMISSION_FR.pdf | §IV, répétition pré-soumission | preuve |
| 30 | CHECKLIST_DEPLOIEMENT_FR.pdf | machine de test | variante Android |
| 31 | GARANTIE_MAJ_LANGUES_FR.pdf | §I.D, §II.D, §II.E, §VI | variante Android |
| 32 | TICKET_NUMERIQUE_FR.pdf | §III.H | document commun |
| 33 | COMMANDES_EN_LIGNE_FR.pdf | §III.M, commandes en ligne | variante Android |
| 34 | EXPORT_NOCTURNE_FR.pdf | Configuration Internet + §VII.B + acquisition des données (export nocturne facultatif) | variante Android |
| 35 | CADRE_DEMANDE_FR.pdf | cadrage de la demande (article 99) | variante Android |
| 36 | PLATEFORME_ANDROID_FR.pdf | plateforme, verrouillage, identifiant de l'appareil, stockage | variante Android |
| 37 | FACTURATION_PEPPOL_FR.pdf | §III.F (facturation B2B, événement I, envoi Peppol) | variante Android |
| 38 | EDITEUR_CARTE_FR.pdf | §III (édition de la carte à l'écran) | variante Android |
| 39 | PRODUIT_EPUISE_FR.pdf | §III (produit épuisé) | variante Android |
| 40 | RAPPORTS_VENTES_FR.pdf | §III.L (rapport de ventes non fiscal) | variante Android |
| 41 | ALERTES_PROPRIETAIRE_FR.pdf | §III (alertes du propriétaire sur téléphone, non fiscales) | variante Android |
| 42 | ALLERGENES_FR.pdf | §III (allergènes, non fiscaux) | variante Android |
| 43 | SERVICES_CUISINE_FR.pdf | §III (service en plusieurs temps, cuisine) | variante Android |
| 44 | BONS_CADEAUX_FR.pdf | §III.F (bons à usage multiple, UC260 à UC262) | variante Android |

Plus :

- le formulaire de demande de certification signé (remis avec le ZIP, hors des documents ci-dessus) ;
- les preuves brutes du dossier Report_Testing (section 4) ;
- l'empreinte SHA-1 de l'APK signé, déclarée au formulaire §I.D et dans `REPONSES_MANUEL_CERT_FR.md` : À effectuer (EV-01 : SHA-1 et SHA-256 de l'APK signé pili-pos-android 1.0.0) ;
- l'empreinte SHA-1 de la bibliothèque du moteur contenue dans l'APK : À effectuer (EV-02 : SHA-1 de la bibliothèque libpos_ffi.so de l'APK) ;
- l'empreinte du certificat de signature : À effectuer (EV-03 : empreinte SHA-256 du certificat de signature de l'APK).

## 2. Documents communs aux deux demandes

Six annexes décrivent le moteur fiscal seul : `SCHEMA_BD_FR.md`, `SCHEMA_ER_FR.md`, `STRATEGIE_COMPTEURS_FR.md`, `COUVERTURE_FIXTURES_FR.md`, `COUVERTURE_RAPPORTS_FR.md` et `TICKET_NUMERIQUE_FR.md`. Le moteur est le même code source dans les deux produits (`CADRE_DEMANDE_FR.md` §4). Ces six documents sont donc les mêmes pour le dossier Linux et pour le dossier Android, et ne sont pas recopiés.

- Le PDF de chacun est celui du dossier Linux, produit au même état du moteur : l'étiquette de gel `linux-v1.0.0-rc2`, puis l'étiquette finale. Commit de cette étiquette : À effectuer (EV-04 : commit de l'étiquette de gel du moteur et SHA-1 du binaire Linux du même moteur).
- Le ZIP Android contient ces six PDF sous leur nom habituel. Dans `MANIFEST.txt`, ils portent le préfixe commun: pour les distinguer des documents propres au dossier Android.
- Un document commun est vrai pour les deux produits, y compris ses tests, ses chemins de code et ses commandes (`cargo test` sur les crates du moteur). Seuls quelques passages citent la plateforme Linux ; la section 3 donne leur lecture sur Android.
- Un renvoi, dans un document commun, vers une annexe qui existe en deux versions (par exemple `MANUEL_INSTALLATION_FR.md` ou `MANUEL_UTILISATEUR_FR.md`) désigne, dans le ZIP Android, l'annexe Android du même nom. Le mot « binaire » d'un document commun désigne, sur Android, le moteur contenu dans l'APK.

## 3. Lecture des documents communs sur Android

| Document commun | Passage propre à Linux | Lecture sur Android |
|---|---|---|
| `SCHEMA_BD_FR.md` | En-tête : fichier vivant /var/lib/pili-pos/pos.db | La base est le fichier files/engine/pos.db du stockage privé de l'application, jamais sur la carte SD (`PLATEFORME_ANDROID_FR.md` §5). Le schéma est le même. |
| `SCHEMA_BD_FR.md` | En-tête : lecture machine par un shell sqlite3 sur la base | Le terminal verrouillé n'a pas de shell. Le schéma se lit dans la copie de pos.db de l'export SPF, avec n'importe quel outil SQLite (`SECURITE_BD_FR.md` §8). |
| `SCHEMA_BD_FR.md` | Déclencheur `pending_signs_no_delete` : procédure de dépannage du MANUEL_INSTALLATION_FR.md | Procédure de dépannage du `MANUEL_INSTALLATION_FR.md` Android (annexe 5). Les déclencheurs sont ceux du moteur. |
| `SCHEMA_BD_FR.md` | « Binaire certifié » : migrations 12 à 19 réservées ; tables de facturation et Peppol (migration `040`), épuisé et journal de la carte (migrations `041` et `042`), réglages du propriétaire, allergènes, temps de cuisine et bons cadeaux (migrations `060` à `063`) | Le moteur de l'APK est construit avec le même ensemble de fonctions de certification et contient les mêmes migrations et les mêmes tables (`FACTURATION_PEPPOL_FR.md`). L'envoi Peppol n'a lieu que si l'installation contient une section `peppol`. |
| `SCHEMA_ER_FR.md` | Aucun passage propre à Linux (sources dans `crates/pos-db`, « binaire certifié » comme ci-dessus) | Se lit tel quel. |
| `STRATEGIE_COMPTEURS_FR.md` | Blocage sur déconnexion du FDM : événement fdm-status reçu par l'interface Vue, messages dans `ui/src/utils/fdmMessages.ts` | L'événement est le même : il sort du moteur et arrive à l'interface Compose par la couche UniFFI. Les messages français sont dans `android/app/src/main/res/values/strings_vue_fdm.xml`. Même voyant rouge et même voile sur les écrans de vente. |
| `STRATEGIE_COMPTEURS_FR.md` | Requête sans réponse : procédure du MANUEL_INSTALLATION_FR.md et du MANUEL_UTILISATEUR_FR.md | Procédure des deux manuels Android du même nom. |
| `COUVERTURE_FIXTURES_FR.md` | Colonne « E2e applicatif » : M122 couvert par la commande `transfer_table`, M132 par la commande `correct_payment` | Sur Android, les commandes `transfer_table` (panier et écran des tables) et `correct_payment` (onglet Tickets des rapports) du moteur sont appelées par l'interface. Les tests pilotent les fonctions `do_*` du moteur, comme sur Linux. |
| `COUVERTURE_RAPPORTS_FR.md` | « Build de soumission » | La build de soumission est la build de certification du moteur, dans les deux produits. Aucun autre passage propre à Linux. |
| `TICKET_NUMERIQUE_FR.md` | Configuration : bloc `digital_ticket` du pos.json, selon le MANUEL_INSTALLATION_FR.md §4.2 | Le fichier pos.json est le même. Sur Android, il voyage dans le paquet de configuration signé importé depuis une clé USB (`MANUEL_INSTALLATION_FR.md` Android, section de configuration du ticket numérique). |
| `TICKET_NUMERIQUE_FR.md` | Saisie de l'adresse sur un clavier virtuel à l'écran | Clavier à l'écran de l'application Android. Mêmes règles de validation, mêmes codes d'erreur (`email_invalid`, `digital_unavailable`). |
| `TICKET_NUMERIQUE_FR.md` | « Écran de support » qui liste les envois sans adresse en clair, écran « Copie de ticket » du MANUEL_UTILISATEUR_FR.md | Panneau des envois de tickets des réglages Android et écran de copie de l'application, décrits dans le `MANUEL_UTILISATEUR_FR.md` Android. Seul l'indice masqué est affiché, comme sur Linux. |
| `TICKET_NUMERIQUE_FR.md` | Renvoi à la capture `09e` de *CAPTURES_ECRAN_FR.pdf* (écran de repli papier avec `MOCK_TEM_FAIL=1`) | Le renvoi désigne le *CAPTURES_ECRAN_FR.pdf* Android, dont les identifiants sont de la forme A<nn> et ne comprennent pas de 09e. Captures équivalentes : A27 (repli papier) et A117 (e-mail en attente). |

Ces passages ont été relevés en cherchant dans chaque document commun les mots « Linux », « Debian », « Tauri », « Vue », « systemd », « RustDesk », « USB », les chemins système et les écrans de l'interface. Un chiffre de test d'un document commun (par exemple un total de tests de `COUVERTURE_RAPPORTS_FR.md`) est celui du moteur ; les totaux propres à Android sont dans `RAPPORT_TESTS_BASELINE_FR.md`.

## 4. Preuves jointes

Le dossier Report_Testing suit le point 3.2 de la procédure du 3 septembre 2026 : requête JSON, réponse JSON et photo ou PDF de chaque ticket imprimé. Dans le dépôt, les preuves sont sous un répertoire evidence ; le ZIP les place sous Report_Testing sans les dupliquer.

| Pièce | Contenu | Valeur |
|---|---|---|
| Report_Testing/uc | requête et réponse JSON de chaque cas d'utilisation (Basic Set 1.8, Advanced Set 1.6, Reports 1.3), jouées sur l'émulateur Android | À effectuer (EV-06 : résultats UC sur l'émulateur Android, requête et réponse JSON par UC) |
| Report_Testing/tickets | tickets imprimés sur l'imprimante simulée, octets de statut retirés, imprimante Epson réelle si disponible | fait : `TICKETS_EXEMPLES_FR.md` et `evidence/tickets/` (impression sur l'Epson réelle : À effectuer (EV-15 : imprimante réelle)) |
| Report_Testing/build | rapports de construction : empreinte de l'image de construction et SHA-1 de deux APK non signés identiques au commit de la version | À effectuer (EV-05 : empreinte de l'image de construction et SHA-1 des deux APK non signés identiques) |
| CAPTURES_ECRAN_FR.pdf (annexe 28) | captures d'écran du profil sunmi_d3_pro_gms_api33 | 122 captures A01 à A122 du profil D3 Pro émulé, liste dans `internal/CAPTURES_ECRAN_RUNBOOK_FR.md` |
| RESULTATS_TESTS_UC_FR.pdf (annexe 27) | résultats par cas d'utilisation et comparaison avec le journal Linux | À effectuer (EV-07 : comparaison des journaux et requêtes FDM entre Android et Linux) |
| JOURNAL_TESTS_PRE_SOUMISSION_FR.pdf (annexe 29) | déroulement complet sur l'appareil, 24 h de fonctionnement, reprise après coupure | À effectuer (EV-10 : déroulement sur les profils D3 Pro émulés), À effectuer (EV-11 : essai de 24 h du service au premier plan), À effectuer (EV-12 : reprise après coupure d'alimentation) |

Les pièces sur le terminal de test (matériel, simulateur FDM, imprimante, résultats) sont dans `CADRE_DEMANDE_FR.md` §8, et l'état de l'export nocturne sur ce terminal dans son §7. Les mesures propres au D3 Pro avec services Google (applications masquées, politique de mise à jour système, Play Protect) sont dans `PLATEFORME_ANDROID_FR.md` §3 : mesurées le 15.10.2026 sur le profil émulé (EV-13, `evidence/gms/EV13_2026-10-15.md`). Play Protect et la mise à jour du système sur un vrai terminal : À effectuer (EV-15 : D3 Pro réel).
