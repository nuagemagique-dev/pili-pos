<!-- Variante Android, sans équivalent Linux -->
# Cadre de la demande de certification - pili-pos-android

**Distributeur :** Bruno Moyse / nuagemagique.dev (Bruxelles, BE)
**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)

---

## 1. Objet du document

Le producteur certifie son système de caisse `pili-pos-android` 1.0.0, une application Android qui s'exécute sur le terminal Sunmi D3 Pro. Ce document dit comment le producteur cadre cette demande : ce qu'elle couvre, comment la version est identifiée, comment le moteur fiscal est figé et comment les modifications ultérieures seront traitées. Il se lit avant `INDEX_DOSSIER_FR.md`, qui liste les pièces de la demande.

Le cadrage est celui du producteur. Il s'appuie sur le texte de l'arrêté ministériel du 29 avril 2024 (articles 47, 99, 100, 101, 110, 112 et 113) et sur la procédure du SPF Finances du 3 septembre 2026 (Introduction à la certification). Il n'ajoute aucune exigence : il décrit le périmètre de la demande.

## 2. Une demande de certification initiale

L'article 99 prévoit une demande par version ou modèle de système de caisse : « Pour chaque version ou modèle de FDM ou de système de caisse qui sera mis sur le marché belge [...], une demande de certification doit être introduite soit par le fabricant, soit par l'importateur auprès du service compétent du SPF Finances. »

| | |
|---|---|
| Nom | pili-pos-android |
| Version | 1.0.0 |
| Plateforme | Android 13 (API 33), arm64-v8a, Sunmi D3 Pro |
| Demande | initiale, dossier propre (le présent) |
| Certificat actuel | aucun |

`pili-pos-android` 1.0.0 n'est pas certifié aujourd'hui. L'article 113 vise les modifications apportées à « un système de caisse déjà certifié » ; il ne s'applique donc pas à une première version : la version 1.0.0 fait l'objet d'une demande initiale.

Conséquences pour le dossier :

- Il est complet par lui-même, au sens des articles 100 et 101 : la liste des pièces est `INDEX_DOSSIER_FR.md`, l'identification du fabricant est `PROFIL_ENTREPRISE_FR.md`, les distributeurs sont dans `LISTE_DISTRIBUTEURS_FR.md`.
- Il a son propre ZIP, sa propre empreinte SHA-1 (article 112) et, si la demande aboutit, son propre certificat (article 110), qui mentionne la version du logiciel.

## 3. Identification de la version

L'article 47 impose un numéro de version : « Ce numéro de version doit être une mention unique d'une version de logiciel et doit être adapté à chaque modification du logiciel. » Il doit apparaître clairement sur l'écran d'ouverture de session et être envoyé au FDM à chaque communication.

| | |
|---|---|
| Produit | pili-pos-android |
| Version | 1.0.0 |
| posSwVersion envoyé au FDM | pili-pos-android/1.0.0 (22 caractères, la limite de la spécification est 36) |
| Identifiant d'application | dev.nuagemagique.pilipos |
| versionCode | 10000 (MAJOR x 10000 + MINOR x 100 + PATCH) |
| Étiquette git de la version | android-v1.0.0 (`PROFIL_ENTREPRISE_FR.md` §4.2) |
| Artefact déclaré | l'APK signé, par son SHA-1 (`GARANTIE_MAJ_LANGUES_FR.md` §1.2) |

La chaîne pili-pos-android/1.0.0 désigne sans ambiguïté cette version, sur un ticket comme dans le journal du FDM.

La version vient d'une seule source : le numéro de version de la bibliothèque du moteur (`android/rust/pos-ffi/Cargo.toml`). Le nom de version et le versionCode de l'APK sont calculés à partir de cette valeur unique, et la construction échoue si elle n'a pas la forme MAJEUR.MINEUR.CORRECTIF avec un suffixe -dev facultatif. Elle n'est lue dans aucun fichier de configuration et ne peut donc pas être modifiée sur un terminal. Une construction de développement porte le suffixe -dev et n'est jamais une version déclarée.

Le terminal montre la version à deux endroits : l'écran de connexion, sous les contrôles de démarrage (article 47), et l'écran À propos, ouvert depuis les réglages. L'écran À propos montre aussi la référence du moteur (ligne « Référence du moteur » : `pili-pos 1.0.0, android-v1.0.0-rc3` pour la pré-version de gel), le versionCode, les empreintes de l'APK et de la bibliothèque du moteur, l'empreinte du certificat de signature et l'identifiant de l'appareil (`PLATEFORME_ANDROID_FR.md` §4). L'identifiant d'application figure dans le fichier identity.json de l'export destiné au SPF.

