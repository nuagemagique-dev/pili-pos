<!-- Variante Android, sans équivalent Linux -->
# Cadre de la demande de certification - pili-pos-android

**Distributeur :** Bruno Moyse / nuagemagique.dev (Bruxelles, BE)
**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)

---

## 1. Objet du document

Le producteur certifie son système de caisse sur deux plateformes : `pili-pos` sur Linux x86_64 (terminal kiosque) et `pili-pos-android` sur Android (terminal Sunmi D3 Pro). Ce document dit comment le producteur cadre les deux demandes, ce qu'elles ont en commun et ce qui les distingue. Il se lit avant `INDEX_DOSSIER_FR.md`, qui liste les pièces de la demande Android.

Le cadrage est celui du producteur. Il s'appuie sur le texte de l'arrêté ministériel du 29 avril 2024 (articles 47, 99, 100, 101, 110, 112 et 113) et sur la procédure du SPF Finances du 3 septembre 2026 (Introduction à la certification). Il n'ajoute aucune exigence : il décrit le périmètre de chaque demande.

## 2. Deux demandes de certification initiales distinctes

L'article 99 prévoit une demande par version ou modèle de système de caisse : « Pour chaque version ou modèle de FDM ou de système de caisse qui sera mis sur le marché belge [...], une demande de certification doit être introduite soit par le fabricant, soit par l'importateur auprès du service compétent du SPF Finances. »

| | Produit Linux | Produit Android |
|---|---|---|
| Nom | pili-pos | pili-pos-android |
| Version | 1.0.0 | 1.0.0 |
| Plateforme | Linux (Debian 13 minimal), x86_64, terminal kiosque | Android 13 (API 33), arm64-v8a, Sunmi D3 Pro |
| Demande | initiale, dossier propre | initiale, dossier propre (le présent) |
| Certificat actuel | aucun | aucun |

Le producteur lit l'article 99 ainsi : `pili-pos` 1.0.0 et `pili-pos-android` 1.0.0 sont deux produits, car ils diffèrent par le système, le matériel, la couche d'interface, la chaîne de construction et l'empreinte (section 5). Chacun fait l'objet de sa propre demande.

Aucun des deux produits n'est certifié aujourd'hui. L'article 113 vise les modifications apportées à « un système de caisse déjà certifié » ; il ne s'applique donc à aucune des deux demandes. L'application Android n'est pas une modification de l'application Linux, et la version 1.0.0 de chacune est une première version.

Conséquences pour le dossier Android :

- Il est complet par lui-même, au sens des articles 100 et 101 : la liste des pièces est `INDEX_DOSSIER_FR.md`, l'identification du fabricant est `PROFIL_ENTREPRISE_FR.md`, les distributeurs sont dans `LISTE_DISTRIBUTEURS_FR.md`.
- Il a son propre ZIP, sa propre empreinte SHA-1 (article 112) et, si la demande aboutit, son propre certificat (article 110), qui mentionne la version du logiciel.
- Le dossier Linux est déposé séparément. Le présent dossier ne le modifie pas ; il en reprend le texte pour tout ce qui ne dépend pas de la plateforme.

## 3. Identification des versions

L'article 47 impose un numéro de version : « Ce numéro de version doit être une mention unique d'une version de logiciel et doit être adapté à chaque modification du logiciel. » Il doit apparaître clairement sur l'écran d'ouverture de session et être envoyé au FDM à chaque communication.

| | Linux | Android |
|---|---|---|
| Produit | pili-pos | pili-pos-android |
| Version | 1.0.0 | 1.0.0 |
| posSwVersion envoyé au FDM | pili-pos/1.0.0 | pili-pos-android/1.0.0 (22 caractères, la limite de la spécification est 36) |
| Identifiant d'application | sans objet | dev.nuagemagique.pilipos |
| versionCode | sans objet | 10000 (MAJOR x 10000 + MINOR x 100 + PATCH) |
| Étiquette git de la version | vX.Y.Z (`PROFIL_ENTREPRISE_FR.md` §4.2) | android-v1.0.0 |
| Artefact déclaré | le binaire, par son SHA-1 | l'APK signé, par son SHA-1 (`GARANTIE_MAJ_LANGUES_FR.md` §1.2) |

