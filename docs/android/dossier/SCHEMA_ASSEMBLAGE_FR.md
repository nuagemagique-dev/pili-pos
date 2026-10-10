<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/SCHEMA_ASSEMBLAGE_FR.md @ 86088e4 -->
# Schéma d'assemblage physique - pili-pos-android

**Distributeur :** Bruno Moyse / nuagemagique.dev (Bruxelles, BE)
**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)
**Référence :** réponse au §I.C "Schéma d'assemblage" et §II.G du manuel de certification.

Le présent document décrit l'assemblage physique du SCE (Système de
Caisse Enregistreuse) `pili-pos-android` dans sa configuration de référence (un établissement
HoReCa). La machine de test livrée à l'Intake SPF Finances est identique, à l'exception du FDM (simulateur en ligne, cf. §7) et, sauf mention contraire, du terminal (profil émulé, `EV-15`). La plateforme du terminal est décrite dans `PLATEFORME_ANDROID_FR.md`.

---

## 1. Vue d'ensemble

```
                    INTERNET
                       |
                       | (LAN routeur restaurant ou
                       |  routeur fourni Intake)
                       |
              +--------+--------+
              |    ROUTEUR      |
              |  IP : 192.168.1.1
              |  DHCP : actif    |
              +---+----------+---+
                  |          |
                  |          |
                  | RJ45     | RJ45
                  |          |
       +----------+--+    +--+---------------+
       |             |    |                  |
       |   FDM       |    |   IMPRIMANTE     |
       |  module2    |    |   Epson TM-m30III|
       |  (MOD01...) |    |                  |
       | IP : 192.168.1.20|  IP : 192.168.1.30|
       | TCP 443 GraphQL  |  TCP 9100 ESC/POS|
       | (HTTPS, mTLS)    |                  |
       +------------------+  +-------+--------+
                                     |
                                     | RJ11
                                     | (séquence ESC/POS
                                     |  0x1B 0x70 0x00 0x19 0xFA)
                                     |
                                +----+-----+
                                |          |
                                | TIROIR-  |
                                | CAISSE   |
                                | physique |
                                |          |
                                +----------+


              +--------+--------+
              |                 |
              |   ROUTEUR       |
              |  IP : 192.168.1.1
              |  (Wi-Fi ou RJ45)|
              +--------+--------+
                       |
                       | Wi-Fi ou RJ45
                       | (liaison filaire : EV-15)
                       |
              +--------+--------+
              |                 |
              |   POS terminal  |
              |  Sunmi D3 Pro   |
              |  (Android 13)   |
              |                 |
              |  IP : 192.168.1.10
              |  HTTPS sortant   |
              |   vers 192.168.1.20:443
              |   (FDM GraphQL,  |
              |   mTLS + jeton)  |
              |                 |
              |  TCP sortant vers
              |   192.168.1.30:9100
              |   (Imprimante)   |
              |                 |
              |  HTTPS sortant (WAN), le cas
              |   echeant : mise a jour lancee
              |   par le Gerant, service d'envoi
              |   du ticket numerique par email
              |   (si configure), HubRise
              |   (si active) et export nocturne
              |   (si configure) - voir §4
              |   "Pare-feu"     |
              |                 |
              | Écran tactile   |
              | intégré         |
              | 1920×1080       |
              | (interface      |
              |  opérateur)     |
              +-----------------+
```

---

## 2. Composants et identification