## 4. Moteur fiscal

Le moteur fiscal (`crates/pos-engine` et ses crates de la build de certification : `pos-core`, `pos-fiscal`, `pos-db`, `pos-print`, `pos-mail`, `pos-hubrise`, `pos-catalog`, `pos-export`) est compilé dans `libpos_ffi.so`, la bibliothèque native de l'APK, et appelé par l'interface Kotlin au travers de la couche UniFFI. Aucune règle fiscale ou monétaire n'existe côté Kotlin : le moteur seul décide des événements envoyés au FDM, des totaux, des compteurs et des tickets (`PLATEFORME_ANDROID_FR.md` §2).

- Le dossier cite un seul état du moteur : l'étiquette de gel `android-v1.0.0-rc3`, puis l'étiquette finale `android-v1.0.0`. Valeurs de cette étiquette : À effectuer (EV-04 : commit de l'étiquette de gel android-v1.0.0-rc3, puis de l'étiquette finale android-v1.0.0).
- Le producteur prouve que le moteur ne change pas après le gel par un contrôle : à chaque modification, `scripts/android/check_android_frozen.sh` compare à l'étiquette de gel les chemins qui entrent dans la construction de l'APK (le moteur et ses crates, la façade UniFFI, la configuration de construction et le code de l'application) ; il échoue si l'un d'eux a changé.
- La reproductibilité de l'APK non signé est montrée par `scripts/android/check_repro.sh` (deux constructions complètes hors réseau, dans un conteneur, donnent un APK identique octet pour octet ; `GARANTIE_MAJ_LANGUES_FR.md` §1).
- Six documents décrivent le moteur seul : le schéma de la base (`SCHEMA_BD_FR.md`), le schéma entité-relation (`SCHEMA_ER_FR.md`), la stratégie des compteurs (`STRATEGIE_COMPTEURS_FR.md`), la couverture des fixtures (`COUVERTURE_FIXTURES_FR.md`), la couverture des rapports (`COUVERTURE_RAPPORTS_FR.md`) et le ticket numérique (`TICKET_NUMERIQUE_FR.md`). Ils font partie du dossier et sont listés dans `INDEX_DOSSIER_FR.md`.

## 5. Plateforme

| Sujet | Valeur | Annexe |
|---|---|---|
| Matériel | Sunmi D3 Pro, arm64-v8a, 1920 x 1080 | `PLATEFORME_ANDROID_FR.md` §1, `SCHEMA_ASSEMBLAGE_FR.md` |
| Système | Android 13 (API 33), application propriétaire de l'appareil | `PLATEFORME_ANDROID_FR.md` §3 |
| Interface | Kotlin et Compose, sur la bibliothèque du moteur | `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` |
| Identifiant de l'appareil | numéro de série matériel Sunmi, lu par le propriétaire de l'appareil (FAQ Q505 b) | `PLATEFORME_ANDROID_FR.md` §4 |
| Données | base SQLite dans le stockage privé de l'application, jamais sur la carte SD | `PLATEFORME_ANDROID_FR.md` §5 |
| Verrouillage | mode propriétaire de l'appareil, mode Lock Task, restrictions système | `PLATEFORME_ANDROID_FR.md` §3 |
| Mise à jour | mise à jour lancée par le gérant, artefact déclaré seul, depuis un registre public ou une clé USB | `GARANTIE_MAJ_LANGUES_FR.md` §2.1 |
| Accès du SPF aux données | export par le gérant sur clé USB ou carte SD (articles 16 et 102) | `SECURITE_BD_FR.md` §8 |
| Assistance à distance | aucune ; assistance sur place | `PLATEFORME_ANDROID_FR.md` §7 |
| Construction et empreinte | APK reproductible construit en conteneur, signé ensuite ; SHA-1 de l'APK signé, SHA-1 de la bibliothèque du moteur | `GARANTIE_MAJ_LANGUES_FR.md` §1 |
| Configuration | paquet de configuration signé, importé depuis une clé USB | `MANUEL_INSTALLATION_FR.md` |

## 6. Modifications après certification

Une fois son certificat reçu, le producteur suit l'article 113 : il communique chaque modification au service compétent au moins un mois avant de mettre la nouvelle version sur le marché belge, et il regroupe les mises à jour pour rester sous la limite de 12 par année civile prévue par le 2° du même article.

- Toute modification, du moteur comme de l'interface, du verrouillage, de la mise à jour, de l'export ou de la construction, donne une nouvelle version de `pili-pos-android`, avec sa propre empreinte.
- Pour les corrections urgentes de bugs, le producteur applique la règle de la procédure du SPF Finances : elles « constituent une exception à cette règle, mais doivent tout de même nous être signalées ». Il les signale et les reprend dans la demande de mise à jour suivante (`REPONSES_MANUEL_CERT_FR.md` §II.E).
- Les mises à jour du système Android ne changent pas l'APK déclaré. Elles sont différées par le propriétaire de l'appareil (`GARANTIE_MAJ_LANGUES_FR.md` §2.2).
- La règle qui conserve les 7 dernières sauvegardes après Z (`SECURITE_BD_FR.md` §5) fait partie du moteur de l'étiquette de gel. Avant la première certification, ce n'est pas une modification au sens de l'article 113. Commit de cette règle : `4ff25b110c975d497f548538a08d81387b504b25` (commit « feat(engine): keep only the 7 newest database backups » du 2026-10-10, contenu dans l'étiquette de gel android-v1.0.0-rc3).