Les deux lignes de produit portent des chaînes différentes : une version du produit Android ne peut jamais être confondue avec une version du produit Linux, ni sur un ticket, ni dans le journal du FDM.

Sur Android, la version vient d'une seule source : le numéro de version de la bibliothèque du moteur (`android/rust/pos-ffi/Cargo.toml`). Le nom de version et le versionCode de l'APK sont calculés à partir de cette valeur unique, et la construction échoue si elle n'a pas la forme MAJEUR.MINEUR.CORRECTIF avec un suffixe -dev facultatif. Elle n'est lue dans aucun fichier de configuration et ne peut donc pas être modifiée sur un terminal. Une construction de développement porte le suffixe -dev et n'est jamais une version déclarée.

Le terminal montre la version à deux endroits : l'écran de connexion, sous les contrôles de démarrage (article 47), et l'écran À propos, ouvert depuis les réglages. L'écran À propos montre aussi la version du moteur, le versionCode, les empreintes de l'APK et de la bibliothèque du moteur, l'empreinte du certificat de signature et l'identifiant de l'appareil (`PLATEFORME_ANDROID_FR.md` §4). L'identifiant d'application figure dans le fichier identity.json de l'export destiné au SPF.

## 4. Moteur fiscal commun

Les deux produits embarquent le même moteur fiscal, avec les mêmes règles, le même schéma de base de données et les mêmes tickets : la caisse `crates/pos-engine` et les crates de la build de certification (`pos-core`, `pos-fiscal`, `pos-db`, `pos-print`, `pos-mail`, `pos-hubrise`, `pos-catalog`, `pos-export`).

