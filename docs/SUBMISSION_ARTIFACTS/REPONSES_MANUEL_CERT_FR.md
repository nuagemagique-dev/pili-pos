# Réponses au manuel de certification SCE 2.0 - moyun-pos v1.0.0

**Date :** 2026-05-14
**Distributeur :** Bruno Moyse / nuagemagique.dev (Bruxelles, BE)
**Produit :** moyun-pos v1.0.0

Ce document parcourt linéairement le manuel
`20250307_FR - Manuel de certification_POS_V11.pdf` et fournit, dans
l'ordre des sections, la réponse à chaque case à cocher (☐) et à
chaque champ libre. Les renvois aux annexes du dossier sont indiqués
en italique.

Convention :
- ☑ : case cochée (réponse positive ou élément présent).
- ☐ : case non cochée (réponse négative ou non-applicable).
- *Voir : annexe.md* : référence vers un document annexe du dossier.

---

# §I - DOCUMENTATION REQUISE

## §I.A Formulaire de demande

| Item | Réponse |
|---|---|
| ☑ Formulaire complété | Formulaire `20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM.pdf` complété avec les valeurs déclarées dans ce document. |
| ☑ Formulaire signé | Signature électronique qualifiée apposée au moment de la soumission par Bruno Moyse (distributeur). |

## §I.B Résultats de test