| # | Composant | Marque/Modèle | Adressage | Connectivité |
|---|---|---|---|---|
| 1 | Terminal POS | Sunmi D3 Pro, variante avec services Google (Android 13, arm64, écran tactile intégré en 1920×1080 paysage). Autres caractéristiques matérielles : À effectuer (EV-15 : fiche mesurée d'un vrai D3 Pro) | IP LAN `192.168.1.10` (réservation DHCP) | Wi-Fi, ou liaison filaire si le modèle en offre une (EV-15), USB et emplacement de carte SD (EV-15) |
| 2 | Routeur LAN | TP-Link Archer C6 (ou équivalent) - fourni avec la machine-test à l'Intake | IP gateway `192.168.1.1`, DHCP `192.168.1.100-200`, baux réservés pour FDM et imprimante | WAN RJ45 (Internet), LAN RJ45 (4×) |
| 3 | FDM | FDM module2 (PRACSYS, certifié SCE 2.0, identifiant `MOD01...`) chez le client. **Machine de test : simulateur en ligne module2, sans boîtier physique** (cf. §7). | IP statique LAN `192.168.1.20` (réservation DHCP). GraphQL en HTTPS sur le port TCP 443 avec le certificat client module2 (mTLS, recommandé par module2 ; HTTP sur le port 80 reste possible), jeton Bearer propre au FDM (`fdm_token_file`, cf. MANUEL_INSTALLATION_FR.md). | RJ45 vers routeur |
| 4 | Imprimante thermique | Epson TM-m30III (80 mm thermique) | IP statique LAN `192.168.1.30` (réservation DHCP). TCP port 9100 ouvert (ESC/POS raw). RJ11 sortant vers tiroir. | RJ45 vers routeur. USB et Wifi disponibles mais non utilisés. RJ11 vers tiroir-caisse. |
| 5 | Tiroir-caisse | Tiroir RJ11 standard (modèle générique compatible Epson) | N/A (passif) | RJ11 depuis l'imprimante. Ouverture par séquence ESC/POS `0x1B 0x70 0x00 0x19 0xFA` envoyée par le POS. |

---

## 3. Ports utilisés

### Sur le terminal POS

Les ports physiques du D3 Pro ne sont pas établis par le dépôt : À effectuer (EV-15 : liste des ports et de l'emplacement de carte d'un vrai D3 Pro). Usage prévu de ce que le terminal offre :

| Port ou interface | Utilisation |
|---|---|
| Réseau (Wi-Fi, ou RJ45 si le modèle en offre un) | Connexion LAN vers routeur (1) ; le Wi-Fi se prépare au banc avant l'import du lot ; l'application 1.0.0 n'a pas d'écran de réseau (`MANUEL_INSTALLATION_FR.md` §4.3) |
| USB (clé) | Disponible pour la clé du lot de provisionnement signé à l'installation et pour la clé de mise à jour déclarée (retirées en service normal) |
| Emplacement de carte SD ou clé USB | Cible de l'export vers le SPF (Art. 16 et 102), seulement par une action du Gérant (`PLATEFORME_ANDROID_FR.md` §5) ; jamais support de la base de données |
| Transfert de fichiers USB vers un ordinateur | Interdit (`DISALLOW_USB_FILE_TRANSFER`, cf. `PLATEFORME_ANDROID_FR.md` §3.3) |
| Débogage USB (adb) | Coupé sur un terminal provisionné en version de production |

### Sur le routeur

| Port physique | Utilisation |
|---|---|
| WAN RJ45 | Connexion Internet (FAI du restaurant) |
| LAN RJ45 #1 | Libre, ou terminal POS (1) si une liaison filaire est utilisée |
| LAN RJ45 #2 | Vers FDM (3) |
| LAN RJ45 #3 | Vers Imprimante (4) |
| LAN RJ45 #4 | Libre (extension futur) |

### Sur l'imprimante

| Port physique | Utilisation |
|---|---|
| RJ45 | Connexion LAN vers routeur |
| USB-B | (inutilisé) |
| RJ11 (DK port) | Vers tiroir-caisse (5) |
| Alimentation | Bloc 24V dédié |

---

## 4. Configuration réseau

### Plan d'adressage IP

| Appareil | Hostname | IP | MAC (à renseigner à l'install) | Service |
|---|---|---|---|---|
| Routeur | `gw` | `192.168.1.1` | - | DHCP, gateway, DNS |
| POS | - | `192.168.1.10` | (à renseigner ; comportement de l'adresse MAC Wi-Fi : EV-15) | pili-pos-android (application Android, kiosque) |
| FDM | `fdm` | `192.168.1.20` | (à renseigner) | GraphQL du FDM module2 (HTTPS, TCP 443) |
| Imprimante | `printer` | `192.168.1.30` | (à renseigner) | ESC/POS raw TCP 9100 |

### Réservations DHCP

Configurées sur le routeur : 3 baux statiques pour les MAC du POS, du FDM et de l'imprimante, mappés sur leur IP respective. Aucune autre machine n'a accès au LAN du SCE en production (le SCE est isolé du réseau invité du restaurant). Le terminal n'ouvre aucun port en écoute : la réservation de son adresse sert à la maintenance et au plan d'adressage, pas à un service.

### Pare-feu

Sur le routeur : politique par défaut bloquante en LAN <-> LAN. Règles autorisées :
- POS (.10) → FDM (.20) port TCP 443 (GraphQL en HTTPS/mTLS ; 80 si HTTP)
- POS (.10) → Imprimante (.30) port TCP 9100 (ESC/POS)
- FDM (.20) → Internet (WAN) sur les domaines/IP SPF Finances (gérés par le FDM lui-même via mTLS)
- POS (.10) → Internet (WAN) en HTTPS sortant uniquement : - la mise à jour déclarée, tirée de façon anonyme du registre public des artefacts déclarés, seulement quand un Gérant la lance depuis la caisse (jamais en arrière-plan), - uniquement si `pos.json::digital_ticket` est configuré (§III.H) - l'envoi du ticket numérique par e-mail vers `api.scaleway.com` (port 443, cf. `TICKET_NUMERIQUE_FR.md` §9), et - uniquement si HubRise est activé sur l'installation - `manager.hubrise.com` et `api.hubrise.com` (port 443, cf. `COMMANDES_EN_LIGNE_FR.md`), et - uniquement si la section `export` de `pos.json` est renseignée - l'envoi de la sauvegarde chiffrée et de la copie statistique vers le point de réception propre à l'installation (HTTPS, port 443 ou le port indiqué dans `url`, cf. `EXPORT_NOCTURNE_FR.md`). Sans ces configurations, aucun trafic sortant n'est initié par le POS en dehors du FDM. Le terminal avec services Google ne doit être relié à aucun compte Google (`PLATEFORME_ANDROID_FR.md` §3.1) ; les flux propres aux services Google du système, tant que ses applications ne sont pas masquées, sont À effectuer (EV-13 : trafic sortant du système sur le profil avec services Google).

Aucun port n'est ouvert depuis Internet vers le LAN (zone DMZ vide). Aucun port d'administration distante permanent (SSH, RDP, VNC) n'est ouvert sur le POS et aucun outil d'assistance à distance n'est installé : le terminal n'a aucun port en écoute et adb est coupé en version de production (cf. `PLATEFORME_ANDROID_FR.md` §6 et §7).

---

## 5. Procédure d'installation physique

Étapes, dans cet ordre :

1. Déballer et installer le routeur. Le brancher au modem Internet du restaurant. Vérifier l'accès Internet.
2. Configurer le routeur (login admin du routeur, IP statique gateway 192.168.1.1, DHCP 100-200, réservations DHCP pour les 3 appareils SCE).
3. Brancher le FDM au routeur via RJ45 (port LAN 2). Mettre sous tension. Vérifier que le FDM obtient son IP réservée 192.168.1.20.
4. Brancher l'imprimante au routeur via RJ45 (port LAN 3). Brancher l'alimentation. Vérifier IP 192.168.1.30. Brancher le RJ11 du tiroir-caisse au DK port de l'imprimante.
5. Poser le terminal D3 Pro, brancher son alimentation et le relier au routeur (Wi-Fi, ou liaison filaire si le modèle en offre une). Le terminal est provisionné au banc du producteur avant livraison : propriétaire de l'appareil attribué sans compte Google, application installée et verrouillée (`PLATEFORME_ANDROID_FR.md` §3.1). Sur le terrain, un terminal réinitialisé se provisionne par code QR.
6. Au démarrage, l'application, écran d'accueil du terminal, se lance en plein écran en mode kiosque après quelques secondes.
7. Configuration du POS : elle ne se fait pas à l'écran de la caisse. Le technicien prépare le lot de provisionnement signé sur clé USB (`pos.json`, fichiers du FDM, jeton, certificats), le branche sur le terminal et l'importe depuis l'écran de provisionnement, comme décrit dans `MANUEL_INSTALLATION_FR.md` (import du lot, premiers comptes). L'application vérifie la signature du lot avant tout usage ; sans lot valide, le moteur ne démarre pas. Pour le FDM module2 : `fdm_url` `https://192.168.1.20/graphql/`, `fdm_token_file`, et `fdm_ca_file`, `fdm_client_cert_file`, `fdm_client_key_file` (`ca.crt`, `client.crt`, `client.key`, publiés sur manual.module2.be) fournis dans le lot. `printer_address` vaut `192.168.1.30:9100`.
8. Première connexion d'un Gérant sur l'écran de la caisse (`MANUEL_UTILISATEUR_FR.md` §2.1) : le POS signe un `signWorkIn` (événement S), ce qui valide la chaîne complète (FDM, identité, horloge). Si le FDM l'accepte, le POS est opérationnel.

---

## 6. Procédure de réassemblage (pour audit futur)

En cas d'audit SPF Finances, le SCE peut être réassemblé à l'identique à partir du présent schéma et du manuel d'installation `MANUEL_INSTALLATION_FR.md`. Les éléments à conserver pour ré-installation :

- L'APK signé de la version déclarée (artefact public `declared-android`, sommes de contrôle dans `GARANTIE_MAJ_LANGUES_FR.md` §1.2) et la procédure de provisionnement du terminal (`PLATEFORME_ANDROID_FR.md` §3.1).
- Le lot de provisionnement signé, dont le fichier `pos.json` de configuration (sauvegardé séparément).
- La base SQLite `pos.db` : les 7 sauvegardes les plus récentes, écrites après chaque rapport Z dans le stockage privé de l'application, sont reprises par l'export vers le SPF (`SECURITE_BD_FR.md` §8).
- Le présent schéma + les n° de série des composants, dont le numéro de série du terminal, qui est son deviceId (`PLATEFORME_ANDROID_FR.md` §4), à consigner au formulaire d'Intake. Si le numéro de série du terminal diffère de l'enregistrement de provisionnement, le moteur reste fermé jusqu'à un nouvel import signé du lot (`PLATEFORME_ANDROID_FR.md` §4).

---

## 7. Variante pour la machine de test à l'Intake

Pour la machine de test livrée au rendez-vous d'admission SPF Finances :

- Terminal : profil émulé du D3 Pro avec services Google, sauf si un vrai D3 Pro est disponible au dépôt (À effectuer (EV-15 : terminal réel, ou déclaration du profil émulé)). Numéro de série de la machine de test : À effectuer (EV-16 : deviceId de la machine de test).
- Routeur fourni par le distributeur (mini-routeur de prêt).
- **Pas de FDM physique : le simulateur en ligne module2 est fourni avec la machine de test.** Le manuel de certification (§IV « Procédure de communication GraphQL », procédure de test, étape 1) demande d'appliquer les scénarios « à l'aide d'un simulateur FDM » et de fournir « un simulateur avec la machine test » ; le guide d'introduction (section « Intake », « Machine de test ») précise : « Si un simulateur de FDM est utilisé, sa méthode de connexion et ses ports doivent être clairement documentés. » Connexion du simulateur :

  | Élément | Valeur |
  |---|---|
  | Fournisseur | module2 (PRACSYS), FDM certifié SCE 2.0 |
  | Simulateur | `FDM02000425` (logiciel FDM 2.2.0.5), rattaché au compte développeur du producteur |
  | URL GraphQL | `https://fdm.module2.be/graphql/` (TCP 443, HTTPS avec le certificat client module2), configurée sur la machine de test ; `http://fdm.module2.be/graphql/` (TCP 80) reste possible |
  | Certificats | `ca.crt`, `client.crt`, `client.key` de module2 (`fdm_ca_file`, `fdm_client_cert_file`, `fdm_client_key_file`). Le POS n'accepte que le certificat serveur signé par l'autorité de module2 (ce certificat ne porte aucun nom d'hôte) ; tout autre est refusé avec le message « Connexion sécurisée à la boîte noire refusée (certificat) » |
  | Authentification | en-tête `Authorization: Bearer <jeton du simulateur>`, fichier `fdm_token_file` fourni dans le lot de provisionnement de la machine de test |
  | En-tête facultatif | `Simulator-Version` (sans lui, la dernière version du simulateur) |
  | Identité de test | `posId` `CPOS0031234567`, terminal `POS-2-ANDROID`, TVA `BE0000000097`, unité d'établissement `2000000042` (UC_TestSetup_20240806) |
  | Réseau requis | accès Internet sortant de la machine de test (Wi-Fi ou UTP du laboratoire, via le routeur de prêt) |

  Les erreurs et avertissements du FDM (buffer plein, FDM verrouillé, buffer presque plein, nombre de tentatives) se déclenchent depuis l'onglet « Developer » du portail module2, et les requêtes reçues par le simulateur s'y consultent (onglet « Logs »). Les essais en direct contre le simulateur se font sur accord du producteur : À effectuer (EV-14 : essai en direct contre le simulateur module2).
- Tiroir-caisse RJ11 fourni avec l'imprimante.
- Personnel du jeu de test SPF (UC_TestSetup_20240806) fourni par la graine de test du lot de provisionnement. Le terminal n'a qu'un seul utilisateur système (pas de compte `spf` de console) : l'agent SPF accède aux données sur place par les écrans du Gérant protégés par PIN et par l'export vers clé USB ou carte SD (`SECURITE_BD_FR.md` §8).

Le reste du schéma est identique à la production, à ceci près que le FDM est joint par Internet et non sur le LAN.