- Les deux dossiers citent le même état du moteur : l'étiquette de gel `linux-v1.0.0-rc2`, puis l'étiquette finale. Valeurs de cette étiquette : À effectuer (EV-04 : commit de l'étiquette et SHA-1 du binaire Linux du même moteur).
- Sur Linux, le moteur est lié au binaire Tauri. Sur Android, il est compilé dans la bibliothèque libpos_ffi.so, à l'intérieur de l'APK, et appelé par l'interface Kotlin au travers de la couche UniFFI. Aucune règle fiscale ou monétaire n'existe côté Kotlin : le moteur seul décide des événements envoyés au FDM, des totaux, des compteurs et des tickets (`PLATEFORME_ANDROID_FR.md` §2).
- Le producteur le prouve de trois façons :
  1. le gel du moteur : un contrôle compare, à chaque modification, les chemins du moteur et de la build Linux avec l'étiquette de gel (`scripts/android/check_linux_frozen.sh`) ; il échoue si l'un d'eux a changé ;
  2. la parité des commandes : chacune des 128 commandes de la build de certification, appelées par l'interface Linux, a sa méthode du même nom dans la couche UniFFI utilisée par l'interface Android, avec les mêmes paramètres (`scripts/android/check_command_parity.py`, résultat 128 sur 128) ;
  3. la comparaison des journaux : les mêmes scénarios, joués sur les deux produits, donnent les mêmes requêtes au FDM et le même journal fiscal. Résultat : À effectuer (EV-07 : comparaison du journal et des requêtes FDM entre Android et Linux).
- Six documents décrivent le moteur seul. Ils sont communs aux deux demandes et ne sont pas recopiés : `INDEX_DOSSIER_FR.md` §2 et §3 donnent leur lecture sur Android.

## 5. Ce qui diffère entre les deux produits

| Sujet | Linux (pili-pos) | Android (pili-pos-android) | Annexe Android |
|---|---|---|---|
| Matériel | Terminal tactile x86_64 (Penetek A3216 M82 ou Advantech USC-360J) | Sunmi D3 Pro, arm64-v8a, 1920 x 1080 | `PLATEFORME_ANDROID_FR.md` §1, `SCHEMA_ASSEMBLAGE_FR.md` |
| Système | Debian 13 minimal, compositeur Cage, système de fichiers racine en lecture seule | Android 13 (API 33), application propriétaire de l'appareil | `PLATEFORME_ANDROID_FR.md` §3 |
| Interface | Vue 3 dans une coquille Tauri | Kotlin et Compose, sur la bibliothèque du moteur | `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` |
| Identifiant de l'appareil | adresse MAC, inscrite au premier démarrage | numéro de série matériel Sunmi, lu par le propriétaire de l'appareil (FAQ Q505 b) | `PLATEFORME_ANDROID_FR.md` §4 |
| Données | base SQLite dans le répertoire système de l'installation | base SQLite dans le stockage privé de l'application, jamais sur la carte SD | `PLATEFORME_ANDROID_FR.md` §5 |
| Verrouillage | image scellée, session unique | mode propriétaire de l'appareil, mode Lock Task, restrictions système | `PLATEFORME_ANDROID_FR.md` §3 |
| Mise à jour | binaire remis sur clé USB, installé par un script qui contrôle le SHA-1 | mise à jour lancée par le gérant, artefact déclaré seul, depuis un registre public ou une clé USB | `GARANTIE_MAJ_LANGUES_FR.md` §2.1 |
| Accès du SPF aux données | console locale et lecture seule de la base | export par le gérant sur clé USB ou carte SD (articles 16 et 102) | `SECURITE_BD_FR.md` §8 |
| Assistance à distance | session RustDesk démarrée par un gérant | aucune ; assistance sur place | `PLATEFORME_ANDROID_FR.md` §7 |
| Construction et empreinte | binaire construit en conteneur, SHA-1 du binaire | APK reproductible construit en conteneur, signé ensuite ; SHA-1 de l'APK signé, SHA-1 de la bibliothèque du moteur | `GARANTIE_MAJ_LANGUES_FR.md` §1 |
| Configuration | fichier de configuration du système | paquet de configuration signé, importé depuis une clé USB | `MANUEL_INSTALLATION_FR.md` |

Le moteur, les règles fiscales, le schéma de base, les tickets et les rapports sont identiques dans les deux colonnes.

## 6. Modifications après certification

Une fois son certificat reçu, chaque produit suit l'article 113 pour son propre compte : le producteur communique chaque modification au service compétent au moins un mois avant de mettre la nouvelle version sur le marché belge, et il regroupe les mises à jour pour rester sous la limite de 12 par année civile prévue par le 2° du même article.

- Une modification du moteur concerne les deux produits : elle est déposée pour les deux, avec le même état du moteur et, pour chacun, sa propre empreinte.
- Une modification propre à une plateforme (interface, verrouillage, mise à jour, export, construction) ne concerne que le produit en cause.
- Pour les corrections urgentes de bugs, le producteur applique la règle de la procédure du SPF Finances : elles « constituent une exception à cette règle, mais doivent tout de même nous être signalées ». Il les signale et les reprend dans la demande de mise à jour suivante (`REPONSES_MANUEL_CERT_FR.md` §II.E).
- Les mises à jour du système Android ne changent pas l'APK déclaré. Elles sont différées par le propriétaire de l'appareil (`GARANTIE_MAJ_LANGUES_FR.md` §2.2).
- La règle qui conserve les 7 dernières sauvegardes après Z (`SECURITE_BD_FR.md` §5) fait partie du moteur de l'étiquette de gel. Avant la première certification, ce n'est pas une modification au sens de l'article 113. Commit de cette règle dans l'étiquette : À effectuer (EV-19 : commit du changement du moteur dans l'étiquette de gel).

## 7. Fonctions déclarées dans les deux demandes

| Fonction | Dans les deux produits | Annexe |
|---|---|---|
| Export nocturne (facultatif) | Oui. Copie chiffrée de la base et copie statistique pseudonymisée envoyées par HTTPS ; c'est une copie, le journal légal reste sur le terminal. Activé seulement si la section export de la configuration est renseignée. | `EXPORT_NOCTURNE_FR.md` |
| Commandes en ligne (HubRise) | Oui. Interrupteur par installation, coupé par défaut ; chaque commande se termine par un événement N signé par le FDM. | `COMMANDES_EN_LIGNE_FR.md` |
| Ticket numérique par e-mail | Oui, à la demande du client à la caisse. | `TICKET_NUMERIQUE_FR.md` (commun) |
| Facturation B2B (événement I, Peppol) | Oui. Fait partie de la certification 1.0.0, dans le même moteur que sous Linux ; l'envoi Peppol n'a lieu que si l'installation contient une section `peppol`. | `FACTURATION_PEPPOL_FR.md`, `DECISION_PEPPOL_AP_FR.md` |
| Produit épuisé | Oui. Aucun effet fiscal. | `PRODUIT_EPUISE_FR.md` |
| Éditeur de carte | Oui. Aucun effet fiscal. | `EDITEUR_CARTE_FR.md` |
| Rapport de ventes non fiscal | Oui. Ne signe rien. | `RAPPORTS_VENTES_FR.md` |

Le terminal de test porte l'état de l'export nocturne (activé ou non, détenteurs des clés de déchiffrement, exploitant du récepteur). Valeur : À effectuer (EV-20 : état de l'export nocturne sur la machine de test).

Deux fonctions n'existent que sur Android et n'interviennent pas dans le calcul fiscal : l'export des données pour le SPF (`SECURITE_BD_FR.md` §8) et la mise à jour lancée par le gérant (`GARANTIE_MAJ_LANGUES_FR.md` §2).

## 8. Machine de test

- **Terminal.** La machine de test Android est le profil émulé sunmi_d3_pro_gms_api33 (Sunmi D3 Pro, 1920 x 1080 en paysage, Android 13 arm64), sauf si un D3 Pro réel est disponible pour le rendez-vous d'admission. Les faits matériels d'un D3 Pro réel (format du numéro de série, stockage, fente de carte, démarrage vérifié, chiffrement, comportement de l'imprimante Epson, durées) sont : À effectuer (EV-15 : faits d'un D3 Pro réel, ou confirmation que la machine de test est le profil émulé).
- **Identifiant de l'appareil.** Valeur imprimée sur les tickets de la machine de test : `EMULATOR37X2X12X0` (profil émulé ; écran « À propos », capture A82, et tickets de `TICKETS_EXEMPLES_FR.md`).
- **FDM.** Le simulateur en ligne de module2, comme pour le produit Linux. Passage en conditions réelles contre ce simulateur : À effectuer (EV-14 : passage réel contre le simulateur module2, avec le terminal POS-2-ANDROID).
- **Imprimante.** Une Epson TM-m30III sur le réseau local (port 9100) ; faute de matériel, l'imprimante simulée du dépôt reçoit les tickets (`SCHEMA_ASSEMBLAGE_FR.md`). Exemples de tickets : imprimés sur l'imprimante simulée, dans `TICKETS_EXEMPLES_FR.md` (impression sur l'Epson réelle : À effectuer (EV-15 : imprimante réelle)).
- **Résultats.** Cas d'utilisation Basic Set 1.8, Advanced Set 1.6 et Reports 1.3 sur l'émulateur : À effectuer (EV-06 : résultats UC sur l'émulateur Android). Déroulement complet sur l'appareil (provisionnement, vente, remboursement, Z, export SPF, mise à jour, verrouillage) : À effectuer (EV-10 : déroulement sur le profil D3 Pro).
- **Totaux de tests à la version déposée.** À effectuer (EV-17 : comptes finaux des tests à la version déposée).