| Item | Réponse |
|---|---|
| ☑ Document PDF complété | Voir `RESULTATS_TESTS_UC_FR.pdf` (annexe, en cours de finalisation - bloc C de l'audit). Couvre les UC du Basic Set + Reports avec JSON request, JSON response et ticket pour chaque UC, **à l'exclusion de M150 (event I, hors périmètre v1.0.0, cf. §III.F)**. Nombre exact d'UC couverts par les preuves d'annexe : 17 UC dans le périmètre v1.0.0 (`docs/SUBMISSION_ARTIFACTS/evidence/uc/`, M150 exclu). |

## §I.C Manuels du POS

| Item | Réponse |
|---|---|
| ☑ Manuel d'utilisateur | *Voir : MANUEL_UTILISATEUR_FR.md* |
| ☑ Manuel de configuration/installation | *Voir : MANUEL_INSTALLATION_FR.md* |
| ☑ Manuel de programmation | *Voir : MANUEL_PROGRAMMATION_FR.md* |
| ☐ Manuel de service | Non applicable. Producteur = distributeur = développeur unique. Le support de service est assuré directement par le producteur (voir §I.F profil de l'entreprise). |
| ☑ Schéma d'assemblage | *Voir : SCHEMA_ASSEMBLAGE_FR.pdf* (annexe). |

## §I.D Hash SHA-1 du logiciel

| Item | Réponse |
|---|---|
| ☑ Le SHA-1 est envoyé en pièce jointe | Le SHA-1 du binaire de certification est produit par le script `scripts/build_and_hash.sh`. Valeur exacte communiquée au moment de la soumission (le hash est lié à la build finale signée). |

## §I.E Documents d'identification

### Producteur (= distributeur = développeur)

| Champ | Valeur |
|---|---|
| Nom de l'entité juridique | Bruno Moyse (personne physique indépendante) |
| Nom commercial | nuagemagique.dev |
| Structure juridique | Personne physique (entreprise individuelle, régime franchise TVA art. 56 §2 CTVA) |
| Adresse | Rue Joseph Claes 4/1, 1060 Saint-Gilles, Belgique |
| Numéro d'identification à la TVA | BE 0783.544.917 (n° d'entreprise BCE) |
| Site web | https://nuagemagique.dev |
| Représentants autorisés | Bruno Moyse |
| Représentant - numéro de téléphone | +32 494 46 50 26 |
| Représentant - adresse électronique | cloud@nuagemagique.dev |
| Contact technique | Bruno Moyse |
| Contact technique - numéro de téléphone | +32 494 46 50 26 |
| Contact technique - adresse électronique | cloud@nuagemagique.dev |

### Importateurs et distributeurs

Le producteur est également importateur et distributeur (configuration mono-acteur). Pour les déploiements clients futurs, *voir : LISTE_DISTRIBUTEURS_FR.md*.

## §I.F Profil de l'entreprise

*Voir : PROFIL_ENTREPRISE_FR.md* - couvre :
- Contrôle interne (enregistrement, plugins matériels)
- Organisation interne (1 développeur, internalisation 100%)
- Distributeurs (dispositions contractuelles à venir pour futurs clients)
- Support technique (canal email cloud@nuagemagique.dev + accès distant)
- Analyse de risques (incl. RGPD - base légale article 6.1.c du RGPD pour l'INSZ employé)
- Contrôle externe (aucun autre certificat à ce jour, pas d'assurance produit)

---

# §II - DESCRIPTION DU POS

## §II.A Identification du logiciel

| Champ | Valeur |
|---|---|
| Nom du POS | moyun-pos |
| Modèle du POS | moyun-pos v1.0.0 (Tauri v2 desktop application) |
| N° de version du logiciel | 1.0.0 |
| Langage(s) de programmation | Rust (backend, ~85% du code) + TypeScript/Vue 3 (frontend, ~15%) |

## §II.B Déclaration d'accès à la base de données

| Item | Réponse |
|---|---|
| ☑ Les utilisateurs n'ont accès qu'en lecture seule | La table `fiscal_events` (archive fiscale) est append-only au niveau applicatif : aucune commande Tauri n'émet de DELETE ou UPDATE sur cette table (vérifié par grep statique et test d'intégration). Pour les autres tables, les rôles utilisateur (Admin/Cashier/Waiter) gouvernent les opérations métier autorisées via la couche de commandes. Aucun utilisateur du POS n'a d'accès SQL direct à la base. *Voir : SECURITE_BD_FR.md et DECLARATION_FONCTIONS_INTERDITES_FR.md*. |

## §II.C Caractéristiques de la base de données

| Champ | Valeur |
|---|---|
| La base de données est | ☑ Locale (SQLite embarqué) |
| La base de données et le POS sont tous les deux basés sur le cloud | ☐ Non |
| Identification du DBMS | SQLite |
| N° de version de la base de données | SQLite 3.45+ (version embarquée par le binaire Tauri/Rust crate `rusqlite`) |
| Nom du logiciel et version | moyun-pos v1.0.0 (le DBMS est embarqué dans le binaire applicatif) |
| Chemin d'accès | `/var/lib/moyun-pos/pos.db` en production (`~/.local/share/moyun-pos/pos.db` en dev) |

### Aperçu des tables

| Item | Réponse |
|---|---|
| ☑ La liste contient toutes les colonnes demandées | *Voir : SCHEMA_BD_FR.md* - inventaire complet par table avec nom, champ, type, longueur, description. |
| ☑ La description est brève et claire | Idem. |

### Informations supplémentaires SPF Finances

| Item | Réponse |
|---|---|
| ☑ Description du format de la base de données | *Voir : SCHEMA_BD_FR.md* |
| ☑ Sécurité de la base de données | *Voir : SECURITE_BD_FR.md* (WAL, FK constraints, append-only enforcement, durcissement systemd, scellement image, ECDSA, vérification SHA-1) |
| ☑ Données utilisées et créées par le POS | *Voir : FEUILLE_ACQUISITION_DONNEES_FR.md* |
| ☑ Aperçu des tables contenant les données des events | Table `fiscal_events` documentée dans `SCHEMA_BD_FR.md`. Table append-only avec 1 ligne par événement N/P/T/C/I/F/S/R. |
| ☑ Disposition des tables de données | *Voir : SCHEMA_BD_FR.md* (organisation par domaine : menu, orders, payments, fiscal, users, reports) |
| ☑ Relations entre les tables | *Voir : SCHEMA_BD_FR.md et SCHEMA_ER_FR.pdf* (diagramme ER en annexe) |

### Captation de la base de données

| Item | Réponse |
|---|---|
| ☑ Les agents SPF peuvent accéder/copier les données pertinentes | Procédure documentée dans `FEUILLE_ACQUISITION_DONNEES_FR.md`. Le fichier SQLite `pos.db` est copiable directement (lecture seule pour l'auditeur). |
| ☑ URL, nom d'utilisateur et mot de passe | Compte SPF pré-créé sur la machine de test au rendez-vous d'admission : login `spf`, mot de passe communiqué sur place (mot de passe simple acceptable selon Manuel d'intro §V). Accès à la DB via la **console de maintenance locale** de l'appareil + `sqlite3 /var/lib/moyun-pos/pos.db`. Il n'y a **pas de serveur SSH** sur le POS (appliance scellée, cf. §II.E) ; aucun accès distant à la base n'est possible. |
| ☐ Les données exportées doivent être converties | Non. SQLite est lisible directement par tout outil standard (DB Browser for SQLite, sqlite3 CLI, etc.). Aucune conversion nécessaire. |
| ☐ Les scripts/programmes de conversion sont fournis | Non applicable (cf. ligne précédente). |

### Sécurité de la base de données

| Item | Réponse |
|---|---|
| ☑ Audit trail possible (timestamps, utilisateurs, Primary Key, ...) | Chaque ligne `fiscal_events` porte `created_at` (timestamp UTC), `user_insz` (identité opérateur), `id` (PK auto-incrément monotone), `sequence_number` (compteur fiscal séquentiel), `pos_id` et `terminal_id`. Toute autre table porte `created_at`. *Voir : SCHEMA_BD_FR.md*. |
| Mesures contre la manipulation frauduleuse | *Voir : SECURITE_BD_FR.md* §1 "Append-only enforcement" + `DECLARATION_FONCTIONS_INTERDITES_FR.md`. Résumé : pas de DELETE/UPDATE applicatif sur `fiscal_events` (vérifié par grep), durcissement systemd (NoNewPrivileges, ProtectSystem=strict, ReadWritePaths uniquement sur `/var/lib/moyun-pos`), checksum SHA-1 du binaire vérifiable, build reproductible (Cargo.lock + rust-toolchain.toml épinglés), pas de port d'administration distante côté POS, FDM = seul canal sortant. |

## §II.D Garantie du produit

**Garantie que machine-test et SCE installé chez le client sont identiques :**

L'identité bit-à-bit entre la build cert soumise au SPF Finances et la build déployée chez chaque client est garantie par :

1. **Build reproductible.** `Cargo.lock` est commité dans le dépôt. `rust-toolchain.toml` épingle la version exacte du compilateur (Rust 1.96.0). La commande de build est documentée et figée :
   ```
   deploy/build-release.sh --docker
   ```
   (construit dans un conteneur Debian 12 linux/amd64, cf. `deploy/build-release.sh` ; équivalent natif : `cargo build --release --no-default-features --features cert` sur une machine Debian 12 x86_64.)
2. **Empreinte SHA-1 du binaire.** Le script `scripts/build_and_hash.sh` produit le SHA-1 du binaire à chaque build et le consigne. Le SHA-1 déclaré au formulaire §I.D est vérifiable sur toute installation cliente via :
   ```
   sha1sum /opt/moyun-pos/moyun-pos
   ```
3. **Image kiosk scellée.** Le déploiement client se fait via une image disque produite à partir du tag git de la build cert. Aucune modification de la build n'est possible une fois l'image flashée (pas de SSH, pas de gestionnaire de paquets actif, système de fichiers root en lecture seule sauf `/var/lib/moyun-pos`). *Voir : SECURITE_BD_FR.md §2 "image scellée"*.
4. **Tag git immutable.** Le commit de la build cert est tagué (`vX.Y.Z`) et signé. Toute MAJ ultérieure passe par un nouveau cycle de certification (procédure §II.E + Article 109 du Manuel).

## §II.E Mises à jour

**Procédure de mise à jour du POS :**

Toute modification du logiciel POS post-certification nécessite une nouvelle demande de certification (mise à jour). Sur la machine de test, le nouveau binaire est remis sur clé USB ou transféré pendant une session RustDesk à la demande, puis installé via `deploy/update.sh <binaire> --sha1 <SHA-1 déclaré>`, qui refuse tout binaire dont le SHA-1 diffère de la valeur déclarée. Aucune mise à jour automatique en arrière-plan n'est implémentée : c'est une décision explicite de scellement. *Voir : GARANTIE_MAJ_LANGUES_FR.md* §2.

**Solution d'accès à distance pour l'agent SPF :**

Aucune solution permanente d'accès à distance n'est installée sur la machine de test livrée à l'Intake (la spec SCE 2.0 demande un appliance scellé, sans SSH ni service de gestion réseau). Cependant, pour faciliter les interventions techniques entre la soumission et le rendez-vous d'admission :

- **Accès local au rendez-vous d'admission.** Lors de l'Intake, l'agent SPF peut accéder à la machine de test localement via la console physique (login `spf` documenté au §II.C). C'est suffisant pour vérifier la version, exécuter les tests, et copier la DB.
- **Accès à distance : RustDesk portable, à la demande.** RustDesk (open source, AppImage autonome, version épinglée) est présent sur la machine de test comme sur les terminaux clients, mais n'est jamais lancé au boot. Une session est ouverte ponctuellement pour une mise à jour de certification ou une intervention technique ; sur demande du SPF Finances, les identifiants d'une session de test sont fournis. *Voir : GARANTIE_MAJ_LANGUES_FR.md* §2.2.

Cette absence de solution d'accès à distance permanente est délibérée : pour un terminal de caisse fiscale, ouvrir un canal d'administration distante augmente la surface d'attaque sans bénéfice opérationnel (le producteur n'a pas accès à la caisse du client en service).

## §II.F Matériel

### Tableau du matériel - machine-test (Intake) + production client

| Type | Délivré machine-test à l'Intake | Production client | Description |
|---|---|---|---|
| **Ordinateur(s)** | | | |
| POS (kiosk) | ☑ | ☑ | Penetek A3216 M82 (Intel J6412, 8 Go RAM, 128 Go SSD, écran tactile PCAP 15.6" 1920×1080, fanless, IP65 façade) |
| Ordinateur fixe | ☐ | ☐ | Le POS est un kiosk dédié |
| Ordinateur portable | ☐ | ☐ | |
| Tablette | ☐ | ☐ | |
| Kiosk | ☑ | ☑ | ☑ Le POS est seulement un kiosk |
| **FDM** | ☑ | ☑ | Checkbox CBX01 (TAIGER BV ; distribution Eutronix, réf. TAFIP0002.00). Pour l'Intake : FDM physique de développement Checkbox. |
| **Multiprise** | ☑ | ☐ | Fourni à l'Intake pour faciliter le branchement physique |
| **Imprimante thermique** | ☑ | ☑ | Epson TM-m30III (80 mm, USB + LAN + RJ11 pour tiroir-caisse) |
| **Connectivité** | | | |
| Router/Hub | ☑ | ☐ | Mini-routeur fourni à l'Intake. En production, le restaurant fournit son propre réseau. |
| Câbles | ☑ | ☐ | Câbles RJ45, USB, alimentation. En production, fournis avec l'install. |
| Clé Wifi | ☐ | ☐ | Connectivité Ethernet privilégiée |
| Moxa | ☐ | ☐ | Non utilisé |
| Raspberry Pi | ☐ | ☐ | Non utilisé |
| **Périphériques d'entrée** | | | |
| Souris | ☐ | ☐ | Interface tactile uniquement |
| Clavier | ☐ | ☐ | Idem (clavier virtuel à l'écran si nécessaire) |
| Scanner de produits (client) | ☐ | ☐ | Non supporté |
| Lecteur de carte | ☐ | ☐ | Paiement carte via terminal TPE externe (non intégré) |
| Clés Dallas | ☐ | ☐ | Auth via INSZ + PIN, pas de support clés Dallas |
| Périphérique supplémentaire | ☐ | ☐ | Aucun |
| **Périphériques de sortie** | | | |
| Écran utilisateur (client-facing) | ☐ | ☐ | Pas en MVP |
| Écran pour la cuisine/le bar | ☐ | ☐ | Sortie cuisine via imprimante dédiée (kitchen printer), pas KDS |
| Balance | ☐ | ☐ | Pas de prix au poids |
| **Livraison** | ☐ | ☐ | Pas de fonctionnalité livraison fiscalisée |
| Distributeur de nourriture | ☐ | ☐ | |
| Robot | ☐ | ☐ | |
| **Commande à distance** | ☐ | ☐ | |
| Boutique en ligne | ☐ | ☐ | Webshop existe mais hors périmètre fiscal (build cert — intégrations tierces désactivées) |
| Commande en ligne | ☐ | ☐ | Idem |
| Scanner de code QR | ☐ | ☐ | |
| **Facturation/paiement** | | | |
| App de commande avec facturation | ☐ | ☐ | |
| Processus via site internet | ☐ | ☐ | |
| Tiroir-caisse connecté | ☑ | ☑ | Tiroir RJ11 raccordé à l'imprimante, ouverture par séquence ESC/POS |
| **Autres** | ☐ | ☐ | |

## §II.G Schéma d'assemblage

| Item | Réponse |
|---|---|
| ☑ Schéma d'assemblage fourni en annexe | *Voir : SCHEMA_ASSEMBLAGE_FR.pdf* (annexe). Représente : POS (1) -- Ethernet --> Routeur (2) -- Ethernet --> FDM (3) ; POS (1) -- Ethernet --> Imprimante (4) -- RJ11 --> Tiroir-caisse (5). |

Remarques point I (matériel) : aucune option exotique. Configuration mono-terminal, mono-FDM, mono-imprimante, mono-tiroir.

---

# §III - FONCTIONNALITÉS DU POS

## §III.A Paramètres d'installation

*Voir : MANUEL_INSTALLATION_FR.md*

| Item | Réponse |
|---|---|
| ☑ La liste est claire et complète | Voir le fichier de configuration documenté `/etc/moyun-pos/pos.json` avec les champs obligatoires (`fdm_url`, `printer_address`, `restaurant_est_no`, `pos_id`, `terminal_id`, etc.). |
| ☑ Les paramètres définis sont indiqués | Idem. |
| ☑ Les paramètres définis peuvent être fixes | Oui. Une fois `pos.json` écrit lors du provisionnement initial, il est protégé en écriture (root uniquement, image système en lecture seule). |
| Logs stockés | ☑ Localement (le POS est un appareil mono-terminal, pas de serveur de logs distant) |
| ☑ Les logs ne peuvent pas être supprimés | Les logs applicatifs sont écrits via systemd-journald + table `fiscal_events` append-only. La table fiscale est intouchable par le code applicatif. journald est configuré en mode persistent. *Voir : SECURITE_BD_FR.md*. |

## §III.B Configuration Internet

| Item | Réponse |
|---|---|
| Le POS nécessite une connexion Internet | ☐ Non (le POS communique uniquement avec le FDM sur le LAN local). Toutefois, le FDM lui-même nécessite Internet pour communiquer avec SPF Finances. |
| La connexion Internet peut se faire par | ☑ Ethernet (recommandé pour fiabilité) - ☐ Wifi (techniquement possible, déconseillé en production) |
| Besoins spécifiques pour la mise en place du réseau | IP statique pour le FDM (réservation DHCP recommandée). Aucun port entrant à ouvrir sur le POS. Le POS ouvre une connexion sortante TCP vers l'IP/port GraphQL du FDM. |

## §III.C Utilisateurs & Mots de Passe

### Tableau utilisateurs

| Rôle | Nom d'utilisateur | Mot de passe | Commentaires |
|---|---|---|---|
| OS login (kiosk Linux) | `spf` (compte agent SPF) | Fourni à l'Intake (mot de passe simple) | Idéalement aucun login OS visible (Cage compositor lance directement moyun-pos en plein écran) |
| POS login - Admin | INSZ : `99012345678` (test) | PIN : `1111` | Voir `PROFILS_UTILISATEURS_FR.md` - rôle "Admin" (= Gérant) |
| POS login - Serveur | INSZ : `99012345679` (test) | PIN : `2222` | Rôle "Waiter" (= Serveur SPF) |
| POS login - Technicien | INSZ : `00000000097` | PIN : `0097` | NISS technicien standard SPF |
| POS login - Kiosk / App | INSZ : `00000000029` | N/A | NISS robot/automate standard SPF (utilisé pour les events système) |
| POS login - Apprenti | INSZ : `99012345680` (test) | PIN : `3333` | Rôle "Cashier" (= Apprenti SPF) avec accès mode formation. Voir `PROFILS_UTILISATEURS_FR.md`. |

### Gestion des utilisateurs et rôles

| Item | Réponse |
|---|---|
| ☑ Description de la gestion des utilisateurs et des rôles | *Voir : PROFILS_UTILISATEURS_FR.md* (mapping Admin/Cashier/Waiter <-> Gérant/Apprenti/Serveur SPF) + `MANUEL_PROGRAMMATION_FR.md` §Users (templates SQL d'insertion). |
| ☑ Description des droits d'accès | *Voir : PROFILS_UTILISATEURS_FR.md* - matrice fonctionnalité × rôle. |

Stockage des données utilisateur : table `users` (`insz` PK 11 chiffres, `name`, `role`, `pin_hash`, `active`). Droits d'accès : lecture sur la liste des utilisateurs réservée à Admin ; ajout/modification réservés à Admin ; suppression interdite (active=0 à la place). Audit trail via timestamps + journal applicatif.

## §III.D Connexion du POS au FDM

| Mode | Réponse |
|---|---|
| Autonome | ☐ |
| ☑ LAN | Le POS et le FDM sont sur le même LAN. Connexion HTTP(S) GraphQL. |
| Basée sur le cloud | ☐ |
| Configurable | ☐ |

| Options | Réponse |
|---|---|
| Load-balancing | ☐ Non (un seul FDM par POS) |
| Si OUI, middleware ? | N/A |
| POS compatible avec d'autres marques FDM | ☑ Oui par conception (protocole GraphQL standard SPF, aucune extension propriétaire). FDM de référence testé : Checkbox CBX01 (TAIGER BV). Autres marques certifiées FDM 2.0 : compatibles par protocole, non testées sur matériel. |
| ☑ Les terminaux ne peuvent pas communiquer avec le FDM à moins d'être identifiés avec posId | Confirmé. Chaque requête sortante du POS porte le champ `posId` 14 caractères dans le payload GraphQL (vérifié dans `crates/pos-fiscal/src/inputs.rs::CommonFields`). |

## §III.E Connexion au FDM

| Item | Réponse |
|---|---|
| ☑ La connexion au POS est expliquée | *Voir : DOCUMENT_PROCESSUS_TECHNIQUE_FR.md* §3.1 Transport et §9 Carte des sources. La connexion utilise `reqwest` (client HTTP Rust) sur l'URL `fdm_url` configurée dans `/etc/moyun-pos/pos.json`. |
| Logiciel intermédiaire externe | ☑ Aucun (le POS parle GraphQL directement au FDM, pas de proxy intermédiaire) |

## §III.F FDM Non-Opérationnel

| Item | Réponse |
|---|---|
| ☑ La déconnexion du FDM est expliquée | *Voir : STRATEGIE_COMPTEURS_FR.md* §4 "Blocage opérationnel sur déconnexion" et *DOCUMENT_PROCESSUS_TECHNIQUE_FR.md* §5. Résumé : (a) le POS détecte l'erreur **immédiatement** au prochain appel `FdmClient::status` ou `sign_*` (pas seulement quand une transaction échoue) ; (b) une modale UI bloquante s'affiche, le checkout est refusé tant que le FDM n'est pas opérationnel ; (c) si une transaction a été envoyée mais aucune réponse n'a été reçue, la transaction SQLite locale est rollback : aucun compteur n'avance, aucune ligne `fiscal_events` n'est écrite, l'opérateur peut retenter avec idempotence (cf. STRATEGIE_COMPTEURS_FR.md §3 et §5 fix B3). Mécanisme applicatif : `commands/mod.rs::commit_signed_event` est le **seul** point d'écriture dans `fiscal_events` - il avance le compteur et archive l'événement dans une seule transaction SQLite, et n'est appelé qu'après réception d'une signature FDM valide (`SignResult`). Un échec de signature (FDM injoignable, refusé, timeout) retourne une erreur à l'appelant sans jamais atteindre `commit_signed_event` : pas d'avance de compteur, pas d'archive non signée. |

## §III.A (suite) Events possibles

| Event | Disponible ? | Imprimable papier ? | Imprimable digitalement ? | Copie possible (event C) ? |
|---|---|---|---|---|
| N - Sales | Obligatoire (☑) | Obligatoire (☑) | Obligatoire (☐ pas en MVP - voir §III.H) | ☑ (via M160 signCopy) |
| R - Report | Obligatoire (☑) | ☑ | ☐ (pas en MVP) | ☐ (ne s'applique pas aux R) |
| S - Social | Obligatoire (☑) | N/A | N/A | N/A |
| P - Pro Forma | ☑ | ☑ | ☐ (pas en MVP) | ☑ (via M161 signCopy_Order) |
| T - Mode Training | ☑ | ☑ (avec watermark TRAINING) | ☐ | ☑ (via M172 signCopy_Training) |
| I - Invoice | ☐ non disponible en v1.0.0 (mise à jour certifiée prévue v1.1) | ☐ | ☐ | ☐ |
| C - Copy | ☑ (M160/M161/M172) | ☑ | ☐ | Interdit (par la spec) |
| F - Financial | ☑ (M130 money_in_out, M131 drawer_open) | ☐ (events F sont non-imprimés en standard) | ☐ | ☐ |

## §III.B Numérotation d'events

| Item | Réponse |
|---|---|
| PosFiscalTicketNo | ☑ Par type d'event - *Voir : STRATEGIE_COMPTEURS_FR.md* §1 "stratégie #1 de la spec POS v2.2 §385-388". |
| Numérotation des tickets | ☑ Par type d'event (mêmes compteurs `fiscal_counters` que PosFiscalTicketNo) |

## §III.C Fonctionnalités interdites

| Item | Réponse |
|---|---|
| ☑ Le producteur déclare que de telles fonctionnalités (suppression, modification, ajout d'enregistrements déjà enregistrés) ne sont pas disponibles | Déclaration signée *DECLARATION_FONCTIONS_INTERDITES_FR.md* couvrant : pas de DELETE/UPDATE sur fiscal_events, pas d'antidate, pas de manipulation de compteur, pas de bypass FDM, pas d'effacement de logs, pas de réimpression silencieuse, pas de tickets sans QR, pas de multi-user non-traçable, pas de communication directe avec SPF Finances. |

## §III.D Fonctions obligatoires

**Enregistrement zéro :**

Un enregistrement sans ligne de produit, ou avec un total de vente nul ou négatif, est **techniquement impossible** à produire avec `moyun-pos` v1.0.0 : la garde n'est pas seulement une omission de l'UI, elle est appliquée côté serveur (Rust), dans la couche `src-tauri/src/commands`, avant tout appel au FDM :

- `commands/checkout.rs` : rejette la commande si `lines.is_empty()` (aucune ligne de produit) et rejette explicitement tout total non strictement positif - message `"Order total must be greater than zero"`.
- `commands/tables.rs` (signature d'un Pro Forma / d'une commande de table) : rejette de la même façon `lines.is_empty()`.
- `commands/refund.rs` : rejette le remboursement si `items.is_empty()` (aucune ligne à rembourser).

Ces trois gardes s'exécutent avant toute construction de payload FDM ; aucun `SaleInput`/`OrderInput` à zéro ligne ou à total nul/négatif n'est donc jamais soumis au FDM.

**Déclaration formelle :** un enregistrement sans ligne de produit, ou avec un total de vente nul ou négatif, **n'est jamais possible** avec `moyun-pos` v1.0.0, ni via l'UI opérateur, ni via la couche applicative (garde serveur-side dans `checkout.rs`, `tables.rs` et `refund.rs`, cf. ci-dessus).

**Journal de commande (`order_entries`) :** chaque ajout, correction ou
remboursement de ligne sur une commande ouverte est journalisé dans la
table append-only `order_entries` (`crates/pos-db`, triggers SQL qui
bloquent tout `DELETE`/`UPDATE` des champs journalisés), avec un
`kind` parmi `entry` / `correction` / `refund` - conforme à
l'exigence POS v2.2 §2 "toutes les données d'entrée, par ordre
chronologique, y compris les corrections". `order_lines` reste la vue
consolidée affichée/imprimée. Les entrées non encore transmises sont
envoyées au FDM comme événement **P** (`sign_table_order`,
"vente interrompue") lorsque l'opérateur quitte une table avec une
commande non payée, ou dans son intégralité dans l'événement **N**
final à l'encaissement ; `sent_event_id` marque l'entrée comme
transmise, en excluant tout double-comptage. Les lignes à quantité
négative portent `negQuantityReason` (`crates/pos-fiscal`) :
`CORRECTION` dans les événements P (commande / pré-addition -
réduction avant encaissement) et `CORRECTION` ou `REFUND` dans
l'événement N final selon qu'une vente d'origine est référencée ou
non (`event.rs::reason`).

**Bouton "Addition" (pré-addition, événement P) :** `sign_pre_bill`
imprime la facture consolidée au format PRO FORMA ("addition
provisoire", POS v2.2 §1.4). Comme une pré-addition ne peut ni
ajouter ni modifier de lignes, la commande enregistre d'abord les
entrées de journal en attente via `sign_table_order` (même flux que
ci-dessus), puis signe et imprime la pré-addition à partir des lignes
consolidées.

## §III.E Enregistrement de la présence

**Description de l'implémentation :**

L'enregistrement du temps de présence est implémenté via l'event **S** (Work In / Work Out) du SCE 2.0. À la connexion d'un opérateur sur le POS, après saisie INSZ + PIN, la commande Tauri `commands::social::work_in` est invoquée. Elle :

1. Construit un `WorkInput` avec `pos_id`, `pos_date_time`, `terminal_id`, `employee_insz`, `device_id`.
2. Envoie un `signWorkIn` au FDM.
3. Sur succès, archive l'event S dans `fiscal_events` et bascule `users.social_status='present'`.

Le même mécanisme côté `work_out` à la déconnexion.

**Enregistrement séparé pour les utilisateurs qui n'utilisent pas le POS :**

L'écran de gestion des utilisateurs (rôle Admin uniquement) permet à un manager de pointer des employés qui ne touchent pas le POS au quotidien (cuisine, salle). Ces pointages génèrent quand même des events S avec l'INSZ de l'employé concerné.

| Item | Réponse |
|---|---|
| ☑ Description de l'utilisation | *Voir : MANUEL_UTILISATEUR_FR.md* §"Pointage entrée / sortie" |
| ☑ L'enregistrement du temps de présence est lié aux boutons "Work In / Work Out" | Oui |

## §III.F Fonctionnalités avancées

| Fonction | Utilisée | UseCase | Statut |
|---|---|---|---|
| Normal (N) | Obligatoire (☑) | M110 | OK |
| Remboursement total | ☑ | M111 | OK *Voir : FLUX_REMBOURSEMENT_FR.md* §1 |
| Remboursement partiel | ☑ (déclaré **M112B** - ticket séparé) | M112 | OK *Voir : FLUX_REMBOURSEMENT_FR.md* §1 |
| Report (R) | Obligatoire (☑) | Reports | OK (M180-M184) |
| Social (S) | Obligatoire (☑) | M140/141 | OK |
| Pro Forma (ticket non-TVA) | ☑ | M121 | OK |
| Commande en attente | ☑ | - | OK (gestion d'ordre ouvert sur table) |
| Prebill Pro Forma | ☑ | M123 | OK |
| Tous les events P sont imprimables | ☑ Format papier | - | OK |
| Gestion des cost centers (attribution / division / fusion) | ☐ | M122 / UC240-243 / UC250-252 | Hors périmètre MVP (Advanced Set). Fixtures parsent mais UI non implémentée. |
| Attribution | ☐ | - | Idem |
| Division | ☐ | UC240/241/243 | Idem |
| Fusion | ☐ | UC250/251/252 | Idem |
| Gestion d'addition (division table / addition / parts égales) | ☐ | - | Idem (Advanced Set) |
| Division de table | ☐ | - | Idem |
| Division d'addition (parts égales) | ☐ | - | Idem |
| Division d'addition (montants égaux) | ☐ | - | Idem |
| Invoice (I) | ☐ | M150 | Non disponible en v1.0.0. Le code (`crates/pos-peppol`) existe mais le feature flag `peppol` n'est pas activé dans la build cert (`cert = ["custom-protocol"]`, voir `src-tauri/Cargo.toml`). Mise à jour certifiée prévue v1.1 *Voir : DECISION_PEPPOL_AP_FR.md* |
| Note de crédit | ☐ | - | Reporté avec Peppol / event I (cf. *FLUX_REMBOURSEMENT_FR.md* §3) |
| Copy (C) | ☑ | M160 | OK |
| Training (T) | ☑ | M171 | OK |
| Gestion des cash flows | ☑ | M130 | OK |
| Gestion du tiroir-caisse | ☑ | M131 | OK |
| **Moyens de paiement** | | | |
| Cash | ☑ | - | OK (arrondi belge implémenté) |
| Carte de débit | ☑ | - | OK |
| Carte de crédit | ☑ | - | OK (même flux que débit) |
| Crédit client | ☐ | - | Pas de système clients (B2C) |
| Facturation à l'hôtel | ☐ | - | Non supporté (POS destiné à la restauration) |
| Chèque repas | ☐ | - | Non supporté en MVP |
| Changement de paiement (M132) | ☐ | M132 | Reporté (cf. *FLUX_REMBOURSEMENT_FR.md* §2) |
| Vouchers à but multiple | ☐ | UC260/261 | Hors MVP |
| Vouchers à but unique | ☐ | UC270/271 | Hors MVP |
| Devise étrangère | ☐ | UC296 | Déclaré : EUR uniquement |
| Arrondissement des paiements cash | ☑ | Roundings | Obligatoire pour le cash, implémenté (arrondi belge à 5 centimes) |
| Arrondissement carte EFT | ☐ | Roundings | Optionnel - pas d'arrondi sur paiements EFT |
| Gestion des livraisons | ☐ | - | Non supporté (sur place + emporter uniquement) |
| Réductions | ☐ | - | Non supporté en MVP (les modificateurs +/- sur prix oui, les réductions globales non) |
| Produits composés | ☐ | UC280 | Fixtures parsent, UI non implémentée (Advanced Set) |
| Modification de prix ligne | ☐ | UC220 | Hors MVP |
| Modification de prix event | ☐ | UC223 | Hors MVP |
| Modification de prix multiple | ☐ | UC224 | Hors MVP |
| Corrections (modif quantité) | ☐ | UC230 | Hors MVP (la suppression de ligne avant checkout existe, la correction post-checkout non) |
| Article offert/gratuit (100% réduction) | ☐ | - | Hors MVP |
| Happy Hour programmé | ☐ | - | Hors MVP |
| Gestion des clients | ☐ | - | B2C, pas de base clients |
| Réduction membre | ☐ | - | Idem |
| Mensualités | ☐ | - | Idem |
| Prix libre / "prix du jour" | ☑ | - | Implémenté : colonne `menu_items.open_price` ; le produit `22010` ("Plat du jour") du seed de certification est marqué `open_price = 1`, le prix est saisi par l'opérateur à l'ajout au lieu du prix catalogue. |
| Prix au poids | ☐ | - | Pas de balance |
| Ticket de service (bar / cuisine) | ☑ | - | Imprimante cuisine dédiée, ticket non-fiscal sans prix |
| Autres | ☐ | - | - |

### Remarque importante - facturation B2B Peppol

À partir du 01.01.2026, toutes les factures B2B doivent être émises via le réseau Peppol (BIS Billing 3.0). Cette obligation est **hors périmètre de la build cert v1.0.0** de `moyun-pos` :

- **Sérialiseur UBL** et trait `AccessPoint` existent dans le workspace (crate `pos-peppol`, deux implémentations : `LoggingAccessPoint` no-op et `StorecoveAccessPoint` squelette de production) et sont couverts par des tests unitaires, mais ce code n'est **pas compilé dans la build cert** : `cargo build --release --no-default-features --features cert` n'active ni `peppol` ni `hubrise` (`src-tauri/Cargo.toml` : `cert = ["custom-protocol"]`). L'événement fiscal **I (Invoice)** n'est donc pas disponible en v1.0.0.
- **Table d'audit** `peppol_deliveries` existe dans le schéma pour une activation future, mais n'est alimentée par aucun flux en v1.0.0.
- **Mise à jour certifiée prévue v1.1** : câblage de l'Access Point (Storecove pressenti - voir *DECISION_PEPPOL_AP_FR.md*) et activation du feature flag `peppol`, suivis d'une nouvelle demande de certification (procédure §II.E).

**En attendant v1.1**, les factures B2B du restaurant (cas exceptionnels - client entreprise, établissement voisin) sont émises **en dehors de `moyun-pos`**, via l'outil de comptabilité/facturation du restaurant. `moyun-pos` reste la seule source pour les events N/P/T/C/F/S/R fiscalisés ; aucune facture B2B n'est émise ni imprimée par le POS en v1.0.0.

## §III.G CostCenterChange (Gestion de Table)

| Item | Réponse |
|---|---|
| Le POS permet le changement de cost center vers ROOM (chambre) | ☐ Non (POS destiné à la restauration, pas d'hôtellerie) |
| Le POS permet de recalculer le changement de prix en cas de changement de cost center | ☐ Non (changement de cost center pas implémenté en MVP) |
| Les liens dans la base de données sont décrits | N/A (fonctionnalité non implémentée) |

## §III.H Emission des tickets numériques

L'émission de tickets sous forme numérique est implémentée via le
**QR Code Type 2 imprimé sur chaque ticket papier**, contenant
l'URL de vérification publique fournie par le FDM SPF Finances.

- **Émission numérique :** le QR Code encode l'URL `verificationUrl`
  (38 caractères) renvoyée par le FDM dans la réponse de signSale.
  Le client scanne ce QR avec n'importe quel smartphone et accède
  immédiatement à la page de vérification SPF Finances avec le
  détail complet du ticket.
- **Garantie de remise :** le QR Code est imprimé sur chaque ticket
  fiscal (N, P, C, T, I) - imposé par le code de `pos-print`,
  vérifié par tests unitaires. La remise physique du ticket papier
  au client emporte la livraison numérique.
- **Conservation :** l'URL reste valide pendant les 10 ans de
  conservation légale (Article 60 §4 Code TVA). Le client peut
  scanner à tout moment.

*Voir : TICKET_NUMERIQUE_FR.md* pour le détail de l'implémentation
et la feuille de route post-cert (aperçu QR à l'écran, email opt-in,
intégration Apple/Google Wallet).

## §III.I Terminaux et appareils

| Item | Réponse |
|---|---|
| Le POS permet-il de travailler avec plusieurs périphériques sur un terminal (roaming terminals) ? | ☐ Non (configuration mono-terminal, mono-imprimante, mono-tiroir) |

## §III.J Périodes

| Item | Réponse |
|---|---|
| Le POS permet-il de travailler à la fois avec des périodes d'ouverture (bookingDate) et des créneaux horaires (bookingPeriod) ? | ☑ Oui |
| Configuration | `bookingDate` (date de service fiscale) avance automatiquement au signReportTurnoverZ (clôture journalière). `bookingPeriodId` est **persisté** en base (table `booking_periods`) - une période court du premier événement suivant un Z jusqu'au Z suivant, et porte la date d'ouverture (fuseau Europe/Brussels). Un seul redémarrage du POS un jour donné ne coupe donc jamais la journée en deux périodes : au démarrage, le POS reprend la période ouverte si elle existe, sinon en ouvre une nouvelle. Des triggers SQL interdisent la réouverture et la suppression d'une période close ; un index unique garantit qu'une seule période est ouverte à la fois. Une clôture Z **en mode formation** ne clôture pas la période (ni ne consomme de numéro de Z, cf. §III.L). Aucune configuration manuelle requise par l'opérateur. *Voir : DOCUMENT_PROCESSUS_TECHNIQUE_FR.md* §4 et *FLUX_DONNEES_FR.md* §5. |

## §III.K Tiroirs-Caisse

| Item | Réponse |
|---|---|
| Est-ce que cette fonctionnalité est disponible sur votre POS ? | ☑ Oui |
| **Si oui uniquement :** | |
| S'agit-il d'un tiroir-caisse / portefeuille physique ? | ☑ Oui (tiroir RJ11 raccordé à l'imprimante) |
| Est-il possible d'avoir plusieurs tiroirs par POS ou par utilisateur ? | ☐ Non (1 seul tiroir physique par POS) |
| S'agit-il d'un tiroir-caisse virtuel ? | ☐ Non |
| Aucun (tiroir-caisse par défaut "hardcodé" pour les mutations GraphQL) | ☐ Non |
| Implémentation | Le tiroir est ouvert par séquence ESC/POS `0x1B 0x70 0x00 0x19 0xFA` envoyée à l'imprimante au moment d'un paiement cash (close de vente) ou via la commande manuelle `commands::financial::open_drawer_fiscal` (event F-WAR émis et archivé). Les rapports utilisateur (M183) ventilent les paiements par mode (cash / card / autre). |

## §III.L Rapports

| Item | Réponse |
|---|---|
| Rapports Z : Un seul rapport par BookingDate | ☑ Oui (la clôture Z incrémente `bookingDate` ; deux Z dans la même journée fiscale sont refusés au niveau applicatif) |
| Rapports Financier Z et Utilisateurs Z : sont-ils liés et émis ensemble ? | ☑ Oui (M184 - mutation GraphQL aliasant `signReportTurnoverZ` + `signReportUserZ` dans une seule opération atomique - cf. *COUVERTURE_RAPPORTS_FR.md* §M184) |
| Rapports Financier Z et Utilisateurs Z : sont-ils émis automatiquement ? | ☐ Non (déclenché manuellement par l'opérateur "Fin de journée" dans l'UI). Note : préférable selon le manuel, mais le déclenchement manuel est conscient (le gérant valide la clôture après recompte de caisse). |
| Le POS dispose-t-il d'autres rapports que les rapports obligatoires X et Z ? | ☐ Non. Seuls X (turnover + user) et Z (turnover + user) sont implémentés. |
| Calcul des totaux (turnover, users, posDevices, fdmDevices) | Calculés par `crates/pos-fiscal/src/report_totals.rs` **exclusivement à partir des événements fiscaux archivés et signés** de la période d'ouverture en cours (`booking_periods`, cf. §III.J), jamais des tables `orders`/`order_lines`/`payments` mutables - conforme POS v2.2 §2.2.2. Détail : `departments`, `payments` et `negQuantities` sont calculés à partir des événements N uniquement ; `vats` est la somme du `vatCalc` renvoyé par le FDM pour chaque N (repli sur les taux standard A/B/C si absent) ; `users[].totalAmount` est la somme des `transactionTotal` des N de l'employé, et la plage d'activité (`first`/`lastPosDateTime`) couvre tous ses événements **sauf** les rapports R eux-mêmes. Chaque moyen de paiement a son propre `financials.id` (spec §2.2.2) - `"1"` espèces, `"2"` carte - pour que les rapports les distinguent. Tous les montants sont additionnés en centimes (pas de flottants) pour éviter la perte de centime observée avant ce correctif (3,04 € devenait 3,03 €). |
| Rapport Z en mode formation | ☐ Un Z de formation **ne consomme pas de numéro de Z** et **ne clôture pas** la période d'ouverture (`booking_periods` reste ouverte) - cohérent avec le principe que les événements T ne comptent jamais fiscalement. |
| Rejeu du scénario officiel Reports v1.3 | Les 8 tickets `T1`-`T8` du scénario SPF (`docs/20250916_UC_Reports_1.3/`) sont rejoués deux fois par les tests : purement via `report_totals` (`crates/pos-fiscal/tests/report_totals_fixtures.rs`) et de bout en bout via les commandes réelles (`src-tauri/tests/spf_e2e_report_fixtures.rs`). Les deux correspondent aux rapports publiés M180-M184, avec un écart assumé et documenté dans le test : le rapport publié compte 3 tickets P pour 198 € (T1 exclu), alors que le scénario enregistre 4 événements P (commandes T1/T5/T6 + pré-addition T7) pour 250 € ; la spec définit `ticketCount` comme le nombre d'événements du label sur la `bookingDate` (§2.2.2 turnover), donc moyun-pos rapporte 4 et 250 € - conforme à la spec, différent du chiffre publié dans la fixture. Les paiements carte sont toujours `CARD_DEBIT` (l'opérateur ne distingue pas débit/crédit à l'encaissement). |

## §III.M Intégration des tiers-parties

| Nom | Statut |
|---|---|
| Deliveroo | ☐ Non |
| TakeAway.com | ☐ Non |
| Uber Eats | ☐ Non |
| Autres partenaires | ☐ Non (aucune intégration tierce active dans le build cert) |

### Remarques sur les fonctionnalités du POS

Toutes les fonctionnalités déclarées ☑ sont couvertes par des tests automatisés (unit, intégration, e2e applicatif via Tauri commands). Voir *RAPPORT_TESTS_BASELINE_FR.md* pour le détail des 190 tests passants.

## §III - Lignes de produit prédéfinies

**Statut :** la machine de test est initialisée avec exactement les 25 produits du manuel (Dry Martini productId 10006, Burger of the Chef 22001, Carpaccio Beef 20001, Catch of the day 22005, Dame Blanche 25003, Crème brûlée 25008, etc.), leurs départements, prix et codes TVA, depuis `src-tauri/seed_spf_test.sql` appliqué à la construction de l'image d'admission. *Voir : LIGNES_PRODUITS_FR.md*.

Ce seed sera chargé sur la machine de test à l'Intake. Les UC du Bloc IV (procédure GraphQL) référencent ces productId précis ; le seed SPF garantit que les UC peuvent être exécutés à l'identique.

---

# §IV - PROCÉDURE DE COMMUNICATION GRAPHQL

Le manuel comprend une procédure de test détaillée par mutation (M110, M111, M112, M121, M122, M123, M130, M131, M132, M140/141, M150, M160, M171, M180-184) plus les Advanced UC. Pour chaque mutation, le manuel attend :
1. Un tableau "Champs à vérifier" coché.
2. Une grille TVA vérifiée.
3. La capture JSON request + response.
4. Le ticket imprimé.

**Statut moyun-pos :**

- Les 22 fixtures Basic Set v1.7 + 5 Reports v1.3 + 141 Advanced Set v1.5 sont chargées et parsées par les tests automatisés (*Voir : COUVERTURE_FIXTURES_FR.md, COUVERTURE_RAPPORTS_FR.md, COUVERTURE_ADVANCED_SET_FR.md*).
- 16 UC Basic Set + 4 Reports + 2 no-gap sont couverts par les tests e2e applicatifs (commande Tauri → tx SQLite → archive). *Voir : RAPPORT_TESTS_BASELINE_FR.md*.
- **Document détaillé par UC** avec JSON request/response et ticket : *Voir : RESULTATS_TESTS_UC_FR.pdf* (annexe en cours de finalisation - bloc C de `GAP_AUDIT_FR.md`).

Récap mappings UC <-> commande Tauri <-> test :

| UC | Mutation | Commande | Test |
|---|---|---|---|
| M110 | signSale | `checkout::checkout` | `spf_e2e_basic::m110_*` |
| M111 | signSale refund full | `refund::refund_checkout` | `spf_e2e_basic::m111_*` |
| M112B | signSale refund partial (événement séparé, chaîné) | `refund::refund_checkout` (multi-step) | `spf_e2e_basic::m112b_*` |
| M121 | signOrder | `tables::sign_table_order` | `spf_e2e_basic::m121_*` |
| M123 | signPreBill | `tables::sign_pre_bill` | `spf_e2e_basic::m123_*` |
| M130 | signMoneyInOut | `financial::money_in_out` | `spf_e2e_basic::m130_*` |
| M131 | signDrawerOpen | `financial::open_drawer_fiscal` | `spf_e2e_basic::m131_*` |
| M140 | signWorkIn | `social::work_in` | `spf_e2e_basic::m140_*` |
| M141 | signWorkOut | `social::work_out` | `spf_e2e_basic::m141_*` |
| M160 | signCopy | `copy::copy_ticket` | `spf_e2e_basic::m160_*` |
| M170/171/172 | signSale/Copy training | (mêmes commandes, flag is_training) | `spf_e2e_basic::m170_*` / `m171_*` / `m172_*` |
| M150 | signInvoice | `invoice::sign_invoice_fiscal` | `spf_e2e_basic::m150_*` |
| M161 | signCopy (commande P) | `copy::copy_ticket` | `spf_e2e_basic::m161_*` |
| M180 | signReportTurnoverX | `report::generate_x_report` | `spf_e2e_reports::m180_*` |
| M182 | signReportUserX | `report::generate_user_x_report` | `spf_e2e_reports::m182_*` |
| M184 (Z combiné) | signReportTurnoverZ + signReportUserZ (un document) | `report::generate_z_report` | `spf_e2e_reports::m184_*` |

La ligne M150 ci-dessus décrit la couverture de test (code + e2e réels,
sous les features par défaut) ; elle n'implique pas que l'événement I
soit disponible dans la build cert v1.0.0 soumise - voir §III.F et
*DECISION_PEPPOL_AP_FR.md*.

---

# §V - COMPORTEMENT DU POS AUX RÉPONSES DU FDM

**Statut implémentation :** le composant Vue `ui/src/components/FdmStatus.vue` mappe **21 codes FDM** vers des messages FR (constante `ERROR_MESSAGES`) et les affiche dans une pilule du bandeau (rouge / jaune / vert), plus un repli générique pour les erreurs de connexion (mots-clés `connect`/`timeout`) et un repli générique final pour tout code non listé. Le sondage du status FDM (`FdmClient::status`) tourne en arrière-plan toutes les 30 secondes. Les textes ci-dessous sont recopiés **mot pour mot** depuis le code source (source de vérité) ; toute divergence future entre ce tableau et `FdmStatus.vue` doit être résolue en faveur du code.

**Statut tests/captures :** l'injection d'erreurs runtime est implémentée dans le mock FDM (`crates/pos-fdm-mock`, variables d'environnement `MOCK_FDM_ERROR` / `MOCK_FDM_WARNING`, codes séparés par des virgules). Les **preuves JSON** (requête signSale + réponse d'erreur GraphQL, requête status + réponse avec le message) sont capturées par `scripts/capture_fdm_error_evidence.sh` et archivées sous `evidence/fdm_errors/` pour les 4 erreurs à affichage obligatoire (BUFFER_FULL, FDM_LOCKED, FDM_NOT_OPERATIONAL, UNKNOWN_POS), les 2 avertissements à affichage obligatoire (CLIENT_CERT_EXPIRED, BUFFER_NEAR_FULL) et 2 codes optionnels (UNAUTHORIZED, RTC_SYNC_FAILED). Le payload signSale utilisé est la fixture officielle M110 (Basic Set v1.7). Les **captures d'écran** de l'UI restent à produire (bloc B2, `CAPTURES_ECRAN_RUNBOOK_FR.md`).

Tableau de réponse (JSON fournis ; captures d'écran à compléter - bloc B2) :

| Code | Nature | Affichage requis | Affichage moyun-pos | Capture |
|---|---|---|---|---|
| CLIENT_CERT_NEAR_EXPIRATION | Warning | O | ☑ (mappé « Le certificat de la boîte noire expire bientôt. ») | À fournir |
| CLIENT_CERT_EXPIRED | Warning | M | ☑ (mappé « Le certificat de la boîte noire a expiré. Contactez votre technicien. ») | JSON : `evidence/fdm_errors/CLIENT_CERT_EXPIRED_*.json` ; écran à fournir |
| RTC_SYNC_FAILED | Warning | O | ☑ (mappé « L'horloge de la boîte noire n'a pas pu se synchroniser. ») | JSON : `evidence/fdm_errors/RTC_SYNC_FAILED_*.json` |
| UPDATE_URLS_FAILED | Warning | O | ☑ (mappé « Mise à jour des URLs de la boîte noire échouée. ») | - |
| UPDATE_TRUST_CERT_FAILED | Warning | O | ☑ (mappé « Mise à jour des certificats de confiance échouée. ») | - |
| UPDATE_TASK_LIST_FAILED | Warning | O | ☑ (mappé « Téléchargement des tâches FDM échoué. ») | - |
| TASK_FEEDBACK_FAILED | Warning | O | ☑ (mappé « La boîte noire n'a pas pu signaler le résultat d'une tâche. ») | - |
| NOP_FAILED | Warning | O | ☑ (mappé « Communication intermédiaire avec la boîte noire échouée. ») | - |
| BUFFER_NEAR_FULL | Warning | M | ☑ (mappé « La boîte noire est presque pleine. Programmez une intervention. ») | JSON : `evidence/fdm_errors/BUFFER_NEAR_FULL_*.json` ; écran à fournir |
| SERVER_CERT_RESOLVE_FAILED | Warning | O | ☑ (mappé « Impossible de résoudre le certificat du serveur FDM. ») | - |
| TRANSACTION_UPLOAD_FAILED | Warning | O | ☑ (mappé « La boîte noire échoue à transmettre les transactions au SPF Finances. ») | - |
| INITIALIZATION_FAILED | Warning | O | ☑ (mappé « L'initialisation de la boîte noire a échoué. ») | - |
| RTC_NOT_INITIALIZED | Warning | N (non-affichable) | - | - |
| CORRUPT_RECORD_ENCOUNTERED | Warning | N | - | - |
| UPDATE_PARAMS_FAILED | Warning | N | - | - |
| UPDATE_CLIENT_CERT_FAILED | Warning | N | - | - |
| UPDATE_VAT_RATES_FAILED | Warning | N | - | - |
| UPDATE_POS_ALLOWLIST_FAILED | Warning | N | - | - |
| DUPLICATE_REQUEST | Warning | N | - | - |
| BUFFER_FULL | Erreur | M | ☑ (mappé « La boîte noire est pleine. Contactez votre technicien. ») + modale bloquante au checkout | JSON : `evidence/fdm_errors/BUFFER_FULL_*.json` ; écran à fournir |
| FDM_LOCKED | Erreur | M | ☑ (mappé « La boîte noire est verrouillée. Contactez votre technicien. ») + modale bloquante | JSON : `evidence/fdm_errors/FDM_LOCKED_*.json` ; écran à fournir |
| TOO_MANY_MEMORY_ERRORS | Erreur | N | - | - |
| UNAUTHORIZED | Erreur | O | ☑ (mappé « La caisse n'est pas autorisée à parler à la boîte noire. ») | JSON : `evidence/fdm_errors/UNAUTHORIZED_*.json` ; écran à fournir |
| INVALID_REQUEST | Erreur | O | ☑ (mappé « Erreur de communication avec la boîte noire. Contactez votre technicien. ») | À fournir |
| INTERNAL_ERROR | Erreur | O | ☑ (mappé « Erreur technique de la boîte noire. Contactez votre technicien. ») | À fournir |
| UNDEFINED_ERROR | Erreur | O | ☑ (mappé « Erreur indéfinie de la boîte noire. Contactez votre technicien. ») | À fournir |
| UNDEFINED_OTHER | Erreur | O | ☑ (mappé « Erreur de la boîte noire. Contactez votre technicien. ») | À fournir |
| FDM_NOT_OPERATIONAL | Erreur | M | ☑ (mappé « La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes. ») + modale bloquante | JSON : `evidence/fdm_errors/FDM_NOT_OPERATIONAL_*.json` ; écran à fournir |
| UNKNOWN_POS | Erreur | M | ☑ (mappé « Cette caisse n'est pas reconnue par la boîte noire. ») | JSON : `evidence/fdm_errors/UNKNOWN_POS_*.json` ; écran à fournir |

En complément des 21 codes ci-dessus, `FdmStatus.vue` affiche deux messages génériques non listés dans la spec §V : « Connexion perdue avec la boîte noire. Vérifiez le câble réseau. » (si le message brut contient `connect`/`timeout`) et « Erreur boîte noire. Contactez votre technicien. » (repli final pour tout code FDM non reconnu par le mapping ci-dessus) - ceci garantit qu'aucune erreur FDM ne s'affiche jamais sans message FR compréhensible par l'opérateur.

**☐ Le test a été effectué et le JSON ainsi que les captures d'écran ont été ajoutés** - JSON ajoutés (`evidence/fdm_errors/`, 8 codes × 4 fichiers) ; case à cocher après les captures d'écran du bloc B2.

---

# §VI - TESTING DES TICKETS

## Contenu obligatoire du ticket TVA

Tableau des 18 éléments obligatoires × 6 types de tickets. *Voir : TICKETS_EXEMPLES_FR.md* pour les exemples de tickets et le tableau de conformité coché.

| Élément | N | N Refund | P | C | T | Autres (cuisine) |
|---|---|---|---|---|---|---|
| I. Dénomination complète prestataire | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| II. Identification (nom/adresse/TVA) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| III. Dénomination commerciale | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| IV. N° unité établissement (EST) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| V. N° fabrication POS + version | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VI. Identification terminal | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VII. Identification dispositif (MAC, SN) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| VIII. Identification utilisateur (INSZ) | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| IX. Date et heure | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ |
| X. N° ticket continu | ☑ | ☑ | ☑ | ☑ | ☑ | - |
| XI. Opérations produits (qté, desc, prix, TVA) | ☑ | ☑ | ☑ | ☑ | ☑ | (descriptions seules, pas de prix) |
| XII. Total à payer TVA comprise | ☑ | ☑ | - (pas encore payé) | ☑ | ☑ | - |
| XIII. Base imposable par taux | ☑ | ☑ | - | ☑ | ☑ | - |
| XIV. Montant TVA par taux | ☑ | ☑ | - | ☑ | ☑ | - |
| XV. Modifications prix non-lignes | ☐ (pas de réductions globales en MVP) | ☐ | ☐ | ☐ | ☐ | - |
| XVI. Données FDM (footer, fdmId, dateTime, label, counters, shortSignature) | ☑ | ☑ | ☑ (sans shortSignature) | ☑ (sans shortSignature) | ☑ (sans shortSignature) | - |
| XVII. Lignes paiement (arrondi, tips, remboursements) | ☑ | ☑ (négatif) | - | - | ☑ (FICTIF) | - |
| XVIII. QR Code URL FDM | ☑ | ☑ | ☑ | ☑ | ☑ | - |

**Mentions obligatoires spécifiques :**

- Ticket C (Copy) : **"CECI N'EST PAS UN TICKET DE CAISSE DE TVA VALABLE"** - ☑ imprimé sur tous les tickets C.
- Ticket T (Training) : même mention + bandeau "TRAINING" - ☑.
- Tickets autres (cuisine/bar) : pas de prix, pas de données de contrôle, mention "CECI N'EST PAS UN TICKET DE CAISSE DE TVA VALABLE" - ☑.

## Règles de langue

**Règle appliquée :** conformément au manuel (§VI, p. 99), la langue du ticket est la langue officielle de la région linguistique **où le ticket est émis**, c'est-à-dire celle de l'établissement exploitant le POS. Elle est fixée par installation lors de la mise en service (région de langue française → `fr`, néerlandaise → `nl`, allemande → `de`, Bruxelles-Capitale → `fr` ou `nl`). Détail : *GARANTIE_MAJ_LANGUES_FR.md* §3.2.

**Exemple :** un établissement à 4000 Liège (région de langue française) émet ses tickets en **français**. Le code source supporte aussi le rendu multilingue des libellés produits via la table `translations` (cf. *SCHEMA_BD_FR.md*).

Pour répondre à la spec belge, le POS génère les mentions fiscales du ticket dans la langue configurée au niveau du POS (clé `language` de la configuration, cf. `MANUEL_INSTALLATION_FR.md`), mappée sur une langue nationale via `ReceiptLanguage::from_code` (`crates/pos-fiscal/src/ticket.rs`) : `fr` → FR, `nl` → NL, `de` → DE ; toute langue d'affichage non officielle (zh, en) retombe sur le FR. Les trois jeux de mentions implémentés :

| Mention | FR | NL | DE |
|---|---|---|---|
| Dénomination ticket N | TICKET DE CAISSE TVA | BTW-KASTICKET | MWST.-KASSENZETTEL |
| Remboursement | REMBOURSEMENT DU TICKET DE CAISSE TVA | TERUGBETALING BTW-KASTICKET | RUECKERSTATTUNG MWST.-KASSENZETTEL |
| Mention d'invalidité (bas de ticket non-N) | CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE | DIT IS GEEN GELDIG BTW-KASTICKET | DIES IST KEIN GUELTIGER MWST.-KASSENZETTEL |
| Copie | COPIE | KOPIE | KOPIE |
| Facture | FACTURE | FACTUUR | RECHNUNG |

Les libellés FR proviennent de la description détaillée POS v2.2 et du tableau §VI du manuel ; les libellés NL des circulaires GKS ; les libellés DE sont une traduction de style officiel à confirmer sur la version DE de la spec (translittération ASCII, l'imprimante thermique n'ayant pas de négociation de code page). La langue est un paramètre d'installation (`pos.json::language`) : aucun changement de binaire d'un établissement à l'autre.

**Cette politique de langue par client est documentée dans `MANUEL_INSTALLATION_FR.md` §"Langue du ticket".**

## Contenu obligatoire de la facture imprimée (B2B)

*Voir : DECISION_PEPPOL_AP_FR.md* + *FLUX_REMBOURSEMENT_FR.md* §3. L'émission de factures B2B (event I) est implémentée de bout en bout côté code : `invoice::do_sign_invoice_fiscal` construit l'InvoiceInput (n° de facture séquentiel, TVA client, `fdmRefs` vers les ventes sous-jacentes — forme de la fixture M150), l'envoie au FDM via `signInvoice` et archive l'événement I - exercé en e2e (`spf_e2e_basic::m150_*`) sous les features par défaut (`hubrise` + `peppol`). Mais ce code est **hors périmètre de la build cert v1.0.0** : `cert = ["custom-protocol"]` n'active pas `peppol`, donc `invoice.rs` n'est pas compilé dans le binaire soumis. **Aucune facture B2B n'est émise ni imprimée par le POS au rendez-vous d'admission.** L'UI complète + le câblage AP Peppol sont planifiés pour la mise à jour certifiée v1.1.

Les 11 éléments obligatoires (a-k) sur la facture seront couverts et cochés au dossier de mise à jour v1.1, une fois le feature flag `peppol` activé dans une build cert et le flux UI complet livré.

---

# §VII - ACQUISITION DES DONNÉES

## §VII.A Les données sont conservées localement

*Voir : FEUILLE_ACQUISITION_DONNEES_FR.md* §A "Conservation locale" - procédure complète d'accès, copie, extraction des données du fichier SQLite `pos.db`.

## §VII.B Les données sont conservées dans le cloud

**☑ J'ai pris ceci en considération et j'ai fourni une procédure à l'équipe SCE** (cochée par défaut bien que non applicable : `moyun-pos` est 100% local, aucune donnée fiscale dans le cloud. La déclaration cochée reconnaît qu'on a pris en compte la question).

## Feuille d'acquisition des données (DAS)

*Voir : FEUILLE_ACQUISITION_DONNEES_FR.md* qui couvre l'intégralité des §A-F (Informations générales, Réseaux & Appareils, Logs, Rapports X/Z, Base de données, Version du logiciel).

Recap des réponses clés DAS (extrait, le détail est dans l'annexe) :

| Champ | Valeur |
|---|---|
| Producteur | Bruno Moyse / nuagemagique.dev |
| Point de contact | Bruno Moyse |
| E-mail | cloud@nuagemagique.dev |
| Nom du logiciel | moyun-pos |
| N° de version | 1.0.0 |
| Système d'exploitation | Debian 12 minimal + Cage Wayland compositor |
| Numéro de certificat | (à compléter par le SPF Finances) |
| Date de certification | (à compléter par le SPF Finances) |
| Agent de certification | (à compléter par le SPF Finances) |
| SCE basé sur un système Cloud | ☐ Non |
| Connexion internet requise | ☐ Non (le POS est offline ; seul le FDM nécessite Internet) |
| Connexion POS-FDM | ☑ Ethernet |
| Périphériques supplémentaires | ☐ Non |
| Logs disponibles, lisibles, copiables | ☑ Oui (journald + table `fiscal_events`) |
| Méthode de lecture des logs | `journalctl -u moyun-pos.service` + `sqlite3 /var/lib/moyun-pos/pos.db ".dump fiscal_events"` |
| Logiciel DBMS | SQLite 3.45+ (embarqué) |
| Langage de programmation | Rust + TypeScript/Vue |
| Sécurité base de données | *Voir : SECURITE_BD_FR.md* |
| Login SPF Finances | `spf` (compte machine-test) |
| Mot de passe SPF Finances | Communiqué à l'Intake |
| Version du logiciel | v1.0.0, date `2026-05-14`, SHA-1 produit par `scripts/build_and_hash.sh` |

## §VII.D Rapports X/Z

**Comment les rapports financiers X/Z sont-ils visibles/imprimables ?**

L'opérateur (rôle Admin ou Cashier) accède au menu "Rapports" depuis la barre de navigation principale. Deux boutons distincts "Rapport X (intermédiaire)" et "Rapport Z (clôture)". Le clic déclenche :
1. La commande Tauri `commands::report::generate_x_report` ou `generate_z_report`.
2. L'envoi au FDM via `signReportTurnoverX/Z` (et `signReportUserX/Z` pour Z atomique).
3. Sur succès : impression automatique du ticket de rapport sur l'imprimante thermique.
4. Affichage UI d'un récapitulatif lisible (montants par TVA, par moyen de paiement, par utilisateur).

*Voir : MANUEL_UTILISATEUR_FR.md §"Rapports" et TICKETS_EXEMPLES_FR.md §6 (rapport X) et §7 (rapport Z) pour le rendu papier.*

**Captures d'écran à fournir** : oui (cf. bloc B2 de `GAP_AUDIT_FR.md`).

**Comment les rapports d'utilisateur X/Z sont-ils visibles/imprimables ?**

Émis automatiquement avec le rapport financier Z dans la même opération atomique M184 (un seul appel GraphQL aliase `signReportTurnoverZ` + `signReportUserZ`). Le rapport utilisateur Z imprime un deuxième ticket séquentiellement après le rapport financier. Le rapport utilisateur X (M182) est déclenché séparément depuis le menu Rapports si l'opérateur le demande.

*Voir : COUVERTURE_RAPPORTS_FR.md §M184 pour le détail de l'émission combinée.*

---

# §VIII - SIGNATURE

| Item | Réponse |
|---|---|
| ☑ Demande complète et signée | Sera cochée et signée électroniquement au moment de la soumission par Bruno Moyse via certificat eID belge ou Itsme. |
| ☑ Résultats des tests présents dans le PDF ou dans les annexes | *Voir : RESULTATS_TESTS_UC_FR.pdf* (en cours de finalisation - bloc C) + *RAPPORT_TESTS_BASELINE_FR.md* + *COUVERTURE_FIXTURES_FR.md* + *COUVERTURE_RAPPORTS_FR.md* + *COUVERTURE_ADVANCED_SET_FR.md*. |

**SIGNEZ ICI :** signature électronique qualifiée apposée sur le PDF final du formulaire au moment de l'envoi à `secr.gksce@minfin.fed.be`.

---

# ANNEXES DU DOSSIER

Liste des fichiers à inclure dans le ZIP de soumission :

| # | Fichier | Section couverte |
|---|---|---|
| 1 | REPONSES_MANUEL_CERT_FR.pdf (ce document) | parcours linéaire §I-VIII |
| 2 | LISTE_DISTRIBUTEURS_FR.pdf | §I.E |
| 3 | PROFIL_ENTREPRISE_FR.pdf | §I.F |
| 4 | MANUEL_UTILISATEUR_FR.pdf | §I.C |
| 5 | MANUEL_INSTALLATION_FR.pdf | §I.C |
| 6 | MANUEL_PROGRAMMATION_FR.pdf | §I.C |
| 7 | DECLARATION_FONCTIONS_INTERDITES_FR.pdf | §III.C |
| 8 | PROFILS_UTILISATEURS_FR.pdf | §III.C |
| 9 | DOCUMENT_PROCESSUS_TECHNIQUE_FR.pdf | §II + §III.E |
| 10 | FLUX_DONNEES_FR.pdf | §II.C + §III |
| 11 | SCHEMA_BD_FR.pdf | §II.C |
| 12 | SECURITE_BD_FR.pdf | §II.C |
| 13 | SCHEMA_ER_FR.pdf | §II.C |
| 14 | SCHEMA_ASSEMBLAGE_FR.pdf | §I.C + §II.G |
| 15 | LISTE_FONCTIONNALITES_FR.pdf | §III |
| 16 | LIGNES_PRODUITS_FR.pdf | §III.N |
| 17 | STRATEGIE_COMPTEURS_FR.pdf | §III.B + §III.F |
| 18 | FLUX_REMBOURSEMENT_FR.pdf | §III.F |
| 19 | TICKETS_EXEMPLES_FR.pdf | §VI |
| 20 | DECISION_PEPPOL_AP_FR.pdf | §III.F (encadré B2B) |
| 21 | MAPPING_CONFORMITE_FR.pdf | §IV + §V |
| 22 | FEUILLE_ACQUISITION_DONNEES_FR.pdf | §VII |
| 23 | RAPPORT_TESTS_BASELINE_FR.pdf | §IV + §VIII |
| 24 | COUVERTURE_FIXTURES_FR.pdf | §IV |
| 25 | COUVERTURE_RAPPORTS_FR.pdf | §IV |
| 26 | COUVERTURE_ADVANCED_SET_FR.pdf | §IV |
| 27 | RESULTATS_TESTS_UC_FR.pdf | §IV - PDF de résultats UC (en cours, bloc C) |
| 28 | CAPTURES_ECRAN_FR.pdf | §V + §VII.D (en cours, bloc B2) |
| 29 | JOURNAL_TESTS_PRE_SOUMISSION_FR.pdf | §IV (répétition pré-soumission) |
| 30 | ASSEMBLAGE_FINAL_FR.pdf | runbook opérateur |
| 31 | DOSSIER_SOUMISSION_FR.pdf | checklist maître |
| 32 | CHECKLIST_DEPLOIEMENT_KIOSK_FR.pdf | machine de test |

Plus :
- `20250115_FORMULAIRE DEMANDE_CERTIF_POS-FDM_signe.pdf` - formulaire SPF rempli et signé.
- Hash SHA-1 du binaire `moyun-pos` (produit par `scripts/build_and_hash.sh`).
- Image kiosk ISO (lien de téléchargement séparé via WeTransfer).