## 7. Fonctions déclarées

| Fonction | Déclarée | Annexe |
|---|---|---|
| Export nocturne (facultatif) | Oui. Copie chiffrée de la base et copie statistique pseudonymisée envoyées par HTTPS ; c'est une copie, le journal légal reste sur le terminal. Activé seulement si la section export de la configuration est renseignée. | `EXPORT_NOCTURNE_FR.md` |
| Commandes en ligne (HubRise) | Oui. Interrupteur par installation, coupé par défaut ; chaque commande se termine par un événement N signé par le FDM. | `COMMANDES_EN_LIGNE_FR.md` |
| Ticket numérique par e-mail | Oui, à la demande du client à la caisse. | `TICKET_NUMERIQUE_FR.md` |
| Facturation B2B (événement I, Peppol) | Oui. Fait partie de la certification 1.0.0 ; l'envoi Peppol n'a lieu que si l'installation contient une section `peppol`. | `FACTURATION_PEPPOL_FR.md`, `DECISION_PEPPOL_AP_FR.md` |
| Produit épuisé | Oui. Aucun effet fiscal. | `PRODUIT_EPUISE_FR.md` |
| Éditeur de carte | Oui. Aucun effet fiscal. | `EDITEUR_CARTE_FR.md` |
| Rapport de ventes non fiscal | Oui. Ne signe rien. | `RAPPORTS_VENTES_FR.md` |

Le terminal de test porte l'état de l'export nocturne (activé ou non, détenteurs des clés de déchiffrement, exploitant du récepteur). Valeur : À effectuer (EV-20 : état de l'export nocturne sur la machine de test).

Deux fonctions n'interviennent pas dans le calcul fiscal : l'export des données pour le SPF (`SECURITE_BD_FR.md` §8) et la mise à jour lancée par le gérant (`GARANTIE_MAJ_LANGUES_FR.md` §2).

## 8. Machine de test

- **Terminal.** La machine de test Android est le profil émulé sunmi_d3_pro_gms_api33 (Sunmi D3 Pro, 1920 x 1080 en paysage, Android 13 arm64), sauf si un D3 Pro réel est disponible pour le rendez-vous d'admission. Les faits matériels d'un D3 Pro réel (format du numéro de série, stockage, fente de carte, démarrage vérifié, chiffrement, comportement de l'imprimante Epson, durées) sont : À effectuer (EV-15 : faits d'un D3 Pro réel, ou confirmation que la machine de test est le profil émulé).
- **Identifiant de l'appareil.** Valeur imprimée sur les tickets de la machine de test : `EMULATOR37X2X12X0` (profil émulé ; écran « À propos », capture A82, et tickets de `TICKETS_EXEMPLES_FR.md`).
- **FDM.** Le simulateur en ligne de module2. Passage en conditions réelles contre ce simulateur : À effectuer (EV-14 : passage réel contre le simulateur module2, avec le terminal POS-2-ANDROID).
- **Imprimante.** Une Epson TM-m30III sur le réseau local (port 9100) ; faute de matériel, l'imprimante simulée du dépôt reçoit les tickets (`SCHEMA_ASSEMBLAGE_FR.md`). Exemples de tickets : imprimés sur l'imprimante simulée, dans `TICKETS_EXEMPLES_FR.md` (impression sur l'Epson réelle : À effectuer (EV-15 : imprimante réelle)).
- **Résultats.** Cas d'utilisation Basic Set 1.8, Advanced Set 1.6 et Reports 1.3 sur l'émulateur : À effectuer (EV-06 : résultats UC sur l'émulateur Android). Déroulement complet sur l'appareil (provisionnement, vente, remboursement, Z, export SPF, mise à jour, verrouillage) : À effectuer (EV-10 : déroulement sur le profil D3 Pro).
- **Totaux de tests à la version déposée.** À effectuer (EV-17 : comptes finaux des tests à la version déposée).
